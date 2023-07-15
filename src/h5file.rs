use anyhow::Context;
use hdf5::File;
use std::path::Path;

#[derive(Debug)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
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

        Ok(Self { name, size })
    }
}
