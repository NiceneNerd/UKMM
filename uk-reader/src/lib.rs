use once_cell::sync::{Lazy, OnceCell};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uk_content::{resource::ResourceData, Result, UKError};

type ResourceCache = HashMap<String, Arc<ResourceData>>;

pub static RESOURCE_CACHE: Lazy<Arc<RwLock<ResourceCache>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
pub static GAME_DUMP_ARCHIVE: OnceCell<Option<zarchive::reader::ZArchiveReader>> = OnceCell::new();

pub fn get_resource(name: &str) -> Result<Arc<ResourceData>> {
    let cache = RESOURCE_CACHE.read().unwrap();
    Ok(cache
        .get(name)
        .ok_or_else(|| UKError::MissingResource(name.to_string()))?
        .clone())
}
