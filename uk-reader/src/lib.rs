// mod nsp;
mod unpacked;
mod zarchive;

use self::{unpacked::Unpacked, zarchive::ZArchive};
use anyhow::Context;
use moka::sync::Cache;
use roead::sarc::Sarc;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{canonicalize, platform_prefixes, prelude::Endian, resource::*};

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
const CACHE_SIZE: u64 = 10_000;
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
    ResourceCache::new(CACHE_SIZE)
}

#[derive(Serialize, Deserialize)]
pub struct ResourceReader {
    bin_type: BinType,
    source: Box<dyn ResourceLoader>,
    #[serde(skip, default = "construct_cache")]
    cache: ResourceCache,
}

impl std::fmt::Debug for ResourceReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceReader")
            .field("source", &self.source)
            .field("cache_len", &self.cache.entry_count())
            .finish()
    }
}

impl ResourceReader {
    pub fn from_zarchive(archive_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            source: Box::new(ZArchive::new(archive_path)?),
            cache: ResourceCache::new(CACHE_SIZE),
            bin_type: BinType::Nintendo,
        })
    }

    pub fn from_unpacked_dirs(
        content_dir: Option<impl AsRef<Path>>,
        update_dir: Option<impl AsRef<Path>>,
        aoc_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        Ok(Self {
            source: Box::new(Unpacked::new(content_dir, update_dir, aoc_dir, true)?),
            cache: ResourceCache::new(CACHE_SIZE),
            bin_type: BinType::Nintendo,
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
            source: Box::new(Unpacked::new(content_dir, None::<PathBuf>, aoc_dir, false)?),
            cache: ResourceCache::new(500),
            bin_type: BinType::Nintendo,
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
        let resource =
            self.cache
                .try_get_with(canon.clone(), || -> uk_content::Result<_> {
                    let data = self.source.get_data(path.as_ref())?;
                    let resource = match self.bin_type {
                        BinType::Nintendo => {
                            let data = roead::yaz0::decompress_if(data.as_slice());
                            let res = ResourceData::from_binary(canon.as_str(), data.as_ref())?;
                            if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                                self.process_sarc(Sarc::new(data.as_ref())?)?;
                            }
                            res
                        }
                        BinType::MiniCbor => minicbor_ser::from_slice(data.as_slice())
                            .map_err(anyhow::Error::from)?,
                    };
                    Ok(Arc::new(resource))
                })
                .map_err(|_| {
                    ROMError::FileNotFound(
                        path.as_ref().to_string_lossy().into(),
                        self.source.host_path().to_path_buf(),
                    )
                })?;
        Ok(resource)
    }

    fn process_sarc(&self, sarc: roead::sarc::Sarc) -> uk_content::Result<()> {
        for file in sarc.files() {
            let name = file.name().context("SARC file missing name")?.to_string();
            let canon = canonicalize(&name);
            if !self.cache.contains_key(&canon) {
                let data = file.data;
                let data = roead::yaz0::decompress_if(data);
                let resource = ResourceData::from_binary(&name, data.as_ref())?;
                if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                    self.process_sarc(Sarc::new(data.as_ref())?)?;
                }
                self.cache.insert(canon, Arc::new(resource));
            }
        }
        Ok(())
    }
}
