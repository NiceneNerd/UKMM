mod nsp;
mod unpacked;
mod zarchive;

use self::{nsp::Nsp, unpacked::Unpacked, zarchive::ZArchive};
use enum_dispatch::enum_dispatch;
use moka::sync::Cache;
use std::sync::Arc;
use uk_content::resource::ResourceData;

pub type ResourceCache = Cache<String, Arc<ResourceData>>;

#[enum_dispatch(ROMSource)]
pub trait ROMReader {
    fn get_file_data(&self, name: &str) -> Option<ResourceData>;
    fn get_aoc_file_data(&self, name: &str) -> Option<ResourceData>;
    fn file_exists(&self, name: &str) -> bool;
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
    pub fn get_resource(&self, name: &str) -> Option<Arc<ResourceData>> {
        self.cache
            .try_get_with(name.to_owned(), || {
                self.source.get_file_data(name).map(Arc::new).ok_or(())
            })
            .ok()
    }

    pub fn get_resource_by_path(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Option<Arc<ResourceData>> {
        let canonical = uk_content::canonicalize(path);
        self.get_resource(&canonical)
    }

    fn lookup_real_path(&self, name: &str) -> Option<String> {
        let split = name.split('/');
        Some("".into())
    }
}
