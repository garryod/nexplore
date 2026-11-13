use crate::{
    h5file::{DatasetInfo, DatasetLayoutInfo, EntityInfo, GroupInfo},
    widgets::plot::Plot,
    widgets::tree::{Tree, TreeItem, TreeState},
};
use humansize::{BINARY, ToF64, Unsigned, format_size};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Widget},
};

const INFO_LABEL_MIN_WIDTH: u16 = 20;

#[derive(Debug)]
pub struct Screen {
    frame_layout: Layout,
    header_layout: Layout,
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
        }
    }
}

impl Screen {
    pub fn render(
        &self,
        frame: &mut Frame,
        file_name: &FileName,
        file_size: &FileSize,
        contents_tree: &mut ContentsTree,
        entity_info: impl Widget,
    ) {
        let vertical_chunks = self.frame_layout.split(frame.area());
        let header_chunks = self.header_layout.split(vertical_chunks[0]);
        frame.render_widget(file_name.0.clone(), header_chunks[0]);
        frame.render_widget(file_size.0.clone(), header_chunks[1]);
        let data_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(contents_tree.state.width()),
                Constraint::Min(0),
            ])
            .split(vertical_chunks[1]);
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

impl FileName<'_> {
    pub fn new(file_name: impl AsRef<str>) -> Self {
        Self(
            Paragraph::new(file_name.as_ref().to_string())
                .block(Block::default().title("File").borders(Borders::ALL)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileSize<'a>(Paragraph<'a>);

impl FileSize<'_> {
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
const NXCLASS: &str = "NX_class";

impl Widget for GroupInfo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut info_rows = vec![
            Row::new(vec![Cell::from("ID"), Cell::from(self.id.to_string())]),
            Row::new(vec![
                Cell::from("Link Type"),
                Cell::from(self.link_kind.to_string()),
            ]),
        ];
        if self.attrs.contains_key(NXCLASS) {
            info_rows.insert(
                0,
                Row::new(vec![
                    Cell::from(NXCLASS),
                    Cell::from(Text::from(self.attrs.get(NXCLASS).unwrap().to_string())),
                ]),
            );
        }
        let mut attr_rows = Vec::new();
        for (key, value) in &self.attrs {
            if key != NXCLASS {
                attr_rows.push(Row::new(vec![
                    Cell::from(key.to_string()),
                    Cell::from(value.to_string()),
                ]));
            }
        }

        render_info_panel(
            area,
            buf,
            self.name,
            GROUP_COLOR,
            max_label_width(&self.attrs, false),
            InfoPanelContent {
                info_rows,
                attr_rows,
                plot: None,
            },
        );
    }
}

impl From<GroupInfo> for TreeItem<'static> {
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
        let left_column_width = max_dataset_label_width(&self.attrs);
        let plot_data = self.plot_data.clone();
        let plot_title = self.name.clone();
        let info_rows = vec![
            Row::new(vec![Cell::from("Data Value"), Cell::from(self.data_value)]),
            Row::new(vec![
                Cell::from("Data Type"),
                Cell::from(self.dtype_descr.to_string()),
            ]),
            Row::new(vec![
                Cell::from("Shape"),
                Cell::from(format!("{:?}", self.shape)),
            ]),
            Row::new(vec![Cell::from("ID"), Cell::from(self.id.to_string())]),
            Row::new(vec![
                Cell::from("Link Type"),
                Cell::from(self.link_type.to_string()),
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

        let mut info_rows = info_rows;
        match self.layout_info.clone() {
            DatasetLayoutInfo::Compact {} => {}
            DatasetLayoutInfo::Contiguous {} => {}
            DatasetLayoutInfo::Chunked {
                chunk_shape,
                filters,
            } => {
                info_rows.append(&mut vec![
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
        let mut attr_rows = Vec::new();
        for (key, value) in &self.attrs {
            attr_rows.push(Row::new(vec![
                Cell::from(key.to_string()),
                Cell::from(value.to_string()),
            ]));
        }

        render_info_panel(
            area,
            buf,
            self.name,
            DATASET_COLOR,
            left_column_width,
            InfoPanelContent {
                info_rows,
                attr_rows,
                plot: plot_data.map(|points| Plot::new(format!("Plot: {plot_title}"), points)),
            },
        );
    }
}

impl From<DatasetInfo> for TreeItem<'static> {
    fn from(dataset: DatasetInfo) -> Self {
        Self::new(Text::raw(dataset.name), DATASET_COLOR, vec![])
    }
}

fn max_label_width(
    attrs: &std::collections::HashMap<String, String>,
    includes_nxclass: bool,
) -> u16 {
    let mut width = ["ID", "Link Type"]
        .into_iter()
        .map(str::len)
        .max()
        .unwrap_or(0);
    if includes_nxclass {
        width = width.max(NXCLASS.len());
    }
    width = width.max(attrs.keys().map(|key| key.len()).max().unwrap_or(0));
    (width as u16).max(INFO_LABEL_MIN_WIDTH)
}

fn max_dataset_label_width(attrs: &std::collections::HashMap<String, String>) -> u16 {
    let width = [
        "Data Value",
        "Data Type",
        "Shape",
        "ID",
        "Link Type",
        "Layout",
        "Chunk Shape",
        "Filters",
    ]
    .into_iter()
    .map(str::len)
    .chain(attrs.keys().map(|key| key.len()))
    .max()
    .unwrap_or(0);
    (width as u16).max(INFO_LABEL_MIN_WIDTH)
}

struct InfoPanelContent<'a> {
    info_rows: Vec<Row<'a>>,
    attr_rows: Vec<Row<'a>>,
    plot: Option<Plot>,
}

fn render_info_panel(
    area: Rect,
    buf: &mut Buffer,
    title: String,
    color: Color,
    left_column_width: u16,
    content: InfoPanelContent<'_>,
) {
    let InfoPanelContent {
        info_rows,
        attr_rows,
        plot,
    } = content;
    let block = Block::default()
        .title(title)
        .border_style(Style::new().fg(color))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    Clear.render(area, buf);
    block.render(area, buf);

    if let Some(plot) = plot {
        let metadata_height = u16::try_from(
            info_rows
                .len()
                .saturating_add(attr_rows.len())
                .saturating_add(usize::from(!attr_rows.is_empty())),
        )
        .unwrap_or(u16::MAX);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(metadata_height), Constraint::Min(0)])
            .split(inner);
        render_metadata_panel(chunks[0], buf, left_column_width, info_rows, attr_rows);
        plot.render(chunks[1], buf);
        return;
    }

    render_metadata_panel(inner, buf, left_column_width, info_rows, attr_rows);
}

fn render_metadata_panel(
    area: Rect,
    buf: &mut Buffer,
    left_column_width: u16,
    info_rows: Vec<Row>,
    attr_rows: Vec<Row>,
) {
    if area.height == 0 {
        return;
    }

    let info_height = info_rows.len() as u16;
    let separator_height = u16::from(!attr_rows.is_empty());
    let attr_height = area.height.saturating_sub(info_height + separator_height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(info_height),
            Constraint::Length(separator_height),
            Constraint::Length(attr_height),
        ])
        .split(area);

    Table::new(
        info_rows,
        [Constraint::Length(left_column_width), Constraint::Min(0)],
    )
    .render(chunks[0], buf);

    if separator_height == 1 {
        let separator = "─".repeat(chunks[1].width as usize);
        Paragraph::new(Line::raw(separator)).render(chunks[1], buf);
        Table::new(
            attr_rows,
            [Constraint::Length(left_column_width), Constraint::Min(0)],
        )
        .render(chunks[2], buf);
    }
}
