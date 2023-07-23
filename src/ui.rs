use crate::{
    h5file::{DatasetInfo, EntityInfo, GroupInfo},
    widgets::tree::{Tree, TreeItem, TreeItems, TreeState},
};
use humansize::{format_size, BINARY};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
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

pub trait Render<'f, B: Backend> {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect);
}

impl<'f, B: Backend> Render<'f, B> for EntityInfo {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect) {
        match self {
            EntityInfo::Group(group) => group.render(frame, area),
            EntityInfo::Dataset(dataset) => dataset.render(frame, area),
        }
    }
}

const GROUP_COLOR: Color = Color::Blue;

impl<'f, B: Backend> Render<'f, B> for GroupInfo {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect) {
        let widget = Paragraph::new("").block(
            Block::default()
                .title(self.name.clone())
                .border_style(Style::new().fg(GROUP_COLOR))
                .borders(Borders::ALL),
        );
        frame.render_widget(widget, area);
    }
}

impl From<GroupInfo> for TreeItem<'_> {
    fn from(group: GroupInfo) -> Self {
        Self::new(
            Text::raw(group.name),
            GROUP_COLOR,
            group.entities.into_iter().map(TreeItem::from).collect(),
        )
    }
}

const DATASET_COLOR: Color = Color::Green;

impl<'f, B: Backend> Render<'f, B> for DatasetInfo {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect) {
        let widget = Paragraph::new("").block(
            Block::default()
                .title(self.name.clone())
                .border_style(Style::new().fg(DATASET_COLOR))
                .borders(Borders::ALL),
        );
        frame.render_widget(widget, area);
    }
}

impl From<DatasetInfo> for TreeItem<'_> {
    fn from(dataset: DatasetInfo) -> Self {
        Self::new(Text::raw(dataset.name), DATASET_COLOR, vec![])
    }
}
