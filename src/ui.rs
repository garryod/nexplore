use crate::{
    h5file::{DatasetInfo, DatasetLayoutInfo, EntityInfo, GroupInfo},
    widgets::tree::{Tree, TreeItem, TreeState},
};
use humansize::{format_size, ToF64, Unsigned, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
    Frame,
};
use std::io::Stdout;

#[derive(Debug)]
pub struct Screen {
    frame_layout: Layout,
    header_layout: Layout,
    data_layout: Layout,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            frame_layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Ratio(1, 1)]),
            header_layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(4, 5), Constraint::Ratio(1, 5)]),
            data_layout: Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)]),
        }
    }
}

impl Screen {
    pub fn render(
        &self,
        frame: &mut Frame<'_, CrosstermBackend<Stdout>>,
        file_name: &FileName,
        file_size: &FileSize,
        contents_tree: &mut ContentsTree,
        entity_info: impl Widget,
    ) {
        let vertical_chunks = self.frame_layout.split(frame.size());
        let header_chunks = self.header_layout.split(vertical_chunks[0]);
        frame.render_widget(file_name.0.clone(), header_chunks[0]);
        frame.render_widget(file_size.0.clone(), header_chunks[1]);
        let data_chunks = self.data_layout.split(vertical_chunks[1]);
        frame.render_stateful_widget(
            contents_tree.widget.clone(),
            data_chunks[0],
            &mut contents_tree.state,
        );
        frame.render_widget(entity_info, data_chunks[1]);
    }
}

#[derive(Debug, Clone)]
pub struct FileName<'a>(Paragraph<'a>);

impl<'a> FileName<'a> {
    pub fn new(file_name: impl AsRef<str>) -> Self {
        Self(
            Paragraph::new(file_name.as_ref().to_string())
                .block(Block::default().title("File").borders(Borders::ALL)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileSize<'a>(Paragraph<'a>);

impl<'a> FileSize<'a> {
    pub fn new(file_size: impl ToF64 + Unsigned) -> Self {
        Self(
            Paragraph::new(format_size(file_size, BINARY))
                .block(Block::default().title("Size").borders(Borders::ALL)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ContentsTree<'a> {
    pub widget: Tree<'a>,
    pub state: TreeState<'a>,
}

impl<'a> ContentsTree<'a> {
    pub fn new(items: Vec<TreeItem<'a>>) -> Self {
        Self {
            widget: Tree::default().block(Block::default().title("Contents").borders(Borders::ALL)),
            state: TreeState::new(items),
        }
    }
}

impl Widget for EntityInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            EntityInfo::Group(group) => group.render(area, buf),
            EntityInfo::Dataset(dataset) => dataset.render(area, buf),
        }
    }
}

const GROUP_COLOR: Color = Color::Blue;

impl Widget for GroupInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Table::new(vec![
            Row::new(vec![Cell::from("ID"), Cell::from(self.id.to_string())]),
            Row::new(vec![
                Cell::from("Link Type"),
                Cell::from(self.link_kind.to_string()),
            ]),
        ])
        .widths(&[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .block(
            Block::default()
                .title(self.name.clone())
                .border_style(Style::new().fg(GROUP_COLOR))
                .borders(Borders::ALL),
        )
        .render(area, buf);
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

impl Widget for DatasetInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut rows = vec![
            Row::new(vec![Cell::from("ID"), Cell::from(self.id.to_string())]),
            Row::new(vec![
                Cell::from("Link Type"),
                Cell::from(self.link_type.to_string()),
            ]),
            Row::new(vec![
                Cell::from("Shape"),
                Cell::from(format!("{:?}", self.shape)),
            ]),
            Row::new(vec![
                Cell::from("Layout"),
                Cell::from(match self.layout_info {
                    DatasetLayoutInfo::Compact {} => "Compact",
                    DatasetLayoutInfo::Contiguous {} => "Contiguous",
                    DatasetLayoutInfo::Chunked {
                        chunk_shape: _,
                        filters: _,
                    } => "Chunked",
                    DatasetLayoutInfo::Virtial {} => "Virtual",
                }),
            ]),
        ];

        match self.layout_info.clone() {
            DatasetLayoutInfo::Compact {} => {}
            DatasetLayoutInfo::Contiguous {} => {}
            DatasetLayoutInfo::Chunked {
                chunk_shape,
                filters,
            } => {
                rows.append(&mut vec![
                    Row::new(vec![
                        Cell::from("Chunk Shape"),
                        Cell::from(format!("{chunk_shape:?}")),
                    ]),
                    Row::new(vec![
                        Cell::from("Filters"),
                        Cell::from(format!("{filters:?}")),
                    ]),
                ]);
            }
            DatasetLayoutInfo::Virtial {} => {}
        }

        Table::new(rows)
            .widths(&[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .block(
                Block::default()
                    .title(self.name.clone())
                    .border_style(Style::new().fg(DATASET_COLOR))
                    .borders(Borders::ALL),
            )
            .render(area, buf);
    }
}

impl From<DatasetInfo> for TreeItem<'_> {
    fn from(dataset: DatasetInfo) -> Self {
        Self::new(Text::raw(dataset.name), DATASET_COLOR, vec![])
    }
}
