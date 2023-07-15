use humansize::{format_size, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::io::Stdout;

use crate::h5file::FileInfo;

pub fn render(frame: &mut Frame<'_, CrosstermBackend<Stdout>>, file_info: &FileInfo) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Ratio(1, 1)])
        .split(frame.size());
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(4, 5), Constraint::Ratio(1, 5)])
        .split(vertical_chunks[0]);
    let file_name = Paragraph::new(file_info.name.clone())
        .block(Block::default().title("File Name").borders(Borders::ALL));
    frame.render_widget(file_name, header_chunks[0]);
    let file_size = Paragraph::new(format_size(file_info.size, BINARY))
        .block(Block::default().title("Size").borders(Borders::ALL));
    frame.render_widget(file_size, header_chunks[1]);
}
