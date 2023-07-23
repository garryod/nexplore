use crate::widgets::tree::{TreeItem, TreeItems};
use anyhow::{anyhow, Context};
use hdf5::{Dataset, File, Group};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum EntityInfo {
    Group(GroupInfo),
    Dataset(DatasetInfo),
}

impl From<EntityInfo> for TreeItem<'_> {
    fn from(value: EntityInfo) -> Self {
        match value {
            EntityInfo::Group(info) => TreeItem::from(info),
            EntityInfo::Dataset(info) => TreeItem::from(info),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub name: String,
    pub entities: Vec<EntityInfo>,
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

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    pub name: String,
}

impl From<Dataset> for DatasetInfo {
    fn from(dataset: Dataset) -> Self {
        let name = dataset.name().split('/').last().unwrap().to_string();
        Self { name }
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
