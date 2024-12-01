use std::{
    collections::{BTreeSet, HashSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc, LazyLock},
};

use anyhow_ext::{Context, Result};
use botw_utils::hashes::StockHashTable;
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use parking_lot::Mutex;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::Sarc, yaz0::decompress_if};
pub use sanitise_file_name::sanitise;
use serde::Deserialize;
use serde_with::{serde_as, DefaultOnError};
use smartstring::alias::String;
use uk_content::{
    canonicalize,
    constants::Language,
    platform_prefixes,
    prelude::{Endian, Mergeable},
    resource::{is_mergeable_sarc, ResourceData},
};
use uk_util::PathExt as UkPathExt;
use zip::{
    write::{FileOptions, SimpleFileOptions},
    ZipWriter as ZipW,
};

use crate::{
    ExclusiveOptionGroup, Manifest, Meta, ModOption, ModOptionGroup, ModPlatform,
    MultipleOptionGroup, OptionGroup,
};

pub type ZipWriter = Arc<Mutex<ZipW<fs::File>>>;

static NX_HASH_TABLE: LazyLock<StockHashTable> =
    LazyLock::new(|| StockHashTable::new(&botw_utils::hashes::Platform::Switch));
static WIIU_HASH_TABLE: LazyLock<StockHashTable> =
    LazyLock::new(|| StockHashTable::new(&botw_utils::hashes::Platform::WiiU));

pub struct ModPacker {
    source_dir: PathBuf,
    current_root: PathBuf,
    meta: Meta,
    zip: ZipWriter,
    endian: Endian,
    built_resources: dashmap::DashSet<String>,
    masters: Vec<Arc<uk_reader::ResourceReader>>,
    hash_table: &'static StockHashTable,
    compressor: Arc<Mutex<zstd::bulk::Compressor<'static>>>,
    _zip_opts: SimpleFileOptions,
    _out_file: PathBuf,
}

impl std::fmt::Debug for ModPacker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModBuilder")
            .field("source_dir", &self.source_dir)
            .field("current_root", &self.current_root)
            .field("meta", &self.meta)
            .field("endian", &self.endian)
            .field("masters", &self.masters)
            .field(
                "zip",
                &jstr!("zip::ZipWriter at {&self._out_file.to_string_lossy()}"),
            )
            .field("built_resources", &self.built_resources)
            .finish()
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
#[serde_as]
struct InfoJson {
    name:     String,
    desc:     String,
    #[serde(deserialize_with = "serde_with::As::<DefaultOnError>::deserialize")]
    version:  String,
    platform: String,
    options:  BnpOptions,
}

#[derive(Debug, Deserialize, Default)]
struct BnpOptions {
    #[serde(default)]
    multi:  Vec<BnpOption>,
    #[serde(default)]
    single: Vec<BnpGroup>,
}

#[derive(Debug, Deserialize)]
struct BnpOption {
    name:    String,
    desc:    String,
    folder:  PathBuf,
    default: Option<bool>,
}

impl From<BnpOption> for ModOption {
    fn from(opt: BnpOption) -> Self {
        Self {
            name: opt.name,
            description: opt.desc,
            path: opt.folder,
            requires: vec![],
        }
    }
}

fn multi_from_bnp_multi(opts: Vec<BnpOption>) -> OptionGroup {
    OptionGroup::Multiple(MultipleOptionGroup {
        name: "Optional Components".into(),
        description: "Select one or more of these".into(),
        defaults: opts
            .iter()
            .filter_map(|opt| opt.default.unwrap_or(false).then_some(opt.folder.clone()))
            .collect(),
        options: opts.into_iter().map(|opt| opt.into()).collect(),
        required: false,
    })
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RequireValue {
    Bool(bool),
    String(String),
}

impl RequireValue {
    fn is_true(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::String(s) => s.is_empty() || s == "false",
        }
    }
}

#[derive(Debug, Deserialize)]
struct BnpGroup {
    name:     String,
    desc:     String,
    required: Option<RequireValue>,
    options:  Vec<BnpOption>,
}

impl From<BnpGroup> for ExclusiveOptionGroup {
    fn from(group: BnpGroup) -> Self {
        Self {
            name: group.name,
            description: group.desc,
            default: None,
            options: group.options.into_iter().map(|opt| opt.into()).collect(),
            required: group.required.map(|r| r.is_true()).unwrap_or(false),
        }
    }
}

impl ModPacker {
    pub fn parse_rules(path: PathBuf) -> Result<Meta> {
        use configparser::ini::Ini;
        let mut rules = Ini::new();
        let parent = path.parent().context("No parent path???")?;
        rules.load(&path).map_err(|e| anyhow_ext::anyhow!(e))?;
        Ok(Meta {
            api: env!("CARGO_PKG_VERSION").into(),
            name: rules
                .get("Definition", "name")
                .context("rules.txt missing mod name")?
                .trim_matches('"')
                .into(),
            description: rules
                .get("Definition", "description")
                .context("rules.txt missing mod description")?
                .trim_matches('"')
                .into(),
            category: Default::default(),
            author: Default::default(),
            masters: Default::default(),
            options: vec![],
            platform: if parent.join("content").exists() || parent.join("aoc").exists() {
                ModPlatform::Specific(Endian::Big)
            } else {
                ModPlatform::Specific(Endian::Little)
            },
            url: Default::default(),
            version: "0.1.0".into(),
        })
    }

    pub fn parse_info(path: PathBuf) -> Result<Meta> {
        let info: InfoJson = serde_json::from_reader(fs::File::open(path)?)?;
        Ok(Meta {
            api: env!("CARGO_PKG_VERSION").into(),
            name: info.name,
            description: info.desc,
            category: Default::default(),
            author: Default::default(),
            masters: Default::default(),
            options: (!info.options.multi.is_empty())
                .then(|| multi_from_bnp_multi(info.options.multi))
                .into_iter()
                .chain(
                    info.options
                        .single
                        .into_iter()
                        .map(|grp| OptionGroup::Exclusive(grp.into())),
                )
                .collect(),
            platform: match info.platform.as_str() {
                "wiiu" => ModPlatform::Specific(Endian::Big),
                "switch" => ModPlatform::Specific(Endian::Little),
                _ => anyhow_ext::bail!("Invalid platform value in info.json"),
            },
            url: Default::default(),
            version: info.version,
        })
    }

    pub fn new(
        source: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        meta: Option<Meta>,
        masters: Vec<Arc<uk_reader::ResourceReader>>,
    ) -> Result<Self> {
        fn inner(
            source: &Path,
            dest: &Path,
            meta: Option<Meta>,
            masters: Vec<Arc<uk_reader::ResourceReader>>,
        ) -> Result<ModPacker> {
            log::info!("Attempting to package mod at {}", source.display());
            let source_dir = source.to_path_buf();
            if !(source_dir.exists() && source_dir.is_dir()) {
                anyhow_ext::bail!("Source directory does not exist: {}", source_dir.display());
            }
            let meta = if let Some(meta) = meta {
                log::debug!("Using providing meta info:\n{:#?}", &meta);
                meta
            } else if let Some(rules) = source.join("rules.txt").exists_then() {
                log::debug!("Attempting to parse existing rules.txt");
                ModPacker::parse_rules(rules)?
            } else if let Some(info) = source.join("info.json").exists_then() {
                log::debug!("Attempting to parse existing info.json");
                log::warn!(
                    "`info.json` found. If this is a BNP, conversion will not work properly!"
                );
                ModPacker::parse_info(info)?
            } else {
                anyhow_ext::bail!("No meta info provided or meta file available");
            };
            let ((content_u, dlc_u), (content_nx, dlc_nx)) = (
                platform_prefixes(Endian::Big),
                platform_prefixes(Endian::Little),
            );
            let endian = if source.join(content_u).exists() || source.join(dlc_u).exists() {
                Endian::Big
            } else if source.join(content_nx).exists() || source.join(dlc_nx).exists() {
                Endian::Little
            } else {
                anyhow_ext::bail!(
                    "No content or DLC folder found in source at {}",
                    source.display()
                );
            };
            let dest_file = if dest.is_dir() {
                dest.join(sanitise(&meta.name)).with_extension("zip")
            } else {
                dest.to_path_buf()
            };
            log::debug!("Using temp file at {}", dest_file.display());
            if dest_file.exists() {
                fs::remove_file(&dest_file)?;
            }
            log::debug!("Creating ZIP file");
            let zip = Arc::new(Mutex::new(ZipW::new(fs::File::create(&dest_file)?)));
            Ok(ModPacker {
                current_root: source_dir.clone(),
                source_dir,
                endian,
                zip,
                masters,
                hash_table: match endian {
                    Endian::Little => &NX_HASH_TABLE,
                    Endian::Big => &WIIU_HASH_TABLE,
                },
                meta,
                built_resources: Default::default(),
                compressor: Arc::new(Mutex::new(
                    zstd::bulk::Compressor::with_dictionary(8, super::DICTIONARY).unwrap(),
                )),
                _zip_opts: FileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
                _out_file: dest_file,
            })
        }
        inner(source.as_ref(), dest.as_ref(), meta, masters)
    }

    fn write_resource(&self, canon: &str, resource: &ResourceData) -> Result<()> {
        let data = minicbor_ser::to_vec(&resource)
            .map_err(|e| anyhow::format_err!("{:?}", e))
            .with_context(|| jstr!("Failed to serialize {canon}"))?;
        let zip_path = self
            .current_root
            .strip_prefix(&self.source_dir)
            .unwrap()
            .join(canon);
        {
            log::trace!("Writing {} to ZIP", canon);
            let mut zip = self.zip.lock();
            match zip.start_file(zip_path.to_slash_lossy(), self._zip_opts) {
                Ok(_) => zip.write_all(&self.compressor.lock().compress(&data)?)?,
                Err(zip::result::ZipError::InvalidArchive("Duplicate filename")) => {
                    log::warn!("Attempted to duplicate resource {}, skipping", canon);
                }
                e => return Err(e.unwrap_err().into()),
            }
        }
        self.built_resources.insert(canon.into());
        Ok(())
    }

    fn collect_resources(&self, root: PathBuf) -> Result<BTreeSet<String>> {
        let files = WalkDir::new(&root)
            .into_iter()
            .filter_map(|f| {
                f.ok()
                    .and_then(|f| f.file_type().is_file().then(|| f.path()))
            })
            .collect::<Vec<PathBuf>>();
        let total_files = files.len();
        let current_file = AtomicUsize::new(0);
        log::debug!("Resources found in root {}:\n{:#?}", root.display(), &files);
        Ok(files
            .into_par_iter()
            .map(|path| -> Result<Option<String>> {
                log::trace!("Processing resource at {}", path.display());
                let name: String = path
                    .strip_prefix(&self.current_root)
                    .unwrap()
                    .to_slash_lossy()
                    .into();
                // We know this is sound because we got `path` by iterating the contents of `root`.
                let canon = canonicalize(name.as_str());
                let file_data = fs::read(&path)?;
                let file_data = decompress_if(&file_data);

                if path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "AocMainField.pack")
                    .unwrap_or(false)
                    && file_data.is_empty()
                {
                    self.write_resource(
                        "Aoc/0010/Pack/AocMainField.pack",
                        &ResourceData::Sarc(Default::default()),
                    )?;
                    return Ok(Some("Pack/AocMainField.pack".into()));
                }

                if name.ends_with("sizetable") || file_data.len() < 4 {
                    return Ok(None);
                }

                if !self.hash_table.is_file_modded(&canon, &*file_data, true) {
                    log::trace!("Resource {} not modded, ignoring", &canon);
                    return Ok(None);
                }

                let resource = ResourceData::from_binary(name.as_str(), &*file_data)
                    .with_context(|| jstr!("Failed to parse resource {&name}"))?;
                let is_mergeable = matches!(resource, ResourceData::Mergeable(_));
                if let ResourceData::Mergeable(
                    uk_content::resource::MergeableResource::BinaryOverride(v),
                ) = &resource
                {
                    log::error!(
                        "There was an error processing {name}. It will not be processed but will \
                         be stored as-is, overriding anything else. Error details:\n{}",
                        v.1
                    );
                }
                self.process_resource(name.clone(), canon.clone(), resource, false)
                    .with_context(|| jstr!("Failed to process resource {&canon}"))?;
                if !is_mergeable && is_mergeable_sarc(canon.as_str(), file_data.as_ref()) {
                    log::trace!(
                        "Resource {} is a mergeable SARC, processing contents",
                        &canon
                    );
                    self.process_sarc(
                        Sarc::new(file_data.as_ref())?,
                        name.as_str().as_ref(),
                        self.hash_table.is_file_new(&canon),
                        canon.starts_with("Aoc"),
                    )
                    .with_context(|| jstr!("Failed to process SARC file {&canon}"))?;
                }

                let progress = current_file.load(std::sync::atomic::Ordering::Relaxed) + 1;
                current_file.store(progress, std::sync::atomic::Ordering::Relaxed);
                let percent = (progress as f64 / total_files as f64) * 100.0;
                let fract = percent.fract();
                if fract <= 0.1 || fract >= 0.95 {
                    log::trace!(
                        "PROGRESSBuilding {} files: {}%",
                        total_files,
                        percent as usize
                    );
                }

                Ok(Some(
                    path.strip_prefix(&root).unwrap().to_slash_lossy().into(),
                ))
            })
            .collect::<Result<Vec<Option<_>>>>()?
            .into_par_iter()
            .filter_map(|x| x)
            .collect())
    }

    fn process_resource(
        &self,
        name: String,
        canon: String,
        mut resource: ResourceData,
        in_new_sarc: bool,
    ) -> Result<()> {
        if self.built_resources.contains(&canon) {
            log::trace!("Already processed {}, skipping", &canon);
            return Ok(());
        }
        if canon.starts_with("Pack/Bootup_") {
            log::trace!("{} must always contain the same single file, skipping", &canon);
            return Ok(());
        }
        if resource.as_binary().is_some() && self.meta.platform == ModPlatform::Universal {
            anyhow_ext::bail!(
                "The resource {} is not a mergeable asset. Cross-platform mods must consist only \
                 of mergeable assets. While there is no ready-made comprehensive list of \
                 mergeable assets, common unmergeable assets include models, textures, music, and \
                 Havok physics data.",
                canon
            );
        }
        let prefixes = platform_prefixes(self.endian);
        let ref_name = name
            .trim_start_matches(prefixes.0)
            .trim_start_matches(prefixes.1)
            .trim_start_matches('/');
        let reference = self
            .masters
            .iter()
            .filter_map(|master| {
                master
                    .get_resource(canon.as_str())
                    .or_else(|err| {
                        log::trace!("{err}");
                        master.get_data(ref_name)
                    })
                    .inspect_err(|err| log::trace!("{err}"))
                    .or_else(|err| {
                        if let Some(lang) = canon
                            .starts_with("Message/Msg_")
                            .then(|| Language::from_message_path(ref_name.as_ref()))
                            .flatten()
                        {
                            let langs = master.languages();
                            match langs.iter().find(|l| l.short() == lang.short()) {
                                Some(ref_lang) => master.get_data(ref_lang.message_path().as_str()),
                                None => Err(err),
                            }
                        } else {
                            Err(err)
                        }
                    })
                    .ok()
            })
            .last();
        log::trace!("Resource {} has a master: {}", &canon, reference.is_some());
        if let (Some(res), Some(ref_res)) = (
            resource.as_mergeable(),
            reference.as_ref().and_then(|rrd| rrd.as_mergeable()),
        ) {
            if ref_res == res {
                log::trace!("{} not modded, skipping", &canon);
                return Ok(());
            }
            log::trace!("Diffing {}", &canon);
            resource = ResourceData::Mergeable(ref_res.diff(res));
        } else if let (Some(sarc), Some(ref_sarc)) = (
            resource.as_sarc(),
            reference.as_ref().and_then(|rrd| rrd.as_sarc()),
        ) {
            if ref_sarc == sarc && !in_new_sarc {
                log::trace!("{} not modded, skipping", &canon);
                return Ok(());
            }
            log::trace!("Diffing {}", &canon);
            resource = ResourceData::Sarc(ref_sarc.diff(sarc));
        }

        self.write_resource(&canon, &resource)?;

        Ok(())
    }

    fn process_sarc(&self, sarc: Sarc, path: &Path, is_new_sarc: bool, is_aoc: bool) -> Result<()> {
        for file in sarc.files() {
            if file.data.is_empty() {
                continue;
            }
            let name = file
                .name()
                .with_context(|| jstr!("File in SARC missing name"))?;
            let mut canon = canonicalize(name);
            if is_aoc {
                canon.insert_str(0, "Aoc/0010/");
            }
            let file_data = decompress_if(file.data);

            if !self.hash_table.is_file_modded(&canon, &*file_data, true) && !is_new_sarc {
                log::trace!("{} in SARC {} not modded, skipping", &canon, path.display());
                continue;
            }

            let resource = ResourceData::from_binary(name, &*file_data).with_context(|| {
                jstr!("Failed to parse resource {&canon} in SARC {&path.display().to_string()}")
            })?;
            if let ResourceData::Mergeable(
                uk_content::resource::MergeableResource::BinaryOverride(v),
            ) = &resource
            {
                log::error!(
                    "There was an error processing {name}. It will not be processed but will be \
                     stored as-is, overriding anything else. Error details:\n{}",
                    v.1
                );
            }
            self.process_resource(name.into(), canon.clone(), resource, is_new_sarc)?;
            if is_mergeable_sarc(canon.as_str(), file_data.as_ref()) {
                log::trace!(
                    "Resource {} in SARC {} is a mergeable SARC, processing contents",
                    &canon,
                    path.display()
                );
                self.process_sarc(
                    Sarc::new(file_data.as_ref())?,
                    name.as_ref(),
                    is_new_sarc,
                    is_aoc,
                )
                .with_context(|| {
                    jstr!("Failed to process {&canon} in SARC {&path.display().to_string()}")
                })?;
            }
        }
        Ok(())
    }

    fn pack_root(&self, root: impl AsRef<Path>) -> Result<()> {
        fn inner(self_: &ModPacker, root: &Path) -> Result<()> {
            log::debug!("Packing from root of {}", root.display());
            self_.built_resources.clear();
            let (content, aoc) = platform_prefixes(self_.endian);
            let content_dir = root.join(content);
            log::debug!("Checking for content folder at {}", content_dir.display());
            let content_dir = if content_dir.exists() {
                log::debug!("Found content folder at {}", content_dir.display());
                Some(content_dir)
            } else {
                None
            };
            let aoc_dir = root.join(aoc);
            log::debug!("Checking for DLC folder at {}", aoc_dir.display());
            let aoc_dir = if aoc_dir.exists() {
                log::debug!("Found DLC folder at {}", aoc_dir.display());
                Some(aoc_dir)
            } else {
                None
            };
            let content_files = content_dir
                .map(|content| {
                    log::info!("Collecting resources");
                    self_.collect_resources(content)
                })
                .transpose()?
                .inspect(|_| log::info!("Finished collecting resources"))
                .unwrap_or_default();
            let aoc_files = aoc_dir
                .map(|aoc| {
                    log::info!("Collecting DLC resources");
                    self_.collect_resources(aoc)
                })
                .transpose()?
                .inspect(|_| log::info!("Finished collecting DLC resources"))
                .unwrap_or_default();
            log::info!("Generating manifest");
            let mut manifest = Manifest {
                content_files,
                aoc_files,
            };
            log::trace!("CLEARPROGRESS");
            if manifest
                .aoc_files
                .iter()
                .any(|f| f.contains("Map/MainField"))
                && !manifest.aoc_files.contains("Pack/AocMainField.pack")
            {
                self_.write_resource(
                    "Aoc/0010/Pack/AocMainField.pack",
                    &ResourceData::Sarc(Default::default()),
                )?;
                manifest.aoc_files.insert("Pack/AocMainField.pack".into());
            }
            let manifest = serde_yaml::to_string(&manifest)?;
            log::info!("Writing manifest");
            let mut zip = self_.zip.lock();
            zip.start_file(
                root.strip_prefix(&self_.source_dir)
                    .unwrap()
                    .join("manifest.yml")
                    .to_slash_lossy(),
                self_._zip_opts,
            )?;
            zip.write_all(manifest.as_bytes())?;
            Ok(())
        }
        inner(self, root.as_ref())
    }

    fn collect_roots(&self) -> Vec<PathBuf> {
        let opt_root = self.source_dir.join("options");
        let mut roots = HashSet::new();
        for group in &self.meta.options {
            roots.extend(group.options().iter().map(|opt| opt_root.join(&opt.path)))
        }
        log::debug!("Detected options:\n{:#?}", &roots);
        roots.into_iter().collect()
    }

    fn pack_thumbnail(&self) -> Result<()> {
        for name in ["thumb", "thumbnail", "preview"] {
            for ext in ["jpg", "jpeg", "png", "svg"] {
                let path = self.source_dir.join(name).with_extension(ext);
                if path.exists() {
                    let mut zip = self.zip.lock();
                    zip.start_file(format!("thumb.{}", ext), self._zip_opts)?;
                    zip.write_all(&fs::read(path)?)?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn pack(mut self) -> Result<PathBuf> {
        self.pack_root(&self.source_dir).with_context(|| {
            format!(
                "Failed to package mod root at {} for mod {}",
                self.source_dir.display(),
                self.meta.name
            )
        })?;
        if self.source_dir.join("options").exists() {
            log::debug!("Mod contains options");
            self.masters
                .push(Arc::new(uk_reader::ResourceReader::from_unpacked_mod(
                    &self.source_dir,
                )?));
            log::info!("Collecting resources for options");
            for root in self.collect_roots() {
                self.current_root.clone_from(&root);
                self.pack_root(root).with_context(|| {
                    format!(
                        "Failed to package mod root at {} for mod {}",
                        self.current_root.display(),
                        self.meta.name
                    )
                })?;
            }
        }
        self.pack_thumbnail()?;
        match Arc::try_unwrap(self.zip).map(|z| z.into_inner()) {
            Ok(mut zip) => {
                log::info!("Writing meta");
                zip.start_file("meta.yml", self._zip_opts)?;
                zip.write_all(serde_yaml::to_string(&self.meta)?.as_bytes())?;
                zip.finish()?
            }
            Err(_) => {
                anyhow_ext::bail!("Failed to finish writing zip, this is probably a big deal")
            }
        };
        log::info!("Completed packaging mod");
        Ok(self._out_file)
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use uk_reader::ResourceReader;

    use super::*;
    use crate::{ModOption, MultipleOptionGroup, OptionGroup};
    #[test]
    fn pack_mod() {
        env_logger::init();
        let source = Path::new("test/wiiu");
        let dest = Path::new("test/wiiu.zip");
        let rom_reader = serde_yaml::from_str::<ResourceReader>(
            &std::fs::read_to_string("../.vscode/dump.yml").unwrap(),
        )
        .unwrap();
        let builder = ModPacker::new(
            source,
            dest,
            Some(Meta {
                api: env!("CARGO_PKG_VERSION").into(),
                platform: ModPlatform::Specific(Endian::Big),
                name: "Test Mod".into(),
                version: "0.1.0".into(),
                category: "Overhaul".into(),
                author: "Lord Caleb".into(),
                description: "A test mod".into(),
                masters: IndexMap::default(),
                url: None,
                options: vec![OptionGroup::Multiple(MultipleOptionGroup {
                    name: "Test Option Group".into(),
                    description: "A test option group".into(),
                    defaults: ["option1".into()].into_iter().collect(),
                    options: [ModOption {
                        name: "Test Option".into(),
                        description: "An option".into(),
                        path: "option1".into(),
                        requires: vec![],
                    }]
                    .into_iter()
                    .collect(),
                    required: false,
                })],
            }),
            vec![Arc::new(rom_reader)],
        )
        .unwrap();
        builder.pack().unwrap();
    }
}
