mod nsp;
mod unpacked;
mod zarchive;

use self::{nsp::Nsp, unpacked::Unpacked, zarchive::ZArchive};
use enum_dispatch::enum_dispatch;
use parking_lot::{lock_api::MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::sync::Arc;
use uk_content::resource::ResourceData;

pub type ResourceCache =
    std::collections::HashMap<String, ResourceData, xxhash_rust::xxh3::Xxh3Builder>;

#[enum_dispatch(RomSource)]
pub trait RomReader {
    fn get_file(&self, name: &str) -> Option<ResourceData>;
}

#[enum_dispatch]
#[derive(Debug)]
enum RomSource {
    ZArchive,
    Nsp,
    Unpacked,
}

pub struct GameRomReader {
    source: RomSource,
    cache: Arc<RwLock<ResourceCache>>,
}

impl std::fmt::Debug for GameRomReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameRomReader")
            .field("source", &self.source)
            .field("cache_len", &self.cache.read().len())
            .finish()
    }
}

impl GameRomReader {
    pub fn get_resource<'a>(
        &'a self,
        name: &str,
    ) -> Option<MappedRwLockReadGuard<'a, parking_lot::RawRwLock, ResourceData>> {
        if !self.cache.read().contains_key(name) {
            let res = self.source.get_file(name)?;
            self.cache.write().insert(name.to_owned(), res);
        }
        Some(RwLockReadGuard::map(self.cache.read(), |cache| {
            cache.get(name).unwrap()
        }))
    }
}
