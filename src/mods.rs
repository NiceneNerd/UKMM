use crate::{
    settings::Settings,
    util::{self, HashMap},
};
use anyhow::{Context, Result};
use fs_err as fs;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{
    hash::{Hash, Hasher},
    io::{BufReader, Read},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};
use uk_mod::{pack::ModPacker, unpack::ModReader, Manifest, Meta, ModOption};

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Mod {
    pub meta: Meta,
    pub manifest: Manifest,
    pub enabled_options: Vec<ModOption>,
    pub enabled: bool,
    pub path: PathBuf,
    #[serde_as(as = "DisplayFromStr")]
    pub(crate) hash: usize,
}

impl std::fmt::Debug for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mod")
            .field("meta", &self.meta)
            .field("enabled_options", &self.enabled_options)
            .field("enabled", &self.enabled)
            .field("path", &self.path)
            .field("hash", &self.hash)
            .finish()
    }
}

impl std::hash::Hash for Mod {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.hash)
    }
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
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

    pub fn preview(&self) -> Result<Option<Vec<u8>>> {
        let mut zip = zip::ZipArchive::new(BufReader::new(std::fs::File::open(&self.path)?))?;
        if let Ok(mut file) = zip.by_name("thumb.jpg") {
            let mut vec = vec![0; file.size() as usize];
            file.read_exact(&mut vec)?;
            return Ok(Some(vec));
        }
        if let Ok(mut file) = zip.by_name("thumb.png") {
            let mut vec = vec![0; file.size() as usize];
            file.read_exact(&mut vec)?;
            return Ok(Some(vec));
        }
        Ok(None)
    }

    pub fn manifest_with_options(&self, options: impl AsRef<[ModOption]>) -> Result<Manifest> {
        ModReader::open(&self.path, options.as_ref()).map(|r| r.manifest)
    }

    pub fn state_eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled && self.enabled_options == other.enabled_options
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

impl LookupMod for &Mod {
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
    manager: &'a Manager,
    index: usize,
}

impl<'a> Iterator for ModIterator<'a> {
    type Item = MappedRwLockReadGuard<'a, Mod>;
    fn next(&mut self) -> Option<Self::Item> {
        let mods = self.manager.mods.read();
        let loads = self.manager.load_order.read();
        if self.index < loads.len() {
            let hash = loads[self.index];
            self.index += 1;
            Some(RwLockReadGuard::map(mods, |m| &m[&hash]))
        } else {
            None
        }
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
        log::info!("Initializing mod manager");
        if !mod_dir.exists() {
            fs::create_dir_all(mod_dir)?;
            log::info!("Created mod directory at {}", mod_dir.display());
            let self_ = Self {
                dir: mod_dir.to_path_buf(),
                mods: Default::default(),
                load_order: Default::default(),
                settings: Arc::downgrade(settings),
            };
            self_.save()?;
            return Ok(self_);
        }
        let mods = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("mods.yml"))?)
            .context("Failed to parse mod database")?;
        log::debug!("Mods:\n{:?}", &mods);
        let load_order = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("load.yml"))?)
            .context("Failed to parse load order table")?;
        log::debug!("Load order:\n{:?}", &load_order);
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
            self.dir.join("load.yml"),
            serde_yaml::to_string(self.load_order.read().deref())?,
        )?;
        log::info!("Saved load order");
        log::debug!("{:?}", &self.load_order);
        Ok(())
    }

    pub fn init(settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        Self::init_from(&settings.read().mods_dir(), settings)
    }

    /// Iterate all mods, including disabled, in load order.
    pub fn all_mods(&self) -> ModIterator<'_> {
        ModIterator {
            index: 0,
            manager: self,
        }
    }

    /// Iterate all enabled mods in load order.
    pub fn mods(&self) -> impl Iterator<Item = MappedRwLockReadGuard<'_, Mod>> {
        ModIterator {
            index: 0,
            manager: self,
        }
        .filter(|m| m.enabled)
    }

    /// Iterate all mods which modify any files in the given manifest.
    pub fn mods_by_manifest<'a: 'm, 'm>(
        &'a self,
        manifest: &'m Manifest,
    ) -> impl Iterator<Item = MappedRwLockReadGuard<'_, Mod>> + 'm {
        self.mods().filter(|mod_| {
            !mod_
                .manifest
                .content_files
                .is_disjoint(&manifest.content_files)
                || !mod_.manifest.aoc_files.is_disjoint(&manifest.aoc_files)
        })
    }

    /// Add a mod to the list of installed mods. This function assumes that the
    /// mod at the provided path has already been validated.
    pub fn add<'a, 'b>(&'a self, mod_path: &'b Path) -> Result<&'a Mod> {
        let mut san_opts = sanitise_file_name::Options::DEFAULT;
        san_opts.url_safe = true;
        let sanitized = sanitise_file_name::sanitise_with_options(
            mod_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim_start_matches('.'),
            &san_opts,
        );
        let stored_path = self.dir.join(sanitized);
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
        {
            self.load_order.write().push(mod_.hash);
        }
        // Convince the compiler that this does not leave us with an outstanding mutable reference
        let mod_: &Mod =
            unsafe { &*(self.mods.write().entry(mod_.hash).or_insert(mod_) as *const _) };
        log::info!("Added mod {}", mod_.meta.name);
        log::debug!("{:?}", mod_);
        Ok(mod_)
    }

    pub fn del(&self, mod_: impl LookupMod) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        let mod_ = self.mods.write().remove(&hash);
        if let Some(mod_) = mod_ {
            let manifest = mod_.manifest;
            if mod_.path.is_dir() {
                util::remove_dir_all(&mod_.path)?;
            } else {
                fs::remove_file(&mod_.path)?;
            }
            self.load_order.try_write().unwrap().retain(|m| m != &hash);
            log::info!("Deleted mod {}", mod_.meta.name);
            Ok(manifest)
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            Ok(Default::default())
        }
    }

    pub fn set_enabled(&self, mod_: impl LookupMod, enabled: bool) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.mods.write().get_mut(&hash) {
            mod_.enabled = enabled;
            manifest = mod_.manifest.clone();
            log::info!(
                "{} mod {}",
                if enabled { "Enabled" } else { "Disabled" },
                mod_.meta.name
            );
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        Ok(manifest)
    }

    pub fn set_enabled_options(
        &self,
        mod_: impl LookupMod,
        options: Vec<ModOption>,
    ) -> Result<Manifest> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.mods.write().get_mut(&hash) {
            manifest = mod_.manifest_with_options(&options)?;
            mod_.enabled_options = options;
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        Ok(manifest)
    }

    pub fn set_order(&self, order: Vec<usize>) {
        *self.load_order.write() = order;
    }

    pub fn get_mod(&self, hash: usize) -> Option<MappedRwLockReadGuard<'_, Mod>> {
        if !self.mods.read().contains_key(&hash) {
            None
        } else {
            Some(RwLockReadGuard::map(self.mods.read(), |mods| &mods[&hash]))
        }
    }
}

pub fn convert_gfx(path: &Path) -> Result<PathBuf> {
    log::info!("Attempting to convert mod at {}", path.display());
    let path = if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_uppercase();

        let find_rules = |path: &Path| -> Option<PathBuf> {
            jwalk::WalkDir::new(path)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .find_map(|f| {
                    (f.file_name().to_str() == Some("rules.txt")).then(|| f.parent_path().into())
                })
        };

        if ext == "ZIP" {
            log::info!("Extracting ZIP file...");
            let tmpdir = tempfile::tempdir()?.into_path();
            zip::ZipArchive::new(BufReader::new(fs::File::open(path)?))
                .context("Failed to open ZIP")?
                .extract(&tmpdir)
                .context("Failed to extract ZIP")?;
            find_rules(&tmpdir).context("Could not find rules.txt in extracted")?
        } else if ext == "7Z" {
            log::info!("Extracting 7Z file...");
            let tmpdir = tempfile::tempdir()?.into_path();
            sevenz_rust::decompress_file(path, &tmpdir).context("Failed to extract 7Z file")?;
            find_rules(&tmpdir).context("Could not find rules.txt in extracted")?
        } else if path.file_name().context("No file name")?.to_str() == Some("rules.txt") {
            path.parent().unwrap().to_owned()
        } else {
            log::error!("{} is not a supported mod archive", path.display());
            anyhow::bail!("{} files are not supported", ext)
        }
    } else {
        log::info!("Unpacked mod, that's easy");
        path.to_path_buf()
    };
    let temp = tempfile::tempdir()?.into_path();
    log::debug!("Temp folder: {}", temp.display());
    log::info!("Attempting to convert mod...");
    let packer = ModPacker::new(&path, &temp, None, vec![])?;
    let result_path = packer.pack()?;
    log::info!("Conversion complete");
    Ok(result_path)
}
