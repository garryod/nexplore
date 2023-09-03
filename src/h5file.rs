use crate::widgets::tree::TreeItem;
use anyhow::{anyhow, Context};
use hdf5::{dataset::Layout, filters::Filter, Dataset, File, Group};
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
    pub id: i64,
    pub entities: Vec<EntityInfo>,
}

impl TryFrom<Group> for GroupInfo {
    type Error = anyhow::Error;

    fn try_from(group: Group) -> Result<Self, Self::Error> {
        let name = group.name().split('/').last().unwrap().to_string();
        let id = group.id();
        let entities = group
            .iter_visit_default(Vec::new(), |group, key, _, entities| {
                let entity = if let Ok(group) = group.group(key) {
                    GroupInfo::try_from(group).map(EntityInfo::Group)
                } else if let Ok(dataset) = group.dataset(key) {
                    Ok(EntityInfo::Dataset(DatasetInfo::from(dataset)))
                } else {
                    Err(anyhow!("Found link to entity of unknown kind"))
                };
                entities.push(entity);
                true
            })?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { name, id, entities })
    }
}

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    pub name: String,
    pub id: i64,
    pub shape: Vec<usize>,
    pub layout_info: DatasetLayoutInfo,
}

#[derive(Debug, Clone)]
pub enum DatasetLayoutInfo {
    Compact {},
    Contiguous {},
    Chunked {
        chunk_shape: Vec<usize>,
        filters: Vec<Filter>,
    },
    Virtial {},
}

impl From<Dataset> for DatasetInfo {
    fn from(dataset: Dataset) -> Self {
        let name = dataset.name().split('/').last().unwrap().to_string();
        let id = dataset.id();
        let shape = dataset.shape();
        let layout_info = match dataset.layout() {
            Layout::Compact => DatasetLayoutInfo::Compact {},
            Layout::Contiguous => DatasetLayoutInfo::Contiguous {},
            Layout::Chunked => DatasetLayoutInfo::Chunked {
                chunk_shape: dataset.chunk().unwrap(),
                filters: dataset.filters(),
            },
            Layout::Virtual => DatasetLayoutInfo::Virtial {},
        };
        Self {
            name,
            id,
            shape,
            layout_info,
        }
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

    pub fn to_tree_items(&self) -> Vec<TreeItem<'_>> {
        self.entities
            .iter()
            .cloned()
            .map(TreeItem::from)
            .collect::<Vec<_>>()
    }
}
