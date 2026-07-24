// mod nsp;
mod unpacked;
mod zarchive;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::Context;
use dashmap::DashMap;
use dyn_clone::DynClone;
use include_flate::flate;
use join_str::jstr;
use moka::sync::Cache;
use roead::sarc::Sarc;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{
    canonicalize, constants::Language, platform_prefixes, prelude::Endian, resource::*,
};
use uk_util::PathExt;

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
    Any(#[from] anyhow_ext::Error),
}

impl From<ROMError> for uk_content::UKError {
    fn from(err: ROMError) -> Self {
        Self::Any(err.into())
    }
}

flate!(static MAP_SRC_U: str from "data/filemap_wiiu.json");
const FILE_MAP_U: LazyLock<Arc<DashMap<String, [Arc<&'static str>;3]>>> =
    LazyLock::new(|| Arc::new(serde_json::from_str(MAP_SRC_U.as_ref()).unwrap()));
flate!(static MAP_SRC_NX: str from "data/filemap_nx.json");
const FILE_MAP_NX: LazyLock<Arc<DashMap<String, [Arc<&'static str>;3]>>> =
    LazyLock::new(|| Arc::new(serde_json::from_str(MAP_SRC_NX.as_ref()).unwrap()));
type ResourceCache = Cache<String, Arc<ResourceData>>;
type SarcCache = Cache<String, Arc<Sarc<'static>>>;
const CACHE_SIZE: usize = 10000;
pub type Result<T> = std::result::Result<T, ROMError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinType {
    Nintendo,
    MiniCbor,
}

#[typetag::serde(tag = "type")]
pub trait ResourceLoader: std::fmt::Debug + Send + Sync + DynClone {
    fn get_base_file_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn get_update_file_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn file_exists(&self, name: &Path) -> bool;
    fn host_path(&self) -> &Path;
}

dyn_clone::clone_trait_object!(ResourceLoader);

fn construct_res_cache() -> ResourceCache {
    log::debug!(
        "Initializing resource cache (up to {} resources)",
        CACHE_SIZE
    );
    ResourceCache::builder()
        .max_capacity(CACHE_SIZE as u64)
        .initial_capacity(CACHE_SIZE / 10)
        .time_to_idle(Duration::from_secs(30))
        .build()
}

fn construct_sarc_cache() -> SarcCache {
    Cache::new(100)
}

#[derive(Serialize, Deserialize)]
pub struct YAMLResourceReader {
    bin_type: BinType,
    source: Box<dyn ResourceLoader>,
    endian: Option<Endian>,
}

impl From<ResourceReader> for YAMLResourceReader {
    fn from(value: ResourceReader) -> Self {
        Self {
            endian: match &value.source.file_exists(&PathBuf::from("Movie/Demo101_0.mp4")) {
                false => Some(Endian::Little),
                true => Some(Endian::Big),
            },
            bin_type: value.bin_type,
            source: value.source,
        }
    }
}

impl From<YAMLResourceReader> for ResourceReader {
    fn from(value: YAMLResourceReader) -> Self {
        Self {
            file_map: match value.endian {
                Some(e) => match e {
                    Endian::Little => FILE_MAP_NX.clone(),
                    Endian::Big => FILE_MAP_U.clone(),
                },
                None => match &value.source.file_exists(&PathBuf::from("Movie/Demo101_0.mp4")) {
                    false => FILE_MAP_NX.clone(),
                    true => FILE_MAP_U.clone(),
                },
            },
            bin_type: value.bin_type,
            source: value.source,
            cache: construct_res_cache(),
            sarc_cache: construct_sarc_cache(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "YAMLResourceReader", into = "YAMLResourceReader")]
pub struct ResourceReader {
    bin_type: BinType,
    source: Box<dyn ResourceLoader>,
    cache: ResourceCache,
    sarc_cache: SarcCache,
    file_map: Arc<DashMap<String, [Arc<&'static str>; 3]>>,
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
    pub fn clear_cache(&self) {
        self.cache.invalidate_all();
    }

    pub fn source(&self) -> &dyn ResourceLoader {
        self.source.as_ref()
    }

    pub fn source_ser(&self) -> std::string::String {
        serde_json::to_string(&self.source).unwrap()
    }

    pub fn from_zarchive(archive_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            source: Box::new(ZArchive::new(archive_path)?),
            cache: construct_res_cache(),
            sarc_cache: construct_sarc_cache(),
            bin_type: BinType::Nintendo,
            file_map: FILE_MAP_U.clone(),
        })
    }

    pub fn from_unpacked_dirs(
        content_dir: Option<impl AsRef<Path>>,
        update_dir: Option<impl AsRef<Path>>,
        aoc_dir: Option<impl AsRef<Path>>,
        endian: Endian,
    ) -> Result<Self> {
        Ok(Self {
            source: Box::new(Unpacked::new(content_dir, update_dir, aoc_dir, true)?),
            cache: construct_res_cache(),
            sarc_cache: construct_sarc_cache(),
            bin_type: BinType::Nintendo,
            file_map: match endian {
                Endian::Little => FILE_MAP_NX.clone(),
                Endian::Big => FILE_MAP_U.clone(),
            },
        })
    }

    pub fn from_unpacked_mod(mod_dir: impl AsRef<Path>) -> Result<Self> {
        fn inner(mod_dir: &Path) -> Result<ResourceReader> {
            let endian = match mod_dir.join("content").exists() {
                true => Endian::Big,
                false => Endian::Little,
            };
            let (content, aoc) = platform_prefixes(endian);
            let content_dir = mod_dir
                .join(content)
                .exists_then();
            let update_dir = match endian {
                Endian::Big => mod_dir
                    .join(content)
                    .exists_then(),
                Endian::Little => None,
            };
            let aoc_dir = mod_dir
                .join(aoc)
                .exists_then();
            Ok(ResourceReader {
                source: Box::new(Unpacked::new(content_dir, update_dir, aoc_dir, false)?),
                cache: construct_res_cache(),
                sarc_cache: construct_sarc_cache(),
                bin_type: BinType::Nintendo,
                file_map: match endian {
                    Endian::Little => FILE_MAP_NX.clone(),
                    Endian::Big => FILE_MAP_U.clone(),
                },
            })
        }
        inner(mod_dir.as_ref())
    }

    pub fn get_data(&self, path: impl AsRef<Path>) -> Result<Arc<ResourceData>> {
        let canon = canonicalize(path.as_ref());
        log::trace!("Loading resource {}", &canon);
        self
            .cache
            .try_get_with(canon.clone(), || -> Result<_> {
                log::trace!("Resource {} not in cache, pulling", &canon);
                let data = self.get_bytes_uncached(path)?;
                let resource = match self.bin_type {
                    BinType::Nintendo => {
                        let data = roead::yaz0::decompress_if(data.as_slice());
                        ResourceData::from_binary(canon.as_str(), data.as_ref())?
                    }
                    BinType::MiniCbor => {
                        minicbor_ser::from_slice(data.as_slice())
                            .map_err(anyhow_ext::Error::from)?
                    }
                };
                Ok(Arc::new(resource))
            })
            .map_err(|e| Arc::try_unwrap(e)
                .unwrap_or_else(|e| anyhow::format_err!("{e}").into())
            )
    }

    pub fn get_bytes_uncached(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let canon = canonicalize(path.as_ref());
        self.file_map.get(&canon)
            .with_context(|| jstr!("File {&canon} not in vanilla files"))?
            .iter()
            .enumerate()
            .find_map(|(dump, file_path)| {
                match file_path.is_empty() {
                    false => {
                        match file_path.contains("//") {
                            false => {
                                match dump {
                                    0 => Some(self.source.get_aoc_file_data(&PathBuf::from(file_path.to_string()))),
                                    1 => Some(self.source.get_update_file_data(&PathBuf::from(file_path.to_string()))),
                                    2 => Some(self.source.get_base_file_data(&PathBuf::from(file_path.to_string()))),
                                    _ => unreachable!("An [Arc<&str>;3] cannot be longer than length 3")
                                }
                            },
                            true => Some(self.get_bytes_from_sarc(file_path, dump == 0)),
                        }
                    },
                    true => None,
                }
            })
            .transpose()?
            .ok_or_else(|| ROMError::FileNotFound(canon, self.source.host_path().into()))
    }

    pub fn get_aoc_bytes_uncached(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let canon = canonicalize(path.as_ref());
        self.file_map.get(&canon)
            .with_context(|| jstr!("File {&canon} not in vanilla files"))?
            .iter()
            .next()
            .iter()
            .find_map(|file_path| {
                match file_path.is_empty() {
                    false => {
                        match file_path.contains("//") {
                            false => Some(self.source.get_aoc_file_data(&PathBuf::from(file_path.to_string()))),
                            true => Some(self.get_bytes_from_sarc(file_path, true)),
                        }
                    },
                    true => None,
                }
            })
            .transpose()?
            .ok_or_else(|| ROMError::FileNotFound(canon, self.source.host_path().into()))
    }

    pub fn get_bytes_from_sarc(&self, nest_path: &str, aoc: bool) -> Result<Vec<u8>> {
        let parts = nest_path.split("//").collect::<Vec<_>>();
        let root_str = format!("Aoc/0010/{}", parts[0]);
        let root = aoc.then_some(root_str.as_str()).unwrap_or(parts[0]);
        let (super_container_path, container_path, file_path) = match parts.len() {
            2 => (None, root, parts[1]),
            3 => (Some(root), parts[1], parts[2]),
            _ => unreachable!("All sarc paths have 2 or 3 parts")
        };
        let parent = self
            .sarc_cache
            .try_get_with(container_path.into(), || -> Result<_> {
                match super_container_path {
                    None => Ok(Arc::new(Sarc::new(self.get_bytes_uncached(container_path)?)
                        .map_err(|e| anyhow::format_err!("{e}"))?)),
                    Some(p) => {
                        let root = self
                            .sarc_cache
                            .try_get_with(p.into(), || -> anyhow_ext::Result<Arc<Sarc<'static>>> {
                                Ok(Arc::new(Sarc::new(self.get_bytes_uncached(p)?)
                                    .map_err(|e| anyhow::format_err!("{e}"))?))
                            })
                            .map_err(|e| {
                                Arc::try_unwrap(e)
                                    .unwrap_or_else(|e| anyhow::format_err!("{e}"))
                            })?;
                        let sarc = Sarc::new(
                            root.get_data(container_path)
                                .context("Couldn't get nested SARC")?
                                .to_vec(),
                        )
                        .map_err(|e| 
                            ROMError::FileNotFound(format!("{e}").into(), container_path.into())
                        )?;
                        Ok(Arc::new(sarc))
                    }
                }
            })
            .map_err(|e| Arc::try_unwrap(e)
                .unwrap_or_else(|e| ROMError::Any(e.into()))
            )?;
        Ok(roead::yaz0::decompress_if(
            parent
                .get_data(file_path)
                .with_context(|| format!("Could not get nested file at {nest_path}"))?,
        )
        .into())
    }

    pub fn languages(
        &self,
    ) -> dashmap::mapref::one::RefMut<
        '_,
        std::path::PathBuf,
        std::vec::Vec<uk_content::constants::Language>,
    > {
        static LANGS: LazyLock<DashMap<PathBuf, Vec<Language>>> = LazyLock::new(Default::default);
        LANGS
            .entry(self.source().host_path().to_path_buf())
            .or_insert_with(|| {
                Language::iter()
                    .filter(|l| self.source().file_exists(l.bootup_path().as_str().as_ref()))
                    .copied()
                    .collect()
            })
    }
}
