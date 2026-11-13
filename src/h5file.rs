use crate::widgets::tree::TreeItem;
use anyhow::{Context, anyhow};
use hdf5::Container;
use hdf5::sync::sync;
use hdf5::types::{VarLenAscii, VarLenUnicode};
use hdf5::{
    Dataset, File, Group, LinkInfo, LinkType, Location, SliceOrIndex, dataset::Layout,
    filters::Filter, types::TypeDescriptor,
};
use hdf5_sys::h5a::H5Aread;
use hdf5_sys::h5d::H5Dread;
use hdf5_sys::h5p::H5P_DEFAULT;
use hdf5_sys::h5s::H5S_ALL;
use std::collections::HashMap;
#[cfg(test)]
use std::str::FromStr;
use std::{fmt::Display, path::Path};

#[cfg(test)]
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum EntityInfo {
    Group(GroupInfo),
    Dataset(DatasetInfo),
}

impl From<EntityInfo> for TreeItem<'static> {
    fn from(value: EntityInfo) -> Self {
        match value {
            EntityInfo::Group(info) => TreeItem::from(info),
            EntityInfo::Dataset(info) => TreeItem::from(info),
        }
    }
}

fn to_string(field: &Container) -> Result<String, anyhow::Error> {
    // Get the data type descriptor for the attribute
    let dtype = field.dtype()?.to_descriptor()?;
    match &dtype {
        // Handle variable-length ASCII string
        TypeDescriptor::VarLenAscii => {
            let value: VarLenAscii = field.read_scalar()?;
            Ok(value.as_str().to_owned())
        }
        // Handle variable-length UTF-8 string
        TypeDescriptor::VarLenUnicode => {
            let value: VarLenUnicode = field.read_scalar()?;
            Ok(value.as_str().to_owned())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U1) => {
            Ok(field.read_scalar::<i8>()?.to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U2) => {
            Ok(field.read_scalar::<i16>()?.to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U4) => {
            Ok(field.read_scalar::<i32>()?.to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U8) => {
            Ok(field.read_scalar::<i64>()?.to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U1) => {
            Ok(field.read_scalar::<u8>()?.to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U2) => {
            Ok(field.read_scalar::<u16>()?.to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U4) => {
            Ok(field.read_scalar::<u32>()?.to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U8) => {
            Ok(field.read_scalar::<u64>()?.to_string())
        }
        TypeDescriptor::Float(hdf5::types::FloatSize::U4) => {
            Ok(field.read_scalar::<f32>()?.to_string())
        }
        TypeDescriptor::Float(hdf5::types::FloatSize::U8) => {
            Ok(field.read_scalar::<f64>()?.to_string())
        }
        // Handle fixed-length ASCII string scalars and 1D arrays.
        TypeDescriptor::FixedAscii(size) => decode_fixed_width_attribute_strings(field, *size),
        // Handle fixed-length UTF-8 string
        TypeDescriptor::FixedUnicode(size) => decode_fixed_width_attribute_strings(field, *size),
        _ => Err(anyhow!("unsupported attribute string type: {dtype}")),
    }
}

fn decode_fixed_width_attribute_strings(
    field: &Container,
    element_width: usize,
) -> Result<String, anyhow::Error> {
    if element_width == 0 {
        return Ok(String::new());
    }

    let dtype = field.dtype()?;
    let mut raw = vec![0u8; field.storage_size() as usize];
    let result = sync(|| unsafe { H5Aread(field.id(), dtype.id(), raw.as_mut_ptr().cast()) });
    if result < 0 {
        return Err(anyhow!("failed to read fixed-width string bytes"));
    }

    decode_fixed_width_bytes(
        &raw,
        field.is_scalar(),
        field.ndim(),
        field.shape(),
        element_width,
    )
}

fn decode_fixed_width_dataset_strings(
    dataset: &Dataset,
    element_width: usize,
) -> Result<String, anyhow::Error> {
    if element_width == 0 {
        return Ok(String::new());
    }

    let dtype = dataset.dtype()?;
    let mut raw = vec![0u8; dataset.storage_size() as usize];
    let result = sync(|| unsafe {
        H5Dread(
            dataset.id(),
            dtype.id(),
            H5S_ALL,
            H5S_ALL,
            H5P_DEFAULT,
            raw.as_mut_ptr().cast(),
        )
    });
    if result < 0 {
        return Err(anyhow!("failed to read fixed-width string bytes"));
    }

    decode_fixed_width_bytes(
        &raw,
        dataset.is_scalar(),
        dataset.ndim(),
        dataset.shape(),
        element_width,
    )
}

fn decode_fixed_width_bytes(
    raw: &[u8],
    is_scalar: bool,
    ndim: usize,
    shape: Vec<usize>,
    element_width: usize,
) -> Result<String, anyhow::Error> {
    if is_scalar {
        return Ok(decode_fixed_width_string(raw));
    }

    if ndim != 1 {
        return Err(anyhow!("unsupported fixed-width string shape: {:?}", shape));
    }

    Ok(raw
        .chunks(element_width)
        .map(decode_fixed_width_string)
        .collect::<Vec<_>>()
        .join(", "))
}

fn decode_fixed_width_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_owned()
}

pub fn get_attrs(location: &Location) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    if let Ok(attr_names) = location.attr_names() {
        for name in attr_names {
            let attr = location.attr(&name).unwrap();
            let value = to_string(&attr).unwrap_or_else(|_| {
                format!("<{:?}>", attr.dtype().unwrap().to_descriptor().unwrap())
            });
            let mut label = name.clone();
            if name != "NX_class" && name != "target" {
                label = format!(
                    "{}|{}",
                    name,
                    attr.dtype().unwrap().to_descriptor().unwrap()
                )
            }
            attrs.insert(label, value);
        }
    };
    attrs
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub name: String,
    pub id: i64,
    pub link_kind: LinkKind,
    pub entities: Vec<EntityInfo>,
    pub attrs: HashMap<String, String>,
}

impl GroupInfo {
    fn try_from_group_and_link(group: Group, link: LinkInfo) -> Result<Self, anyhow::Error> {
        let name = group.name().split('/').next_back().unwrap().to_string();
        let id = group.id();
        let attrs = get_attrs(&group);
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
            attrs,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DatasetInfo {
    pub name: String,
    pub id: i64,
    pub link_type: LinkKind,
    pub data_value: String,
    pub plot_data: Option<Vec<(f64, f64)>>,
    pub shape: Vec<usize>,
    pub layout_info: DatasetLayoutInfo,
    pub dtype_descr: TypeDescriptor,
    pub attrs: HashMap<String, String>,
    dataset: Dataset,
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
        let name = dataset.name().split('/').next_back().unwrap().to_string();
        let id = dataset.id();
        let data_value = dataset_value(&dataset).unwrap_or_default();
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
        let dtype_descr = dataset.dtype().unwrap().to_descriptor().unwrap();
        let attrs = get_attrs(&dataset);

        Self {
            name,
            id,
            link_type: link.link_type.into(),
            data_value,
            plot_data: None,
            shape,
            layout_info,
            dtype_descr,
            attrs,
            dataset,
        }
    }

    pub fn load_plot_data(&mut self, minimum_samples: usize) -> Result<bool, anyhow::Error> {
        if self.plot_data.is_none() {
            self.plot_data = dataset_plot_data(&self.dataset, minimum_samples)?;
        }
        Ok(self.plot_data.is_some())
    }

    pub fn unload_plot_data(&mut self) -> bool {
        self.plot_data.take().is_some()
    }
}

fn dataset_value(dataset: &Dataset) -> Result<String, anyhow::Error> {
    let dtype = dataset.dtype()?.to_descriptor()?;
    match &dtype {
        TypeDescriptor::FixedAscii(size) | TypeDescriptor::FixedUnicode(size) => {
            decode_fixed_width_dataset_strings(dataset, *size)
        }
        _ if dataset.is_scalar() || is_string_dtype(&dtype) => to_string(dataset),
        TypeDescriptor::Integer(hdf5::types::IntSize::U1) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<i8>()?[0].to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U2) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<i16>()?[0].to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U4) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<i32>()?[0].to_string())
        }
        TypeDescriptor::Integer(hdf5::types::IntSize::U8) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<i64>()?[0].to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U1) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<u8>()?[0].to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U2) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<u16>()?[0].to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U4) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<u32>()?[0].to_string())
        }
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U8) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<u64>()?[0].to_string())
        }
        TypeDescriptor::Float(hdf5::types::FloatSize::U4) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<f32>()?[0].to_string())
        }
        TypeDescriptor::Float(hdf5::types::FloatSize::U8) if dataset.size() == 1 => {
            Ok(dataset.read_raw::<f64>()?[0].to_string())
        }
        _ => Ok(String::new()),
    }
}

fn dataset_plot_data(
    dataset: &Dataset,
    minimum_samples: usize,
) -> Result<Option<Vec<(f64, f64)>>, anyhow::Error> {
    if dataset.shape().len() != 1 || dataset.size() <= 1 {
        return Ok(None);
    }

    let sample_step = plot_sample_step(dataset.size(), minimum_samples);
    let span = SliceOrIndex::SliceTo {
        start: 0,
        step: sample_step,
        end: dataset.size(),
        block: 1,
    };
    let dtype = dataset.dtype()?.to_descriptor()?;
    let values: Vec<f64> = match dtype {
        TypeDescriptor::Integer(hdf5::types::IntSize::U1) => dataset
            .read_slice_1d::<i8, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Integer(hdf5::types::IntSize::U2) => dataset
            .read_slice_1d::<i16, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Integer(hdf5::types::IntSize::U4) => dataset
            .read_slice_1d::<i32, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Integer(hdf5::types::IntSize::U8) => dataset
            .read_slice_1d::<i64, _>(span)?
            .iter()
            .map(|value| *value as f64)
            .collect(),
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U1) => dataset
            .read_slice_1d::<u8, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U2) => dataset
            .read_slice_1d::<u16, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U4) => dataset
            .read_slice_1d::<u32, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Unsigned(hdf5::types::IntSize::U8) => dataset
            .read_slice_1d::<u64, _>(span)?
            .iter()
            .map(|value| *value as f64)
            .collect(),
        TypeDescriptor::Float(hdf5::types::FloatSize::U4) => dataset
            .read_slice_1d::<f32, _>(span)?
            .iter()
            .copied()
            .map(f64::from)
            .collect(),
        TypeDescriptor::Float(hdf5::types::FloatSize::U8) => dataset
            .read_slice_1d::<f64, _>(span)?
            .iter()
            .copied()
            .collect(),
        _ => return Ok(None),
    };

    Ok(Some(
        values
            .into_iter()
            .enumerate()
            .map(|(index, value)| ((index * sample_step) as f64, value))
            .collect(),
    ))
}

fn plot_sample_step(dataset_size: usize, minimum_samples: usize) -> usize {
    (dataset_size / minimum_samples.max(1)).max(1)
}

fn is_string_dtype(dtype: &TypeDescriptor) -> bool {
    matches!(
        dtype,
        TypeDescriptor::VarLenAscii
            | TypeDescriptor::VarLenUnicode
            | TypeDescriptor::FixedAscii(_)
            | TypeDescriptor::FixedUnicode(_)
    )
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

impl Display for LinkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Hard => "Hard",
            Self::Soft => "Soft",
            Self::External => "External",
        })
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

    pub fn load_plot_data(
        &mut self,
        index: Vec<usize>,
        minimum_samples: usize,
    ) -> Result<bool, anyhow::Error> {
        match self.entity_mut(index)? {
            EntityInfo::Dataset(dataset) => dataset.load_plot_data(minimum_samples),
            EntityInfo::Group(_) => Ok(false),
        }
    }

    pub fn unload_plot_data(&mut self, index: Vec<usize>) -> Result<bool, anyhow::Error> {
        Ok(match self.entity_mut(index)? {
            EntityInfo::Dataset(dataset) => dataset.unload_plot_data(),
            EntityInfo::Group(_) => false,
        })
    }

    fn entity_mut(&mut self, index: Vec<usize>) -> Result<&mut EntityInfo, anyhow::Error> {
        let mut indices = index.into_iter();
        let mut entity = self
            .entities
            .get_mut(indices.next().context("Index was empty")?)
            .context("No entity at index")?;
        for idx in indices {
            match entity {
                EntityInfo::Group(group) => {
                    entity = group.entities.get_mut(idx).context("No entity at index")?
                }
                EntityInfo::Dataset(_) => Err(anyhow!("Cannot index into a dataset"))?,
            }
        }
        Ok(entity)
    }

    pub fn to_tree_items(&self) -> Vec<TreeItem<'static>> {
        self.entities
            .iter()
            .cloned()
            .map(TreeItem::from)
            .collect::<Vec<_>>()
    }
}

// ---------- TESTS START HERE

#[cfg(test)]
fn get_file_path(filename: &str) -> PathBuf {
    // cargo sets where project root is
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir).join(filename)
}

#[test]
fn load_nexus_file() {
    let filepath = get_file_path("tests/simple_nexus.h5");
    assert!(filepath.exists());

    // load the nexus file and perform test on root
    let filehandle = FileInfo::read(filepath).unwrap();
    assert!(filehandle.name.ends_with("simple_nexus.h5"));
    assert_eq!(filehandle.size, 45656); // observed

    // other attempt at the tree
    assert_eq!(filehandle.entities.len(), 2); // root node and links
    //println!("{:?}", filehandle.entities[0]);
    // let entry = GroupInfo::from(filehandle.entities[0]);
    //assert_eq!(filehandle.entities[0]["name"], "entry");

    // get to the tree
    let filetree = filehandle.to_tree_items();
    assert_eq!(filetree.len(), 2); // root node and links
}

#[test]
fn get_attrs_reads_ascii_and_unicode_strings() {
    use hdf5::types::{FixedAscii, FixedUnicode, VarLenAscii, VarLenUnicode};

    let test_file =
        std::env::temp_dir().join(format!("nexplore-string-attrs-{}.h5", std::process::id()));
    let _ = std::fs::remove_file(&test_file);

    let file = File::create(&test_file).unwrap();

    let fixed_ascii = file
        .new_attr::<FixedAscii<11>>()
        .shape(())
        .create("fixed_ascii")
        .unwrap();
    fixed_ascii
        .as_writer()
        .write_scalar(&FixedAscii::<11>::from_ascii(b"ascii").unwrap())
        .unwrap();

    let fixed_ascii_1d = file
        .new_attr::<FixedAscii<11>>()
        .shape([2])
        .create("fixed_ascii_1d")
        .unwrap();
    fixed_ascii_1d
        .as_writer()
        .write(&[
            FixedAscii::<11>::from_ascii(b"alpha").unwrap(),
            FixedAscii::<11>::from_ascii(b"beta").unwrap(),
        ])
        .unwrap();

    let fixed_unicode = file
        .new_attr::<FixedUnicode<8>>()
        .shape(())
        .create("fixed_unicode")
        .unwrap();
    fixed_unicode
        .as_writer()
        .write_scalar(&FixedUnicode::<8>::from_str("h5").unwrap())
        .unwrap();

    let var_ascii = file
        .new_attr::<VarLenAscii>()
        .shape(())
        .create("var_ascii")
        .unwrap();
    var_ascii
        .as_writer()
        .write_scalar(&VarLenAscii::from_ascii(b"value").unwrap())
        .unwrap();

    let var_unicode = file
        .new_attr::<VarLenUnicode>()
        .shape(())
        .create("var_unicode")
        .unwrap();
    var_unicode
        .as_writer()
        .write_scalar(&VarLenUnicode::from_str("cafe\u{e9}").unwrap())
        .unwrap();

    let unsigned_u4 = file
        .new_attr::<u32>()
        .shape(())
        .create("unsigned_u4")
        .unwrap();
    unsigned_u4.as_writer().write_scalar(&42u32).unwrap();

    let signed_u1 = file.new_attr::<i8>().shape(()).create("signed_u1").unwrap();
    signed_u1.as_writer().write_scalar(&-8i8).unwrap();

    let signed_u2 = file
        .new_attr::<i16>()
        .shape(())
        .create("signed_u2")
        .unwrap();
    signed_u2.as_writer().write_scalar(&-16i16).unwrap();

    let signed_u4 = file
        .new_attr::<i32>()
        .shape(())
        .create("signed_u4")
        .unwrap();
    signed_u4.as_writer().write_scalar(&-32i32).unwrap();

    let signed_u8 = file
        .new_attr::<i64>()
        .shape(())
        .create("signed_u8")
        .unwrap();
    signed_u8.as_writer().write_scalar(&-64i64).unwrap();

    let unsigned_u1 = file
        .new_attr::<u8>()
        .shape(())
        .create("unsigned_u1")
        .unwrap();
    unsigned_u1.as_writer().write_scalar(&8u8).unwrap();

    let unsigned_u2 = file
        .new_attr::<u16>()
        .shape(())
        .create("unsigned_u2")
        .unwrap();
    unsigned_u2.as_writer().write_scalar(&16u16).unwrap();

    let unsigned_u8 = file
        .new_attr::<u64>()
        .shape(())
        .create("unsigned_u8")
        .unwrap();
    unsigned_u8.as_writer().write_scalar(&64u64).unwrap();

    let float_u4 = file.new_attr::<f32>().shape(()).create("float_u4").unwrap();
    float_u4.as_writer().write_scalar(&3.5f32).unwrap();

    let float_u8 = file.new_attr::<f64>().shape(()).create("float_u8").unwrap();
    float_u8.as_writer().write_scalar(&7.25f64).unwrap();

    let attrs = get_attrs(&file);

    assert_eq!(attrs.get("fixed_ascii|string (len 11)").unwrap(), "ascii");
    assert_eq!(
        attrs.get("fixed_ascii_1d|string (len 11)").unwrap(),
        "alpha, beta"
    );
    assert_eq!(attrs.get("fixed_unicode|unicode (len 8)").unwrap(), "h5");
    assert_eq!(attrs.get("var_ascii|string (var len)").unwrap(), "value");
    assert_eq!(
        attrs.get("var_unicode|unicode (var len)").unwrap(),
        "cafe\u{e9}"
    );
    assert_eq!(attrs.get("signed_u1|int8").unwrap(), "-8");
    assert_eq!(attrs.get("signed_u2|int16").unwrap(), "-16");
    assert_eq!(attrs.get("signed_u4|int32").unwrap(), "-32");
    assert_eq!(attrs.get("signed_u8|int64").unwrap(), "-64");
    assert_eq!(attrs.get("unsigned_u1|uint8").unwrap(), "8");
    assert_eq!(attrs.get("unsigned_u2|uint16").unwrap(), "16");
    assert_eq!(attrs.get("unsigned_u4|uint32").unwrap(), "42");
    assert_eq!(attrs.get("unsigned_u8|uint64").unwrap(), "64");
    assert_eq!(attrs.get("float_u4|float32").unwrap(), "3.5");
    assert_eq!(attrs.get("float_u8|float64").unwrap(), "7.25");

    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn dataset_plot_data_reads_a_strided_hdf5_span() {
    let test_file =
        std::env::temp_dir().join(format!("nexplore-plot-span-{}.h5", std::process::id()));
    let _ = std::fs::remove_file(&test_file);

    let file = File::create(&test_file).unwrap();
    let dataset = file
        .new_dataset::<i32>()
        .shape([1_000])
        .create("values")
        .unwrap();
    dataset
        .as_writer()
        .write(&(0..1_000).collect::<Vec<_>>())
        .unwrap();

    let points = dataset_plot_data(&dataset, 400).unwrap().unwrap();

    assert!(points.len() >= 400);
    assert!(points.len() < dataset.size());
    assert_eq!(points.first(), Some(&(0.0, 0.0)));
    assert_eq!(points.last(), Some(&(998.0, 998.0)));

    drop(dataset);
    drop(file);
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn dataset_info_reads_scalar_and_string_values() {
    use hdf5::types::{FixedAscii, VarLenUnicode};

    let test_file =
        std::env::temp_dir().join(format!("nexplore-dataset-values-{}.h5", std::process::id()));
    let _ = std::fs::remove_file(&test_file);

    let file = File::create(&test_file).unwrap();

    let scalar = file
        .new_dataset::<i32>()
        .shape(())
        .create("scalar")
        .unwrap();
    scalar.as_writer().write_scalar(&12).unwrap();

    let scalar_float = file
        .new_dataset::<f64>()
        .shape(())
        .create("scalar_float")
        .unwrap();
    scalar_float.as_writer().write_scalar(&7.25).unwrap();

    let scalar_unsigned = file
        .new_dataset::<u16>()
        .shape(())
        .create("scalar_unsigned")
        .unwrap();
    scalar_unsigned.as_writer().write_scalar(&16).unwrap();

    let shape_one_int = file
        .new_dataset::<i32>()
        .shape([1])
        .create("shape_one_int")
        .unwrap();
    shape_one_int.as_writer().write(&[99]).unwrap();

    let shape_one_float = file
        .new_dataset::<f32>()
        .shape([1])
        .create("shape_one_float")
        .unwrap();
    shape_one_float.as_writer().write(&[2.5]).unwrap();

    let string = file
        .new_dataset::<FixedAscii<12>>()
        .shape([2])
        .create("string")
        .unwrap();
    string
        .as_writer()
        .write(&[
            FixedAscii::<12>::from_ascii(b"alpha").unwrap(),
            FixedAscii::<12>::from_ascii(b"beta").unwrap(),
        ])
        .unwrap();

    let unicode = file
        .new_dataset::<VarLenUnicode>()
        .shape(())
        .create("unicode")
        .unwrap();
    unicode
        .as_writer()
        .write_scalar(&VarLenUnicode::from_str("cafe\u{e9}").unwrap())
        .unwrap();

    let array = file
        .new_dataset::<i32>()
        .shape([2])
        .create("array")
        .unwrap();
    array.as_writer().write(&[1, 2]).unwrap();

    let mut file_info = FileInfo::read(&test_file).unwrap();

    let scalar_info = match file_info.entity(vec![0]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let scalar_float_info = match file_info.entity(vec![1]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let scalar_unsigned_info = match file_info.entity(vec![2]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let shape_one_int_info = match file_info.entity(vec![3]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let shape_one_float_info = match file_info.entity(vec![4]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let string_info = match file_info.entity(vec![5]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let unicode_info = match file_info.entity(vec![6]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    let array_info = match file_info.entity(vec![7]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };

    assert_eq!(scalar_info.data_value, "12");
    assert_eq!(scalar_float_info.data_value, "7.25");
    assert_eq!(scalar_unsigned_info.data_value, "16");
    assert_eq!(shape_one_int_info.data_value, "99");
    assert_eq!(shape_one_float_info.data_value, "2.5");
    assert!(shape_one_int_info.plot_data.is_none());
    assert!(shape_one_float_info.plot_data.is_none());
    assert_eq!(string_info.data_value, "alpha, beta");
    assert!(string_info.plot_data.is_none());
    assert_eq!(unicode_info.data_value, "cafe\u{e9}");
    assert!(unicode_info.plot_data.is_none());
    assert!(array_info.data_value.is_empty());
    assert!(array_info.plot_data.is_none());

    assert!(file_info.load_plot_data(vec![7], 8).unwrap());
    let array_info = match file_info.entity(vec![7]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    assert_eq!(array_info.plot_data.as_ref().unwrap().len(), 2);
    assert!(!file_info.load_plot_data(vec![3], 8).unwrap());
    assert!(!file_info.load_plot_data(vec![4], 8).unwrap());
    assert!(!file_info.load_plot_data(vec![5], 8).unwrap());
    assert!(file_info.unload_plot_data(vec![7]).unwrap());
    let array_info = match file_info.entity(vec![7]).unwrap() {
        EntityInfo::Dataset(dataset) => dataset,
        EntityInfo::Group(_) => panic!("expected dataset"),
    };
    assert!(array_info.plot_data.is_none());
    assert!(!file_info.unload_plot_data(vec![7]).unwrap());

    std::fs::remove_file(test_file).unwrap();
}
