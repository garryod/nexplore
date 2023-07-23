use crate::widgets::tree::{TreeItem, TreeItems};
use anyhow::{anyhow, Context};
use hdf5::{Dataset, File, Group};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::Path;

pub trait Render<'f, B: Backend> {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect);
}

#[derive(Debug, Clone)]
pub enum EntityInfo {
    Group(GroupInfo),
    Dataset(DatasetInfo),
}

impl<'f, B: Backend> Render<'f, B> for EntityInfo {
    fn render(&self, frame: &mut Frame<'f, B>, area: Rect) {
        match self {
            EntityInfo::Group(group) => group.render(frame, area),
            EntityInfo::Dataset(dataset) => dataset.render(frame, area),
        }
    }
}

impl From<EntityInfo> for TreeItem<'_> {
    fn from(value: EntityInfo) -> Self {
        match value {
            EntityInfo::Group(info) => TreeItem::from(info),
            EntityInfo::Dataset(info) => TreeItem::from(info),
        }
    }
}

const GROUP_COLOR: Color = Color::Green;

#[derive(Debug, Clone)]
pub struct GroupInfo {
    name: String,
    entities: Vec<EntityInfo>,
}

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

impl TryFrom<Group> for GroupInfo {
    type Error = anyhow::Error;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        let name = group.name().split('/').last().unwrap().to_string();
        let mut entities = group
            .groups()?
            .into_iter()
            .map(GroupInfo::try_from)
            .map(|group| group.map(EntityInfo::Group))
            .collect::<Result<Vec<_>, _>>()?;
        entities.extend(
            group
                .datasets()?
                .into_iter()
                .map(DatasetInfo::from)
                .map(EntityInfo::Dataset),
        );
        Ok(Self { name, entities })
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

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    name: String,
}

const DATASET_COLOR: Color = Color::Blue;

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

impl From<Dataset> for DatasetInfo {
    fn from(dataset: Dataset) -> Self {
        let name = dataset.name().split('/').last().unwrap().to_string();
        Self { name }
    }
}

impl From<DatasetInfo> for TreeItem<'_> {
    fn from(dataset: DatasetInfo) -> Self {
        Self::new(Text::raw(dataset.name), DATASET_COLOR, vec![])
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub entities: Vec<EntityInfo>,
}

impl FileInfo {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let name = path
            .as_ref()
            .file_name()
            .context("No file in path")?
            .to_string_lossy()
            .into_owned();
        let file = File::open(path)?;
        let size = file.size();
        let entities = GroupInfo::try_from(file.as_group()?)?.entities;

        Ok(Self {
            name,
            size,
            entities,
        })
    }

    pub fn entity(&self, index: Vec<usize>) -> Result<EntityInfo, anyhow::Error> {
        let mut indices = index.into_iter();
        let mut entity = self
            .entities
            .get(indices.next().context("Index was empty")?)
            .context("No entity at index")?;
        for idx in indices {
            match entity {
                EntityInfo::Group(group) => {
                    entity = group.entities.get(idx).context("Index was empty")?
                }
                EntityInfo::Dataset(_) => Err(anyhow!("Cannot index into a dataset"))?,
            }
        }
        Ok(entity.clone())
    }

    pub fn to_tree_items(&self) -> TreeItems {
        TreeItems::from(
            self.entities
                .iter()
                .cloned()
                .map(TreeItem::from)
                .collect::<Vec<_>>(),
        )
    }
}
