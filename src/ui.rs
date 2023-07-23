use crate::{
    h5file::{EntityInfo, Render},
    widgets::tree::{Tree, TreeItems, TreeState},
};
use humansize::{format_size, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::io::Stdout;

pub fn render(
    frame: &mut Frame<'_, CrosstermBackend<Stdout>>,
    tree_state: &mut TreeState,
    tree_items: TreeItems,
    file_name: String,
    file_size: u64,
    selected_entity: EntityInfo,
) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Ratio(1, 1)])
        .split(frame.size());
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(4, 5), Constraint::Ratio(1, 5)])
        .split(vertical_chunks[0]);
    let file_name = Paragraph::new(file_name.clone())
        .block(Block::default().title("File Name").borders(Borders::ALL));
    frame.render_widget(file_name, header_chunks[0]);
    let file_size = Paragraph::new(format_size(file_size, BINARY))
        .block(Block::default().title("Size").borders(Borders::ALL));
    frame.render_widget(file_size, header_chunks[1]);
    let data_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)])
        .split(vertical_chunks[1]);
    let contents_tree =
        Tree::new(tree_items).block(Block::default().title("Contents").borders(Borders::ALL));
    frame.render_stateful_widget(contents_tree, data_chunks[0], tree_state);
    selected_entity.render(frame, data_chunks[1]);
}
