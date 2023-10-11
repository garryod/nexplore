mod h5file;
mod ui;
pub mod widgets;

use crate::ui::Screen;
use anyhow::Context;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use h5file::FileInfo;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::Stdout, path::PathBuf, time::Duration};
use ui::{ContentsTree, FileName, FileSize};

/// A TUI for exploring HDF5 and NeXus files.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The path to the HDF5 or NeXus file to open.
    path: PathBuf,
}

fn main() {
    let args = Cli::parse();
    let mut terminal = setup_terminal().unwrap();
    let file_info = FileInfo::read(args.path).unwrap();
    run(&mut terminal, file_info).unwrap();
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
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    file_info: FileInfo,
) -> Result<(), anyhow::Error> {
    let mut mode = Mode::default();
    let screen = Screen::default();
    let file_name = FileName::new(file_info.name.clone());
    let file_size = FileSize::new(file_info.size);
    let mut contents_tree = ContentsTree::new(file_info.to_tree_items());
    loop {
        let entity_info = file_info
            .entity(contents_tree.state.position().unwrap())
            .context("Could not find selected entity")?;
        terminal.draw(|frame| {
            screen.render(
                frame,
                &file_name,
                &file_size,
                &mut contents_tree,
                entity_info,
            )
        })?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match (&mut mode, key.code) {
                    (&mut Mode::Normal, KeyCode::Esc | KeyCode::Char('q')) => break,
                    (&mut Mode::Normal, KeyCode::Up | KeyCode::Char('k')) => {
                        contents_tree.state.move_up()
                    }
                    (&mut Mode::Normal, KeyCode::Down | KeyCode::Char('j')) => {
                        contents_tree.state.move_down()
                    }
                    (&mut Mode::Normal, KeyCode::PageUp) => contents_tree.state.page_up(),
                    (&mut Mode::Normal, KeyCode::PageDown) => contents_tree.state.page_down(),
                    (&mut Mode::Normal, KeyCode::Left | KeyCode::Char('h')) => {
                        contents_tree.state.collapse()
                    }
                    (&mut Mode::Normal, KeyCode::Right | KeyCode::Char('l')) => {
                        contents_tree.state.expand()
                    }
                    (mode, KeyCode::Char('/')) if matches!(mode, Mode::Normal) => {
                        *mode = Mode::Search {
                            search: String::default(),
                        };
                        contents_tree.state.search(Some(String::default()));
                    }
                    (&mut Mode::Search { search: _ }, KeyCode::Esc) => {
                        mode = Mode::default();
                        contents_tree.state.search(None);
                    }
                    (&mut Mode::Search { ref mut search }, KeyCode::Char(char)) => {
                        search.push(char);
                        contents_tree.state.search(Some(search.clone()));
                    }
                    (&mut Mode::Search { ref mut search }, KeyCode::Backspace) => {
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
