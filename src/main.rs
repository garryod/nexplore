mod h5file;
mod ui;

use crate::ui::render;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use h5file::FileInfo;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io::Stdout, path::PathBuf, time::Duration};

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
    loop {
        terminal.draw(|frame| render(frame, &file_info))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }
    Ok(())
}
