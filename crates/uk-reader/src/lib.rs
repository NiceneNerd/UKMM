// mod nsp;
mod unpacked;
mod zarchive;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow_ext::Context;
use dashmap::DashMap;
use include_flate::flate;
use join_str::jstr;
use moka::sync::Cache;
// use once_cell::sync::Lazy;
use roead::sarc::Sarc;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{
    canonicalize, constants::Language, platform_prefixes, prelude::Endian, resource::*,
};
use uk_util::{Lazy, PathExt};

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

flate!(static NEST_MAP: str from "data/nest_map.json");
type ResourceCache = Cache<String, Arc<ResourceData>>;
type SarcCache = Cache<String, Arc<Sarc<'static>>>;
const CACHE_SIZE: usize = 10000;
pub type Result<T> = std::result::Result<T, ROMError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinType {
    Nintendo,
    MiniCbor,
}

pub struct LanguageIterator<'a, T, U>
where
    T: Iterator<Item = Language>,
    U: ResourceLoader,
{
    iter: T,
    res:  &'a U,
}

impl<'a, T, U> Iterator for LanguageIterator<'a, T, U>
where
    T: Iterator<Item = Language>,
    U: ResourceLoader,
{
    type Item = Language;

    fn next(&mut self) -> Option<Self::Item>
    where
        T: Iterator,
        U: ResourceLoader,
    {
        self.iter
            .find(|l| self.res.file_exists(l.bootup_path().as_str().as_ref()))
    }
}

#[typetag::serde(tag = "type")]
pub trait ResourceLoader: std::fmt::Debug + Send + Sync {
    fn get_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>>;
    fn file_exists(&self, name: &Path) -> bool;
    fn host_path(&self) -> &Path;
}

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

fn init_nest_map() -> Arc<DashMap<String, Arc<str>>> {
    log::trace!("Initializing nest map...");
    static STOCK: Lazy<Arc<DashMap<String, Arc<str>>>> =
        Lazy::new(|| Arc::new(serde_json::from_str(NEST_MAP.as_ref()).unwrap()));
    STOCK.clone()
}

#[derive(Serialize, Deserialize)]
pub struct ResourceReader {
    bin_type: BinType,
    source: Box<dyn ResourceLoader>,
    #[serde(skip, default = "construct_res_cache")]
    cache: ResourceCache,
    #[serde(skip, default = "construct_sarc_cache")]
    sarc_cache: SarcCache,
    #[serde(skip, default = "init_nest_map")]
    nest_map: Arc<DashMap<String, Arc<str>>>,
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
            nest_map: init_nest_map(),
        })
    }

    pub fn from_unpacked_dirs(
        content_dir: Option<impl AsRef<Path>>,
        update_dir: Option<impl AsRef<Path>>,
        aoc_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        Ok(Self {
            source: Box::new(Unpacked::new(content_dir, update_dir, aoc_dir, true)?),
            cache: construct_res_cache(),
            sarc_cache: construct_sarc_cache(),
            bin_type: BinType::Nintendo,
            nest_map: init_nest_map(),
        })
    }

    pub fn from_unpacked_mod(mod_dir: impl AsRef<Path>) -> Result<Self> {
        fn inner(mod_dir: &Path) -> Result<ResourceReader> {
            let (content_u, aoc_u) = platform_prefixes(Endian::Big);
            let (content_nx, aoc_nx) = platform_prefixes(Endian::Little);
            let content_dir = mod_dir
                .join(content_u)
                .exists_then()
                .or_else(|| mod_dir.join(content_nx).exists_then());
            let aoc_dir = mod_dir
                .join(aoc_u)
                .exists_then()
                .or_else(|| mod_dir.join(aoc_nx).exists_then());
            Ok(ResourceReader {
                source: Box::new(Unpacked::new(content_dir, None::<PathBuf>, aoc_dir, false)?),
                cache: construct_res_cache(),
                sarc_cache: construct_sarc_cache(),
                bin_type: BinType::Nintendo,
                nest_map: init_nest_map(),
            })
        }
        inner(mod_dir.as_ref())
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
        Ok(self.get_or_add_resource(path.as_ref(), canon)?)
    }

    pub fn get_bytes_uncached(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.source().get_data(path.as_ref())
    }

    pub fn get_aoc_bytes_uncached(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.source().get_aoc_file_data(path.as_ref())
    }

    pub fn get_bytes_from_sarc(&self, nest_path: &str) -> uk_content::Result<Vec<u8>> {
        let parts = nest_path.split("//").collect::<Vec<_>>();
        let root = self
            .sarc_cache
            .try_get_with(
                canonicalize(parts[0]),
                || -> anyhow_ext::Result<Arc<Sarc<'static>>> {
                    let sarc = self.source().get_data(parts[0].as_ref()).with_context(|| {
                        format!(
                            "Failed to get parent for nested file at {} using source {}",
                            nest_path,
                            self.source_ser()
                        )
                    })?;
                    Ok(Arc::new(Sarc::new(sarc)?))
                },
            )
            .map_err(|e| Arc::try_unwrap(e).unwrap_or_else(|e| anyhow::format_err!("{e}")))?;
        let nested_parent = if parts.len() == 3 {
            let root = root.clone();
            Some(
                self.sarc_cache
                    .try_get_with(canonicalize(parts[1]), || -> uk_content::Result<_> {
                        let sarc = Sarc::new(
                            root.get_data(parts[1])
                                .context("Couldn't get nested SARC")?
                                .to_vec(),
                        )?;
                        Ok(Arc::new(sarc))
                    })
                    .map_err(|e| Arc::try_unwrap(e).expect("Eh"))?,
            )
        } else {
            None
        };
        let parent = nested_parent.as_ref().unwrap_or(&root);
        Ok(roead::yaz0::decompress_if(
            parent
                .get_data(parts[parts.len() - 1])
                .with_context(|| format!("Could not get nested file at {nest_path}"))?,
        )
        .into())
    }

    pub fn get_from_sarc(
        &self,
        canon: &str,
        nest_path: &str,
    ) -> uk_content::Result<Arc<ResourceData>> {
        let data = self
            .get_bytes_from_sarc(nest_path)
            .with_context(|| format!("Failed to read {} from SARC at path {}", canon, nest_path))?;
        let resource = ResourceData::from_binary(canon, &data)
            .with_context(|| jstr!("Failed to parse resource {canon}"))?;
        if is_mergeable_sarc(canon, &data) {
            self.process_sarc(
                Sarc::new(&data)
                    .with_context(|| format!("Failed to parse nested SARC at {}", nest_path))?,
                nest_path.split("//").last().unwrap_or_default(),
            )?;
        }
        Ok(self.cache.get_with(canon.into(), || Arc::new(resource)))
    }

    fn get_or_add_resource(
        &self,
        path: &Path,
        canon: String,
    ) -> uk_content::Result<Arc<ResourceData>> {
        log::trace!("Loading resource {}", &canon);
        let res_result = self
            .cache
            .try_get_with(canon.clone(), || -> uk_content::Result<_> {
                log::trace!("Resource {} not in cache, pulling", &canon);
                let data = self
                    .source
                    .get_data(path)
                    .with_context(|| jstr!("File {&canon} not found in dump"))?;
                let resource = match self.bin_type {
                    BinType::Nintendo => {
                        let data = roead::yaz0::decompress_if(data.as_slice());
                        let res = ResourceData::from_binary(canon.as_str(), data.as_ref())?;
                        if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                            self.process_sarc(
                                Sarc::new(data.as_ref())?,
                                path.display().to_string().as_str(),
                            )?;
                        }
                        res
                    }
                    BinType::MiniCbor => {
                        minicbor_ser::from_slice(data.as_slice())
                            .map_err(anyhow_ext::Error::from)?
                    }
                };
                Ok(Arc::new(resource))
            });
        match res_result {
            Ok(res) => Ok(res),
            Err(e) => {
                log::trace!("Failed to get file from dump: {e}. Performing parent lookup...");
                let nest_path = self.nest_map.get(&canon);
                log::trace!("{canon} has parent? {}", nest_path.is_some());
                match nest_path {
                    Some(parent) => {
                        log::trace!("Full path found at {}", parent.as_ref());
                        match self.get_from_sarc(&canon, &parent) {
                            Err(e) => {
                                log::warn!("Failed to get {canon} from {}: {e:?}", parent.as_ref());
                                Err(e)
                            }
                            Ok(v) => Ok(v),
                        }
                    }
                    None => {
                        Err(ROMError::FileNotFound(
                            path.to_string_lossy().into(),
                            self.source.host_path().to_path_buf(),
                        )
                        .into())
                    }
                }
            }
        }
    }

    fn process_sarc(&self, sarc: roead::sarc::Sarc, _sarc_path: &str) -> uk_content::Result<()> {
        log::trace!("Resource is SARC, add contents to cache");
        for file in sarc.files() {
            let name = file.name().context("SARC file missing name")?.to_string();
            let canon = canonicalize(&name);
            if !self.cache.contains_key(&canon) {
                let data = file.data;
                let data = roead::yaz0::decompress_if(data);
                let resource = ResourceData::from_binary(&name, data.as_ref())
                    .with_context(|| format!("Failed to parse resource {} in SARC", canon))?;
                if is_mergeable_sarc(canon.as_str(), data.as_ref()) {
                    self.process_sarc(Sarc::new(data.as_ref())?, &name)?;
                }
                self.cache.insert(canon.clone(), Arc::new(resource));
            }
            // if !self.nest_map.contains_key(&canon) {
            //     self.nest_map.insert(canon, sarc_path.into());
            // }
        }
        Ok(())
    }

    pub fn languages(
        &self,
    ) -> dashmap::mapref::one::RefMut<
        '_,
        std::path::PathBuf,
        std::vec::Vec<uk_content::constants::Language>,
    > {
        static LANGS: Lazy<DashMap<PathBuf, Vec<Language>>> = Lazy::new(Default::default);
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
