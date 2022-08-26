use crate::{
    settings::Settings,
    util::{self, HashMap},
};
use anyhow::{Context, Result};
use fs_err as fs;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};
use uk_mod::{unpack::ModReader, Manifest, Meta, ModOption};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub meta: Meta,
    pub manifest: Manifest,
    pub enabled_options: Vec<ModOption>,
    pub enabled: bool,
    pub path: PathBuf,
    hash: usize,
}

impl std::hash::Hash for Mod {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.hash)
    }
}

impl Mod {
    pub fn from_reader(reader: ModReader) -> Self {
        let mut hasher = rustc_hash::FxHasher::default();
        reader.meta.hash(&mut hasher);
        Self {
            hash: hasher.finish() as usize,
            meta: reader.meta,
            manifest: reader.manifest,
            enabled_options: vec![],
            path: reader.path,
            enabled: false,
        }
    }
}

pub trait LookupMod {
    fn as_hash_id(&self) -> usize;
}

impl LookupMod for Mod {
    #[inline(always)]
    fn as_hash_id(&self) -> usize {
        self.hash
    }
}

impl LookupMod for usize {
    #[inline(always)]
    fn as_hash_id(&self) -> usize {
        *self
    }
}

pub struct ModIterator<'a> {
    load_order: MappedRwLockReadGuard<'a, std::slice::Iter<'static, usize>>,
    mods: RwLockReadGuard<'a, HashMap<usize, Mod>>,
    filter: bool,
}

impl<'a> Iterator for ModIterator<'a> {
    type Item = &'a Mod;
    fn next(&mut self) -> Option<Self::Item> {
        self.load_order
            .next()
            .map(|h| &self.mods[h])
            .filter(|m| !self.filter || m.enabled)
    }
}

#[derive(Debug)]
pub struct Manager {
    dir: PathBuf,
    mods: RwLock<HashMap<usize, Mod>>,
    load_order: RwLock<Vec<usize>>,
    settings: Weak<RwLock<Settings>>,
}

impl Manager {
    pub fn init_from(mod_dir: &Path, settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        if !mod_dir.exists() {
            fs::create_dir_all(&mod_dir)?;
            log::info!("Created mod directory at {}", mod_dir.display());
            return Ok(Self {
                dir: mod_dir.to_path_buf(),
                mods: Default::default(),
                load_order: Default::default(),
                settings: Arc::downgrade(settings),
            });
        }
        let mods = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("mods.yml"))?)
            .context("Failed to parse mod database")?;
        log::debug!("{:?}", &mods);
        let load_order = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("load.yml"))?)
            .context("Failed to parse load order table")?;
        log::debug!("{:?}", &load_order);
        Ok(Self {
            dir: mod_dir.to_path_buf(),
            mods: RwLock::new(mods),
            load_order: RwLock::new(load_order),
            settings: Arc::downgrade(settings),
        })
    }

    pub fn save(&self) -> Result<()> {
        fs::write(
            self.dir.join("mods.yml"),
            serde_yaml::to_string(self.mods.read().deref())?,
        )?;
        log::info!("Saved mod list");
        log::debug!("{:?}", &self.mods);
        fs::write(
            self.dir.join("loads.yml"),
            serde_yaml::to_string(self.load_order.read().deref())?,
        )?;
        log::info!("Saved load order");
        log::debug!("{:?}", &self.load_order);
        Ok(())
    }

    pub fn init(settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        Self::init_from(&settings.read().mods_dir(), settings)
    }

    /// Iterate all enabled mods in load order.
    pub fn mods(&self) -> ModIterator<'_> {
        ModIterator {
            load_order: RwLockReadGuard::map(self.load_order.read(), |l| &l.iter()),
            mods: self.mods.read(),
            filter: true,
        }
    }

    /// Iterate all mods, including disabled, in load order.
    pub fn all_mods(&self) -> ModIterator<'_> {
        ModIterator {
            load_order: RwLockReadGuard::map(self.load_order.read(), |l| &l.iter()),
            mods: self.mods.read(),
            filter: false,
        }
    }

    /// Iterate all mods which modify any files in the given manifest.
    pub fn mods_by_manifest<'a: 'm, 'm>(
        &'a self,
        manifest: &'m Manifest,
    ) -> impl Iterator<Item = &'a Mod> + 'm {
        self.load_order.iter().filter_map(|h| {
            let mod_ = &self.mods[h];
            if !mod_
                .manifest
                .content_files
                .is_disjoint(&manifest.content_files)
                || !mod_.manifest.aoc_files.is_disjoint(&manifest.aoc_files)
            {
                Some(mod_)
            } else {
                None
            }
        })
    }

    /// Add a mod to the list of installed mods. This function assumes that the
    /// mod at the provided path has already been validated.
    pub fn add<'a, 'b>(&'a self, mod_path: &'b Path) -> Result<&'a Mod> {
        let stored_path = self.dir.join(mod_path.file_name().unwrap());
        if mod_path.is_file() {
            if self.settings.upgrade().unwrap().read().unpack_mods {
                uk_mod::unpack::unzip_mod(mod_path, &stored_path)
                    .context("Failed to unpack mod to storage folder")?;
            } else {
                fs::copy(mod_path, &stored_path).context("Failed to copy mod to storage folder")?;
            }
        } else {
            dircpy::copy_dir(mod_path, &stored_path)
                .context("Failed to copy mod to storage folder")?;
        }
        let reader = ModReader::open(&stored_path, vec![])?;
        let mod_ = Mod::from_reader(reader);
        self.load_order.write().push(mod_.hash);
        // Convince the compiler that this does not leave us with an outstanding mutable reference
        let mod_: &Mod =
            unsafe { &*(self.mods.write().entry(mod_.hash).or_insert(mod_) as *const _) };
        self.save()?;
        log::info!("Added mod {}", mod_.meta.name);
        log::debug!("{:?}", mod_);
        Ok(mod_)
    }

    pub fn del(&self, mod_: impl LookupMod) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        if let Some(mod_) = self.mods.write().remove(&hash) {
            let manifest = mod_.manifest;
            if mod_.path.is_dir() {
                util::remove_dir_all(&mod_.path)?;
            } else {
                fs::remove_file(&mod_.path)?;
            }
            self.load_order.write().retain(|m| m != &hash);
            self.save()?;
            log::info!("Deleted mod {}", mod_.meta.name);
            Ok(manifest)
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            Ok(Default::default())
        }
    }

    pub fn disable(&self, mod_: impl LookupMod) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.mods.write().get_mut(&hash) {
            mod_.enabled = false;
            manifest = mod_.manifest.clone();
            log::info!("Disabled mod {}", mod_.meta.name);
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        self.save()?;
        Ok(manifest)
    }

    pub fn enable(&self, mod_: impl LookupMod) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.mods.write().get_mut(&hash) {
            mod_.enabled = true;
            manifest = mod_.manifest.clone();
            log::info!("Enabled mod {}", mod_.meta.name);
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        self.save()?;
        Ok(manifest)
    }
}
