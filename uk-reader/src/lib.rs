#![feature(let_chains)]
// mod nsp;
mod unpacked;
mod zarchive;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use moka::sync::Cache;
use parking_lot::RwLock;
use roead::sarc::Sarc;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{canonicalize, platform_prefixes, prelude::Endian, resource::*, util::HashMap};

use self::{unpacked::Unpacked, zarchive::ZArchive};

#[derive(Debug, thiserror::Error)]
pub enum ROMError {
    #[error("File not found in game dump: {0}\n(Using source at {1})")]
    FileNotFound(String, PathBuf),
    #[error("Missing required {0} folder in game dump\n(Using ROM at {1})")]
    MissingDumpDir(&'static str, PathBuf),
    #[error("Invalid resource path: {0}")]
    InvalidPath(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    WUAError(#[from] ::zarchive::ZArchiveError),
    #[error(transparent)]
    UKError(#[from] uk_content::UKError),
    #[error("{0}")]
    OtherMessage(&'static str),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

impl From<ROMError> for uk_content::UKError {
    fn from(err: ROMError) -> Self {
        Self::Any(err.into())
    }
}

type ResourceCache = Cache<String, Arc<ResourceData>>;
const CACHE_SIZE: u64 = 7777;
pub type Result<T> = std::result::Result<T, ROMError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinType {
    Nintendo,
    MiniCbor,
}

#[typetag::serde(tag = "type")]
pub trait ResourceLoader: std::fmt::Debug + Send + Sync {
    fn get_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn file_exists(&self, name: &Path) -> bool;
    fn host_path(&self) -> &Path;
}

fn construct_cache() -> ResourceCache {
    log::debug!(
        "Initializing resource cache (up to {} resources)",
        CACHE_SIZE
    );
    ResourceCache::new(CACHE_SIZE)
}

#[derive(Serialize, Deserialize)]
pub struct ResourceReader {
    bin_type: BinType,
    source:   Box<dyn ResourceLoader>,
    #[serde(skip, default = "construct_cache")]
    cache:    ResourceCache,
    #[serde(skip)]
    nest_map: Arc<RwLock<HashMap<String, Arc<str>>>>,
}

impl PartialEq for ResourceReader {
    fn eq(&self, other: &Self) -> bool {
        self.bin_type == other.bin_type && self.source.host_path() == other.source.host_path()
    }
}

impl std::fmt::Debug for ResourceReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceReader")
            .field("bin_type", &self.bin_type)
            .field("source", &self.source)
            .field("cache_len", &self.cache.entry_count())
            .finish()
    }
}

impl ResourceReader {
    pub fn source(&self) -> &dyn ResourceLoader {
        self.source.as_ref()
    }

    pub fn source_ser(&self) -> std::string::String {
        serde_json::to_string(&self.source).unwrap()
    }

    pub fn from_zarchive(archive_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            source:   Box::new(ZArchive::new(archive_path)?),
            cache:    ResourceCache::new(CACHE_SIZE),
            bin_type: BinType::Nintendo,
            nest_map: Default::default(),
        })
    }

    pub fn from_unpacked_dirs(
        content_dir: Option<impl AsRef<Path>>,
        update_dir: Option<impl AsRef<Path>>,
        aoc_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        Ok(Self {
            source:   Box::new(Unpacked::new(content_dir, update_dir, aoc_dir, true)?),
            cache:    ResourceCache::new(CACHE_SIZE),
            bin_type: BinType::Nintendo,
            nest_map: Default::default(),
        })
    }

    #[allow(irrefutable_let_patterns)]
    pub fn from_unpacked_mod(mod_dir: impl AsRef<Path>) -> Result<Self> {
        let mod_dir = mod_dir.as_ref();
        let (content_u, aoc_u) = platform_prefixes(Endian::Big);
        let (content_nx, aoc_nx) = platform_prefixes(Endian::Little);
        let content_dir = if let content_u = mod_dir.join(content_u) && content_u.exists() {
            Some(content_u)
        } else if let content_nx = mod_dir.join(content_nx) && content_nx.exists() {
            Some(content_nx)
        } else {
            None
        };
        let aoc_dir = if let aoc_u = mod_dir.join(aoc_u) && aoc_u.exists() {
            Some(aoc_u)
        } else if let aoc_nx = mod_dir.join(aoc_nx) && aoc_nx.exists() {
            Some(aoc_nx)
        } else {
            None
        };
        Ok(Self {
            source:   Box::new(Unpacked::new(content_dir, None::<PathBuf>, aoc_dir, false)?),
            cache:    ResourceCache::new(500),
            bin_type: BinType::Nintendo,
            nest_map: Default::default(),
        })
    }

    pub fn get_resource(&self, name: impl AsRef<Path>) -> Result<Arc<ResourceData>> {
        let name = name
            .as_ref()
            .to_str()
            .ok_or_else(|| ROMError::InvalidPath(name.as_ref().to_string_lossy().into()))?
            .into();
        self.cache
            .get(&name)
            .ok_or_else(|| ROMError::FileNotFound(name, self.source.host_path().to_path_buf()))
    }

    pub fn get_data(&self, path: impl AsRef<Path>) -> Result<Arc<ResourceData>> {
        let canon = canonicalize(path.as_ref());
        Ok(self.get_or_add_resource(path, canon)?)
    }

    fn get_or_add_resource(
        &self,
        path: impl AsRef<Path>,
        canon: String,
    ) -> uk_content::Result<Arc<ResourceData>> {
        log::trace!("Loading resource {}", &canon);
        match self
            .cache
            .try_get_with(canon.clone(), || -> uk_content::Result<_> {
                log::trace!("Resource {} not in cache, pulling", &canon);
                let data = self.source.get_data(path.as_ref())?;
                let resource = match self.bin_type {
                    BinType::Nintendo => {
                        let data = roead::yaz0::decompress_if(data.as_slice());
                        let res = ResourceData::from_binary(canon.as_str(), data.as_ref())?;
                        if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                            self.process_sarc(
                                Sarc::new(data.as_ref())?,
                                path.as_ref().display().to_string().as_str(),
                            )?;
                        }
                        res
                    }
                    BinType::MiniCbor => {
                        minicbor_ser::from_slice(data.as_slice()).map_err(anyhow::Error::from)?
                    }
                };
                Ok(Arc::new(resource))
            }) {
            Ok(res) => Ok(res),
            Err(_) => {
                let parent = self.nest_map.read().get(&canon).cloned();
                match parent {
                    Some(parent) => {
                        let parent_canon = canonicalize(parent.as_ref());
                        dbg!(&parent_canon);
                        dbg!(&parent);
                        self.get_or_add_resource(parent.as_ref(), parent_canon)?;
                        Ok(self.cache.get(&canon).ok_or_else(|| {
                            ROMError::FileNotFound(
                                path.as_ref().to_string_lossy().into(),
                                self.source.host_path().to_path_buf(),
                            )
                        })?)
                    }
                    None => {
                        Err(ROMError::FileNotFound(
                            path.as_ref().to_string_lossy().into(),
                            self.source.host_path().to_path_buf(),
                        )
                        .into())
                    }
                }
            }
        }
    }

    fn process_sarc(&self, sarc: roead::sarc::Sarc, sarc_path: &str) -> uk_content::Result<()> {
        log::trace!("Resource is SARC, add contents to cache");
        for file in sarc.files() {
            let name = file.name().context("SARC file missing name")?.to_string();
            let canon = canonicalize(&name);
            if !self.cache.contains_key(&canon) {
                let data = file.data;
                let data = roead::yaz0::decompress_if(data);
                let resource = ResourceData::from_binary(&name, data.as_ref())?;
                if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                    self.process_sarc(Sarc::new(data.as_ref())?, &name)?;
                }
                self.cache.insert(canon.clone(), Arc::new(resource));
                self.nest_map.write().insert(canon, sarc_path.into());
            }
        }
        Ok(())
    }
}
