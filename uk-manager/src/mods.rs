use std::{
    hash::{Hash, Hasher},
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use anyhow::{Context, Result};
use fs_err as fs;
use once_cell::sync::Lazy;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uk_mod::{pack::ModPacker, unpack::ModReader, Manifest, Meta, ModOption};

use crate::{
    settings::Settings,
    util::{self, extract_7z, HashMap},
};

type ManifestCache = Lazy<RwLock<HashMap<(usize, Vec<PathBuf>), Result<Arc<Manifest>>>>>;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Mod {
    pub meta: Meta,
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
            enabled_options: vec![],
            path: reader.path,
            enabled: false,
        }
    }

    pub fn manifest(&self) -> Result<Arc<Manifest>> {
        self.manifest_with_options(&self.enabled_options)
    }

    pub fn manifest_with_options(&self, options: impl AsRef<[ModOption]>) -> Result<Arc<Manifest>> {
        static MANIFEST_CACHE: ManifestCache = Lazy::new(|| RwLock::new(HashMap::default()));
        match MANIFEST_CACHE
            .write()
            .entry((
                self.hash,
                options.as_ref().iter().map(|o| o.path.clone()).collect(),
            ))
            .or_insert_with(|| {
                ModReader::open(&self.path, options.as_ref()).map(|r| Arc::new(r.manifest))
            }) {
            Ok(manifest) => Ok(manifest.clone()),
            Err(e) => Err(anyhow::format_err!("{:?}", e)),
        }
    }

    pub fn state_eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled && self.enabled_options == other.enabled_options
    }

    #[inline(always)]
    pub fn hash(&self) -> usize {
        self.hash
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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Profile {
    mods: RwLock<HashMap<usize, Mod>>,
    load_order: RwLock<Vec<usize>>,
}

impl Clone for Profile {
    fn clone(&self) -> Self {
        Self {
            mods: RwLock::new(self.mods.read().clone()),
            load_order: RwLock::new(self.load_order.read().clone()),
        }
    }
}

impl Profile {
    pub fn mods(&self) -> RwLockReadGuard<HashMap<usize, Mod>> {
        self.mods.read()
    }

    pub fn mods_mut(&self) -> RwLockWriteGuard<HashMap<usize, Mod>> {
        self.mods.write()
    }

    #[allow(unused)]
    pub fn load_order(&self) -> RwLockReadGuard<Vec<usize>> {
        self.load_order.read()
    }

    pub fn load_order_mut(&self) -> RwLockWriteGuard<Vec<usize>> {
        self.load_order.write()
    }
}

pub struct ModIterator<'a> {
    profile: &'a Profile,
    index:   usize,
}

impl<'a> Iterator for ModIterator<'a> {
    type Item = Mod;

    fn next(&mut self) -> Option<Self::Item> {
        let mods = self.profile.mods.read();
        let loads = self.profile.load_order.read();
        if self.index < loads.len() {
            let hash = loads[self.index];
            self.index += 1;
            Some(mods[&hash].clone())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Manager {
    path:     PathBuf,
    profile:  Profile,
    settings: Weak<RwLock<Settings>>,
}

impl Manager {
    pub fn open_profile(path: &Path, settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        log::info!(
            "Initializing mod manager for profile {}",
            path.file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
        );
        if !path.exists() {
            log::info!("Creating profile at {}", path.display());
            fs::create_dir_all(path)?;
            let self_ = Self {
                path:     path.to_path_buf(),
                profile:  Default::default(),
                settings: Arc::downgrade(settings),
            };
            self_.save()?;
            return Ok(self_);
        }
        let profile = serde_yaml::from_str(&fs::read_to_string(path.join("profile.yml"))?)
            .context("Failed to parse mod database")?;
        log::debug!("Profile data:\n{:#?}", &profile);
        Ok(Self {
            path: path.to_path_buf(),
            profile,
            settings: Arc::downgrade(settings),
        })
    }

    pub fn save(&self) -> Result<()> {
        fs::write(
            self.path.join("profile.yml"),
            serde_yaml::to_string(&self.profile)?,
        )?;
        log::info!("Saved profile data");
        log::debug!("{:#?}", &self.profile);
        Ok(())
    }

    pub fn open_current_profile(settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        Self::open_profile(&settings.read().profile_dir(), settings)
    }

    /// Iterate all mods, including disabled, in load order.
    pub fn all_mods(&self) -> ModIterator<'_> {
        ModIterator {
            index:   0,
            profile: &self.profile,
        }
    }

    /// Iterate all enabled mods in load order.
    pub fn mods(&self) -> impl Iterator<Item = Mod> + '_ {
        ModIterator {
            index:   0,
            profile: &self.profile,
        }
        .filter(|m| m.enabled)
    }

    /// Iterate all mods which modify any files in the given manifest.
    pub fn mods_by_manifest<'a: 'm, 'm>(
        &'a self,
        ref_manifest: &'m Manifest,
    ) -> impl Iterator<Item = Mod> + 'm {
        self.mods().filter(|mod_| {
            match mod_.manifest() {
                Ok(manifest) => {
                    !ref_manifest
                        .content_files
                        .is_disjoint(&manifest.content_files)
                        || !ref_manifest.aoc_files.is_disjoint(&manifest.aoc_files)
                }
                Err(_) => false,
            }
        })
    }

    /// Add a mod to the list of installed mods. This function assumes that the
    /// mod at the provided path has already been validated.
    #[allow(irrefutable_let_patterns)]
    pub fn add(&self, mod_path: &Path) -> Result<Mod> {
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
        let stored_path = self
            .settings
            .upgrade()
            .unwrap()
            .read()
            .mods_dir()
            .join(sanitized);
        if let parent = stored_path.parent().unwrap() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
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
        let reader = ModReader::open_peek(&stored_path, vec![])?;
        let mod_ = Mod::from_reader(reader);
        self.profile.load_order_mut().push(mod_.hash);
        self.profile.mods_mut().insert(mod_.hash, mod_.clone());
        log::info!("Added mod {}", mod_.meta.name);
        log::debug!("{:#?}", mod_);
        Ok(mod_)
    }

    pub fn del(&self, mod_: impl LookupMod) -> Result<Arc<Manifest>> {
        let hash = mod_.as_hash_id();
        let mod_ = self.profile.mods_mut().remove(&hash);
        if let Some(mod_) = mod_ {
            let manifest = mod_.manifest()?;
            if mod_.path.is_dir() {
                util::remove_dir_all(&mod_.path)?;
            } else {
                fs::remove_file(&mod_.path)?;
            }
            self.profile.load_order_mut().retain(|m| m != &hash);
            log::info!("Deleted mod {}", mod_.meta.name);
            Ok(manifest)
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            Ok(Default::default())
        }
    }

    pub fn set_enabled(&self, mod_: impl LookupMod, enabled: bool) -> Result<Arc<Manifest>> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.profile.mods_mut().get_mut(&hash) {
            mod_.enabled = enabled;
            manifest = mod_.manifest()?;
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
    ) -> Result<Arc<Manifest>> {
        let hash = mod_.as_hash_id();
        let manifest;
        if let Some(mod_) = self.profile.mods_mut().get_mut(&hash) {
            manifest = mod_.manifest_with_options(&options)?;
            mod_.enabled_options = options;
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        Ok(manifest)
    }

    pub fn set_order(&self, order: Vec<usize>) {
        *self.profile.load_order_mut() = order;
    }

    pub fn get_mod(&self, hash: usize) -> Option<Mod> {
        self.profile.mods().get(&hash).cloned()
    }
}

pub fn convert_gfx(
    core: &crate::core::Manager,
    path: &Path,
    meta: Option<Meta>,
) -> Result<PathBuf> {
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
                    ([Some("rules.txt"), Some("info.json")].contains(&f.file_name().to_str()))
                        .then(|| f.parent_path().into())
                })
        };

        if ext == "ZIP" {
            log::info!("Extracting ZIP file...");
            let tmpdir = util::get_temp_folder();
            zip::ZipArchive::new(BufReader::new(fs::File::open(path)?))
                .context("Failed to open ZIP")?
                .extract(&*tmpdir)
                .context("Failed to extract ZIP")?;
            if meta.is_none() {
                find_rules(&tmpdir).context("Could not find rules.txt in extracted mod")?
            } else {
                tmpdir.to_path_buf()
            }
        } else if ext == "7Z" {
            log::info!("Extracting 7Z file...");
            let tmpdir = util::get_temp_folder();
            extract_7z(path, &tmpdir).context("Failed to extract 7Z file")?;
            if meta.is_none() {
                find_rules(&tmpdir).context("Could not find rules.txt in extracted mod")?
            } else {
                tmpdir.to_path_buf()
            }
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
    let temp = util::get_temp_folder();
    log::debug!("Temp folder: {}", temp.display());
    log::info!("Attempting to convert mod...");
    let packer = ModPacker::new(path, &*temp, meta, vec![
        core.settings()
            .dump()
            .context("No dump available for current platform")?,
    ])?;
    let result_path = packer.pack()?;
    log::info!("Conversion complete");
    Ok(result_path)
}
