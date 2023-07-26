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
use widgets::tree::TreeState;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
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

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    file_info: FileInfo,
) -> Result<(), anyhow::Error> {
    let screen = Screen::default();
    let file_name = FileName::new(file_info.name.clone());
    let file_size = FileSize::new(file_info.size);
    let contents_tree = ContentsTree::new(file_info.to_tree_items());
    let mut contents_tree_state = TreeState::default();
    loop {
        let selected_entity = file_info
            .entity(
                contents_tree_state
                    .position(&file_info.to_tree_items())
                    .unwrap(),
            )
            .context("Could not find selected entity")?;
        terminal.draw(|frame| {
            screen.render(
                frame,
                file_name.clone(),
                file_size.clone(),
                contents_tree.clone(),
                &mut contents_tree_state,
                selected_entity,
            )
        })?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Up | KeyCode::Char('k') => contents_tree_state.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        contents_tree_state.move_down(&file_info.to_tree_items())
                    }
                    KeyCode::PageUp => contents_tree_state.page_up(),
                    KeyCode::PageDown => contents_tree_state.page_down(&file_info.to_tree_items()),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
