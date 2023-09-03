use crate::widgets::tree::TreeItem;
use anyhow::{anyhow, Context};
use hdf5::{dataset::Layout, filters::Filter, Dataset, File, Group, LinkInfo, LinkType};
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
    pub link_kind: LinkKind,
    pub entities: Vec<EntityInfo>,
}

impl GroupInfo {
    fn try_from_group_and_link(group: Group, link: LinkInfo) -> Result<Self, anyhow::Error> {
        let name = group.name().split('/').last().unwrap().to_string();
        let id = group.id();
        let entities = group
            .iter_visit_default(Vec::new(), |group, key, link, entities| {
                let entity = if let Ok(group) = group.group(key) {
                    GroupInfo::try_from_group_and_link(group, link).map(EntityInfo::Group)
                } else if let Ok(dataset) = group.dataset(key) {
                    Ok(EntityInfo::Dataset(DatasetInfo::from_dataset_and_link(
                        dataset, link,
                    )))
                } else {
                    Err(anyhow!("Found link to entity of unknown kind"))
                };
                entities.push(entity);
                true
            })?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            name,
            id,
            link_kind: link.link_type.into(),
            entities,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    pub name: String,
    pub id: i64,
    pub link_type: LinkKind,
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

impl DatasetInfo {
    fn from_dataset_and_link(dataset: Dataset, link: LinkInfo) -> Self {
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
            link_type: link.link_type.into(),
            shape,
            layout_info,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LinkKind {
    Hard,
    Soft,
    External,
}

impl From<LinkType> for LinkKind {
    fn from(value: LinkType) -> Self {
        match value {
            LinkType::Hard => Self::Hard,
            LinkType::Soft => Self::Soft,
            LinkType::External => Self::External,
        }
    }
}

impl ToString for LinkKind {
    fn to_string(&self) -> String {
        match self {
            Self::Hard => "Hard".to_string(),
            Self::Soft => "Soft".to_string(),
            Self::External => "External".to_string(),
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
        let entities = GroupInfo::try_from_group_and_link(
            file.as_group()?,
            LinkInfo {
                link_type: LinkType::Hard,
                creation_order: None,
                is_utf8: true,
            },
        )?
        .entities;

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
