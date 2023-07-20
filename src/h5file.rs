use crate::widgets::tree::TreeItem;
use anyhow::Context;
use hdf5::{Dataset, File, Group};
use ratatui::{style::Color, text::Text};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GroupInfo {
    name: String,
    subgroups: Vec<GroupInfo>,
    datasets: Vec<DatasetInfo>,
}

impl GroupInfo {
    fn extract(group: Group) -> Result<Self, anyhow::Error> {
        let name = group.name().split('/').last().unwrap().to_string();
        let subgroups = group
            .groups()?
            .into_iter()
            .map(GroupInfo::extract)
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        let datasets = group
            .datasets()?
            .into_iter()
            .map(DatasetInfo::extract)
            .collect();
        Ok(Self {
            name,
            subgroups,
            datasets,
        })
    }
}

impl From<GroupInfo> for TreeItem<'_> {
    fn from(group: GroupInfo) -> Self {
        Self::new(
            Text::raw(group.name),
            Color::Green,
            group
                .subgroups
                .into_iter()
                .map(TreeItem::from)
                .chain(group.datasets.into_iter().map(TreeItem::from))
                .collect(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    name: String,
}

impl DatasetInfo {
    fn extract(dataset: Dataset) -> Self {
        let name = dataset.name().split('/').last().unwrap().to_string();
        Self { name }
    }
}

impl From<DatasetInfo> for TreeItem<'_> {
    fn from(dataset: DatasetInfo) -> Self {
        Self::new(Text::raw(dataset.name), Color::Blue, vec![])
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub groups: Vec<GroupInfo>,
    pub datasets: Vec<DatasetInfo>,
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
        let groups = file
            .groups()?
            .into_iter()
            .map(GroupInfo::extract)
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        let datasets = file
            .datasets()?
            .into_iter()
            .map(DatasetInfo::extract)
            .collect();

        Ok(Self {
            name,
            size,
            groups,
            datasets,
        })
    }

    pub fn to_tree_items(&self) -> Vec<TreeItem> {
        self.datasets
            .iter()
            .cloned()
            .map(TreeItem::from)
            .chain(self.groups.iter().cloned().map(TreeItem::from))
            .collect()
    }
}
