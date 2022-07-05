mod nsp;
mod unpacked;
mod zarchive;

use self::{nsp::Nsp, unpacked::Unpacked, zarchive::ZArchive};
use anyhow::Context;
use enum_dispatch::enum_dispatch;
use moka::sync::Cache;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{
    canonicalize,
    prelude::Resource,
    resource::{MergeableResource, ResourceData},
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ROMError {
    #[error("File not found in game dump: {0}\n(Using ROM at {1})")]
    FileNotFound(String, PathBuf),
    #[error("Invalid resource path: {0}")]
    InvalidPath(String),
}

type ResourceCache = Cache<String, Arc<ResourceData>>;
pub type Result<T> = std::result::Result<T, ROMError>;

#[enum_dispatch(ROMSource)]
pub trait ROMReader {
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<ResourceData>;
    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<ResourceData>;
    fn file_exists(&self, name: impl AsRef<Path>) -> bool;
    fn host_path(&self) -> &Path;
}

#[enum_dispatch]
#[derive(Debug)]
enum ROMSource {
    ZArchive,
    Nsp,
    Unpacked,
}

pub struct GameROMReader {
    source: ROMSource,
    cache: ResourceCache,
}

impl std::fmt::Debug for GameROMReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameROMReader")
            .field("source", &self.source)
            .field("cache_len", &self.cache.entry_count())
            .finish()
    }
}

impl GameROMReader {
    pub fn get_resource(&self, name: impl AsRef<Path>) -> Result<Arc<ResourceData>> {
        let name = name
            .as_ref()
            .to_str()
            .ok_or_else(|| ROMError::InvalidPath(name.as_ref().to_string_lossy().into_owned()))?
            .to_owned();
        self.cache
            .get(&name)
            .ok_or_else(|| ROMError::FileNotFound(name, self.source.host_path().to_path_buf()))
    }

    pub fn get_file<T: Resource>(&self, path: impl AsRef<Path>) -> Result<Arc<T>> {
        let canon = canonicalize(path);
        self.cache
            .get_with(canon, || {
                let data = self.source.get_file_data(path)?;
                T::from_binary(data.as_slice()).unwrap()
            })
            .ok_or_else(|| ROMError::FileNotFound(name, self.source.host_path().to_path_buf()))
    }
}
