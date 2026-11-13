mod h5file;
mod ui;
pub mod widgets;

use crate::{
    ui::Screen, widgets::help::render_help_dialog, widgets::plot::HORIZONTAL_PIXELS_PER_COLUMN,
};
use anyhow::Context;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use h5file::FileInfo;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io::Stdout, path::PathBuf, time::Duration};
use ui::{ContentsTree, FileName, FileSize};

const PLOT_SOURCE_SAMPLES_PER_PIXEL: usize = 4;

/// A TUI for exploring HDF5 and NeXus files.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The path to the HDF5 or NeXus file to open.
    path: PathBuf,
}

fn main() {
    // get the command line options
    let args = Cli::parse();
    // get a handle to the file
    let file_info = FileInfo::read(args.path).unwrap();
    // setup the basic curses terminal
    let mut terminal = setup_terminal().unwrap();
    // start the program
    run(&mut terminal, file_info).unwrap();
    // shutdown the program
    restore_terminal(&mut terminal).unwrap();
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, anyhow::Error> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), anyhow::Error> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(terminal.show_cursor()?)
}

#[derive(Debug, Clone, Default)]
enum Mode {
    #[default]
    Normal,
    Search {
        search: String,
    },
    Help,
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut file_info: FileInfo,
) -> Result<(), anyhow::Error> {
    let mut mode = Mode::default();
    let screen = Screen::default();
    let file_name = FileName::new(file_info.name.clone());
    let file_size = FileSize::new(file_info.size);
    let mut contents_tree = ContentsTree::new(file_info.to_tree_items());
    let mut plotted_entity = None;
    loop {
        let selected_entity = contents_tree
            .state
            .position()
            .context("No selected entity")?;
        if plotted_entity.as_ref() != Some(&selected_entity)
            && let Some(previous_entity) = plotted_entity.take()
        {
            file_info.unload_plot_data(previous_entity)?;
        }
        let entity_info = file_info
            .entity(selected_entity)
            .context("Could not find selected entity")?;
        let show_help = matches!(mode, Mode::Help);
        terminal.draw(|frame| {
            screen.render(
                frame,
                &file_name,
                &file_size,
                &mut contents_tree,
                entity_info,
            );
            if show_help {
                render_help_dialog(frame);
            }
        })?;
        if event::poll(Duration::from_millis(250))? {
            let input = event::read()?;
            if let Event::Resize(width, _) = &input
                && let Some(selected_entity) = plotted_entity.clone()
            {
                file_info.unload_plot_data(selected_entity.clone())?;
                if !file_info.load_plot_data(
                    selected_entity,
                    plot_minimum_samples(*width, contents_tree.state.width()),
                )? {
                    plotted_entity = None;
                }
            }
            if let Event::Key(key) = input {
                match (&mut mode, key.code, key.modifiers) {
                    (
                        &mut Mode::Help,
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?'),
                        _,
                    ) => mode = Mode::default(),
                    (&mut Mode::Help, _, _) => {}
                    (&mut Mode::Normal, KeyCode::Char('?'), _) => mode = Mode::Help,
                    (&mut Mode::Normal, KeyCode::Esc | KeyCode::Char('q'), KeyModifiers::NONE) => {
                        break;
                    }
                    (&mut Mode::Normal, KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE) => {
                        contents_tree.state.move_up()
                    }
                    (&mut Mode::Normal, KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE) => {
                        contents_tree.state.move_down()
                    }
                    (&mut Mode::Normal, KeyCode::PageUp, KeyModifiers::NONE) => {
                        contents_tree.state.page_up()
                    }
                    (&mut Mode::Normal, KeyCode::PageDown, KeyModifiers::NONE) => {
                        contents_tree.state.page_down()
                    }
                    (&mut Mode::Normal, KeyCode::Left | KeyCode::Char('h'), KeyModifiers::NONE) => {
                        contents_tree.state.collapse()
                    }
                    (
                        &mut Mode::Normal,
                        KeyCode::Right | KeyCode::Char('l'),
                        KeyModifiers::NONE,
                    ) => contents_tree.state.expand(),
                    (
                        &mut Mode::Normal,
                        KeyCode::Left | KeyCode::Char('H'),
                        KeyModifiers::SHIFT,
                    ) => contents_tree.state.collapse_all(),
                    (
                        &mut Mode::Normal,
                        KeyCode::Right | KeyCode::Char('L'),
                        KeyModifiers::SHIFT,
                    ) => contents_tree.state.expand_all(),
                    (&mut Mode::Normal, KeyCode::Char('p'), KeyModifiers::NONE) => {
                        let selected_entity = contents_tree
                            .state
                            .position()
                            .context("No selected entity")?;
                        if plotted_entity.as_ref() == Some(&selected_entity) {
                            file_info.unload_plot_data(selected_entity)?;
                            plotted_entity = None;
                        } else if file_info.load_plot_data(
                            selected_entity.clone(),
                            plot_minimum_samples(
                                terminal.size()?.width,
                                contents_tree.state.width(),
                            ),
                        )? {
                            plotted_entity = Some(selected_entity);
                        }
                    }
                    (mode, KeyCode::Char('/'), KeyModifiers::NONE)
                        if matches!(mode, Mode::Normal) =>
                    {
                        *mode = Mode::Search {
                            search: String::default(),
                        };
                        contents_tree.state.search(Some(String::default()));
                    }
                    (&mut Mode::Search { search: _ }, KeyCode::Esc, KeyModifiers::NONE) => {
                        mode = Mode::default();
                        contents_tree.state.search(None);
                    }
                    (
                        &mut Mode::Search { ref mut search },
                        KeyCode::Char(char),
                        KeyModifiers::NONE,
                    ) => {
                        search.push(char);
                        contents_tree.state.search(Some(search.clone()));
                    }
                    (
                        &mut Mode::Search { ref mut search },
                        KeyCode::Backspace,
                        KeyModifiers::NONE,
                    ) => {
                        search.pop();
                        contents_tree.state.search(Some(search.clone()));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn plot_minimum_samples(terminal_width: u16, contents_width: u16) -> usize {
    plot_horizontal_pixels(terminal_width, contents_width)
        .saturating_mul(PLOT_SOURCE_SAMPLES_PER_PIXEL)
}

fn plot_horizontal_pixels(terminal_width: u16, contents_width: u16) -> usize {
    usize::from(
        terminal_width
            .saturating_sub(contents_width)
            .saturating_sub(2),
    )
    .saturating_mul(HORIZONTAL_PIXELS_PER_COLUMN)
}
