use std::{
    hash::{Hash, Hasher},
    io::BufReader,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Weak},
};

use anyhow_ext::{Context, Result};
use dashmap::{mapref::one::MappedRef, DashMap};
use fs_err as fs;
use lenient_semver::Version;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use sanitise_file_name as sfn;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use smartstring::alias::String;
use uk_content::platform_prefixes;
use uk_mod::{pack::ModPacker, unpack::ModReader, Manifest, Meta, ModOption};

use crate::{
    settings::Settings,
    util::{self, extract_7z, HashMap},
};

type ManifestCache = LazyLock<RwLock<HashMap<(usize, Vec<PathBuf>), Result<Arc<Manifest>>>>>;

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
        static MANIFEST_CACHE: ManifestCache = LazyLock::new(|| RwLock::new(HashMap::default()));
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

    pub fn enable_default_options(&mut self) {
        if !self.meta.options.is_empty() {
            for group in self.meta.options.iter_mut() {
                match group {
                    uk_mod::OptionGroup::Exclusive(group) => {
                        self.enabled_options
                            .extend(group.default.iter().filter_map(|path| {
                                group.options.iter().find(|o| &o.path == path).cloned()
                            }));
                    }
                    uk_mod::OptionGroup::Multiple(group) => {
                        self.enabled_options
                            .extend(group.defaults.iter().filter_map(|path| {
                                group.options.iter().find(|o| &o.path == path).cloned()
                            }));
                    }
                };
            }
        }
    }
}

pub trait LookupMod {
    fn as_map_id(&self) -> usize;
}

impl LookupMod for Mod {
    #[inline(always)]
    fn as_map_id(&self) -> usize {
        self.hash
    }
}

impl LookupMod for &Mod {
    #[inline(always)]
    fn as_map_id(&self) -> usize {
        self.hash
    }
}

impl LookupMod for usize {
    #[inline(always)]
    fn as_map_id(&self) -> usize {
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

    pub fn iter(self_: MappedRef<'_, String, Profile, Profile>) -> ModIterator<'_> {
        ModIterator {
            profile: self_,
            index:   0,
        }
    }
}

pub struct ModIterator<'a> {
    profile: MappedRef<'a, String, Profile, Profile>,
    index:   usize,
}

impl<'a> Iterator for ModIterator<'a> {
    type Item = Mod;

    fn next(&mut self) -> Option<Self::Item> {
        let loads = self.profile.load_order();
        let mods = self.profile.mods();
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
    dir: PathBuf,
    profiles: DashMap<String, Profile>,
    current_profile: String,
    settings: Weak<RwLock<Settings>>,
}

impl Manager {
    #[inline(always)]
    pub fn path(&self) -> PathBuf {
        self.dir.join(self.current_profile.as_str())
    }

    #[inline(always)]
    pub fn profile(&self) -> MappedRef<'_, String, Profile, Profile> {
        self.profiles
            .get(self.current_profile.as_str())
            .expect("Invalid profile")
            .map(|f| f)
    }

    #[inline(always)]
    pub fn get_profile(&self, profile: Option<&String>) -> MappedRef<'_, String, Profile, Profile> {
        let profile = profile.unwrap_or(&self.current_profile);
        self.profiles
            .get(profile.as_str())
            .expect("Invalid profile")
            .map(|f| f)
    }

    pub fn create_profile_if(&self, profile: &str) -> Result<()> {
        let path = self.dir.join(profile);
        if !path.exists() {
            log::info!("Profile {profile} does not exist, creating it now");
            fs::create_dir_all(path)?;
            self.profiles.insert(profile.into(), Default::default());
            self.save()?;
        }
        Ok(())
    }

    pub fn set_profile(&mut self, profile: &str) -> Result<()> {
        self.current_profile = profile.into();
        self.create_profile_if(profile)?;
        Ok(())
    }

    pub fn init(settings: &Arc<RwLock<Settings>>) -> Result<Self> {
        log::info!("Initializing mod manager");
        let current_profile = settings
            .read()
            .platform_config()
            .as_ref()
            .map(|c| c.profile.clone())
            .unwrap_or_else(|| "Default".into());
        log::info!("Current profile: {}", current_profile);
        let path = settings.read().profiles_dir();
        let profiles = settings
            .read()
            .profiles()
            .map(|profile| {
                let profile_path = path.join(profile.as_str()).join("profile.yml");
                fs::read_to_string(profile_path)
                    .context("Failed to read profile data")
                    .and_then(|t| serde_yaml::from_str(&t).context("Failed to parse profile data"))
                    .map(|v| (profile, v))
            })
            .collect::<Result<_>>()?;
        let self_ = Self {
            dir: path,
            profiles,
            current_profile: current_profile.clone(),
            settings: Arc::downgrade(settings),
        };
        self_.create_profile_if(&current_profile)?;
        Ok(self_)
    }

    pub fn save(&self) -> Result<()> {
        fs::write(
            self.path().join("profile.yml"),
            serde_yaml::to_string(self.profile().deref())?,
        )?;
        log::info!("Saved profile data");
        log::debug!("{:#?}", &self.profile());
        Ok(())
    }

    /// Iterate all mods, including disabled, in load order.
    pub fn all_mods(&self) -> ModIterator<'_> {
        Profile::iter(self.profile())
    }

    /// Iterate all enabled mods in load order.
    pub fn mods(&self) -> impl Iterator<Item = Mod> + '_ {
        self.all_mods().filter(|m| m.enabled)
    }

    /// Iterate all mods which modify any files in the given manifest.
    pub fn mods_by_manifest<'a: 'm, 'm>(
        &'a self,
        ref_manifest: &'m Manifest,
    ) -> impl Iterator<Item = Mod> + 'm {
        let ref_languages = ref_manifest.languages();
        self.mods().filter(move |mod_| {
            match mod_.manifest() {
                Ok(manifest) => {
                    let languages = manifest.languages();
                    !ref_manifest
                        .content_files
                        .is_disjoint(&manifest.content_files)
                        || !ref_manifest.aoc_files.is_disjoint(&manifest.aoc_files)
                        || (ref_languages.iter().any(|l| languages.contains(l)))
                }
                Err(_) => false,
            }
        })
    }

    /// Add a mod to the list of installed mods. This function assumes that the
    /// mod at the provided path has already been validated.
    pub fn add(&self, mod_path: &Path, profile: Option<&String>) -> Result<Mod> {
        let mut old_version = None;
        let mod_name = {
            let peeker = ModReader::open_peek(mod_path, vec![])?;
            let name = peeker.meta.name.as_str();
            if let Some(mod_) =
                Profile::iter(self.get_profile(profile)).find(|m| m.meta.name == name)
            {
                if Version::parse(peeker.meta.version.as_str())
                    .and_then(|pv| Version::parse(mod_.meta.version.as_str()).map(|mv| pv > mv))
                    .map_err(|e| anyhow_ext::anyhow!("{e}"))
                    .context("Bad version strings")?
                {
                    log::info!("Updating {name} to version {}", peeker.meta.version);
                    old_version = Some(mod_);
                } else {
                    anyhow_ext::bail!("Mod \"{}\" already installed", peeker.meta.name);
                }
            }
            peeker.meta.name
        };
        let san_opts: sfn::Options<Option<char>> = sfn::Options {
            url_safe: true,
            collapse_replacements: true,
            ..Default::default()
        };
        let sanitized = sfn::sanitise_with_options(&mod_name, &san_opts);
        let stored_path = self
            .settings
            .upgrade()
            .expect("Settings is GONE!")
            .read()
            .mods_dir()
            .join(sanitized + ".zip");
        if stored_path.exists() && old_version.is_none() {
            log::debug!("Mod already stored, no need to store it");
        } else {
            stored_path.parent().map(fs::create_dir_all).transpose()?;
            if mod_path.is_file() {
                fs::copy(mod_path, &stored_path).context("Failed to copy mod to storage folder")?;
            } else {
                util::copy_dir(mod_path, &stored_path)
                    .context("Failed to copy mod to storage folder")?;
            }
        }
        let reader = ModReader::open_peek(&stored_path, vec![])?;
        let mut mod_ = Mod::from_reader(reader);
        mod_.enabled = true;
        let profile_data = self.get_profile(profile);
        profile_data.load_order_mut().push(mod_.hash);
        profile_data.mods_mut().insert(mod_.hash, mod_.clone());
        if let Some(old_mod) = old_version {
            profile_data.load_order_mut().retain(|h| *h != old_mod.hash);
            profile_data.mods_mut().remove(&old_mod.hash);
            log::info!(
                "Updated mod {} in profile {} to version {}",
                mod_.meta.name,
                profile.unwrap_or(&self.current_profile).as_str(),
                mod_.meta.version
            );
        } else {
            log::info!(
                "Added mod {} to profile {}",
                mod_.meta.name,
                profile.unwrap_or(&self.current_profile).as_str()
            );
        }
        log::debug!("{:#?}", mod_);
        Ok(mod_)
    }

    pub fn del(&self, mod_: impl LookupMod, profile: Option<&String>) -> Result<Arc<Manifest>> {
        let hash = mod_.as_map_id();
        let profile_data = self.get_profile(profile);
        let mod_ = profile_data.mods_mut().remove(&hash);
        if let Some(mod_) = mod_ {
            let manifest = mod_.manifest()?;
            // Only delete the mod file if no other profiles are using it
            if !self
                .profiles
                .iter()
                .any(|p| p.value().mods().contains_key(&hash))
            {
                if mod_.path.is_dir() {
                    util::remove_dir_all(&mod_.path)?;
                } else {
                    fs::remove_file(&mod_.path)?;
                }
            }
            profile_data.load_order_mut().retain(|m| m != &hash);
            log::info!(
                "Deleted mod {} from profile {}",
                mod_.meta.name,
                profile.unwrap_or(&self.current_profile).as_str()
            );
            Ok(manifest)
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            Ok(Default::default())
        }
    }

    pub fn replace(&self, mut mod_: Mod, old_hash: usize) -> Result<Mod> {
        let profile_data = self.profile();
        let old_mod = profile_data
            .mods_mut()
            .remove(&old_hash)
            .expect("The old mod has to be in the profile");
        mod_.enabled = old_mod.enabled;
        mod_.path = old_mod.path;
        profile_data.mods_mut().insert(mod_.hash, mod_.clone());
        let mut load_order = profile_data.load_order_mut();
        if let Some(idx) = load_order.iter().position(|m| *m == old_hash) {
            load_order[idx] = mod_.hash;
        }
        Ok(mod_)
    }

    pub fn set_enabled(
        &self,
        mod_: impl LookupMod,
        enabled: bool,
        profile: Option<&String>,
    ) -> Result<Arc<Manifest>> {
        let hash = mod_.as_map_id();
        let manifest;
        let profile_data = self.get_profile(profile);
        if let Some(mod_) = profile_data.mods_mut().get_mut(&hash) {
            mod_.enabled = enabled;
            manifest = mod_.manifest()?;
            log::info!(
                "{} mod {} in profile {}",
                if enabled { "Enabled" } else { "Disabled" },
                mod_.meta.name,
                profile.unwrap_or(&self.current_profile).as_str()
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
        let hash = mod_.as_map_id();
        let manifest;
        if let Some(mod_) = self.profile().mods_mut().get_mut(&hash) {
            manifest = mod_.manifest_with_options(&options)?;
            mod_.enabled_options = options;
        } else {
            log::warn!("Mod with ID {} does not exist, doing nothing", hash);
            return Ok(Default::default());
        }
        Ok(manifest)
    }

    pub fn set_order(&self, order: Vec<usize>) {
        *self.profile().load_order_mut() = order;
    }

    pub fn get_mod(&self, hash: usize) -> Option<Mod> {
        self.profile().mods().get(&hash).cloned()
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

        let find_root = |path: &Path| -> Option<PathBuf> {
            let (content, dlc) = platform_prefixes(core.settings().current_mode.into());
            jwalk::WalkDir::new(path)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .find_map(|f| {
                    (f.path().join(content).exists() || f.path().join(dlc).exists())
                        .then(|| f.path())
                        .or_else(|| {
                            ([Some(content), Some(dlc)].contains(&f.file_name().to_str()))
                                .then(|| f.parent_path().into())
                        })
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
                find_root(&tmpdir)
                    .context("Could not find base or DLC content folder in extracted mod")?
            }
        } else if ext == "7Z" {
            log::info!("Extracting 7Z file...");
            let tmpdir = util::get_temp_folder();
            extract_7z(path, &tmpdir).context("Failed to extract 7Z file")?;
            if meta.is_none() {
                find_rules(&tmpdir).context("Could not find rules.txt in extracted mod")?
            } else {
                find_root(&tmpdir)
                    .context("Could not find base or DLC content folder in extracted mod")?
            }
        } else if path.file_name().context("No file name")?.to_str() == Some("rules.txt") {
            path.parent()
                .expect("Parent path gotta' exist, right?")
                .to_owned()
        } else {
            log::error!("{} is not a supported mod archive", path.display());
            anyhow_ext::bail!("{} files are not supported", ext)
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[test]
fn san_test() {
    let mut san_opts = sanitise_file_name::Options::DEFAULT;
    san_opts.url_safe = true;
    let sanitized = sanitise_file_name::sanitise_with_options(
        Path::new("mod8378&$*#*FIDIFKHLGF*&#KFDJK2020..+=.zip")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_start_matches('.'),
        &san_opts,
    );
    dbg!(sanitized);
}
