use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc},
};

use anyhow::{Context, Result};
use botw_utils::hashes::StockHashTable;
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::Sarc, yaz0::decompress_if};
pub use sanitise_file_name::sanitise;
use serde::Deserialize;
use smartstring::alias::String;
use uk_content::{
    canonicalize, platform_prefixes,
    prelude::{Endian, Mergeable},
    resource::{is_mergeable_sarc, ResourceData},
};
use zip::{write::FileOptions, ZipWriter as ZipW};

use crate::{
    ExclusiveOptionGroup, Manifest, Meta, ModOption, ModOptionGroup, ModPlatform,
    MultipleOptionGroup, OptionGroup,
};

pub type ZipWriter = Arc<Mutex<ZipW<fs::File>>>;

static NX_HASH_TABLE: Lazy<StockHashTable> =
    Lazy::new(|| StockHashTable::new(&botw_utils::hashes::Platform::Switch));
static WIIU_HASH_TABLE: Lazy<StockHashTable> =
    Lazy::new(|| StockHashTable::new(&botw_utils::hashes::Platform::WiiU));

pub struct ModPacker {
    source_dir: PathBuf,
    current_root: PathBuf,
    meta: Meta,
    zip: ZipWriter,
    endian: Endian,
    built_resources: Arc<RwLock<BTreeSet<String>>>,
    masters: Vec<Arc<uk_reader::ResourceReader>>,
    hash_table: &'static StockHashTable,
    _zip_opts: FileOptions,
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

#[derive(Debug, Deserialize)]
struct InfoJson {
    name:     String,
    desc:     String,
    version:  String,
    platform: String,
    options:  BnpOptions,
}

#[derive(Debug, Deserialize)]
struct BnpOptions {
    multi:  Vec<BnpOption>,
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
struct BnpGroup {
    name:     String,
    desc:     String,
    required: String,
    options:  Vec<BnpOption>,
}

impl From<BnpGroup> for ExclusiveOptionGroup {
    fn from(group: BnpGroup) -> Self {
        Self {
            name: group.name,
            description: group.desc,
            default: None,
            options: group.options.into_iter().map(|opt| opt.into()).collect(),
            required: !group.required.is_empty(),
        }
    }
}

impl ModPacker {
    fn parse_rules(path: PathBuf) -> Result<Meta> {
        use configparser::ini::Ini;
        let mut rules = Ini::new();
        let parent = path.parent().context("No parent path???")?;
        rules.load(&path).map_err(|e| anyhow::anyhow!(e))?;
        Ok(Meta {
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
                _ => anyhow::bail!("Invalid platform value in info.json"),
            },
            url: Default::default(),
            version: info.version,
        })
    }

    #[allow(irrefutable_let_patterns)]
    pub fn new(
        source: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        meta: Option<Meta>,
        masters: Vec<Arc<uk_reader::ResourceReader>>,
    ) -> Result<Self> {
        log::info!("Attempting to package mod at {}", source.as_ref().display());
        let source_dir = source.as_ref().to_path_buf();
        if !(source_dir.exists() && source_dir.is_dir()) {
            anyhow::bail!("Source directory does not exist: {}", source_dir.display());
        }
        let meta = if let Some(meta) = meta {
            log::debug!("Using providing meta info:\n{:#?}", &meta);
            meta
        } else if let rules = source.as_ref().join("rules.txt") && rules.exists() {
            log::debug!("Attempting to parse existing rules.txt");
            Self::parse_rules(rules)?
        } else if let info = source.as_ref().join("info.json") && info.exists() {
            log::debug!("Attempting to parse existing info.json");
            log::warn!("`info.json` found. If this is a BNP, conversion will not work properly!");
            Self::parse_info(info)?
        } else {
            anyhow::bail!("No meta info provided or meta file available");
        };
        let ((content_u, dlc_u), (content_nx, dlc_nx)) = (
            platform_prefixes(Endian::Big),
            platform_prefixes(Endian::Little),
        );
        let endian = if source.as_ref().join(content_u).exists()
            || source.as_ref().join(dlc_u).exists()
        {
            Endian::Big
        } else if source.as_ref().join(content_nx).exists() || source.as_ref().join(dlc_nx).exists()
        {
            Endian::Little
        } else {
            anyhow::bail!(
                "No content or DLC folder found in source at {}",
                source.as_ref().display()
            );
        };
        let dest = dest.as_ref();
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
        Ok(Self {
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
            built_resources: Arc::new(RwLock::new(BTreeSet::new())),
            _zip_opts: FileOptions::default().compression_method(zip::CompressionMethod::Stored),
            _out_file: dest_file,
        })
    }

    fn collect_resources(&self, root: impl AsRef<Path>) -> Result<BTreeSet<String>> {
        let root = root.as_ref();
        let files = WalkDir::new(root)
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

                if name.ends_with("sizetable") {
                    return Ok(None);
                }

                if !self.hash_table.is_file_modded(&canon, &*file_data, true) {
                    log::trace!("Resource {} not modded, ignoring", &canon);
                    return Ok(None);
                }

                let resource = ResourceData::from_binary(name.as_str(), &*file_data)
                    .with_context(|| jstr!("Failed to parse resource {&name}"))?;
                self.process_resource(name.clone(), canon.clone(), resource, false)
                    .with_context(|| jstr!("Failed to process resource {&canon}"))?;
                if is_mergeable_sarc(canon.as_str(), file_data.as_ref()) {
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
                    log::info!(
                        "PROGRESSBuilding {} files: {}%",
                        total_files,
                        percent as usize
                    );
                }

                Ok(Some(
                    path.strip_prefix(root).unwrap().to_slash_lossy().into(),
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
        if self.built_resources.read().contains(&canon) {
            log::trace!("Already processed {}, skipping", &canon);
            return Ok(());
        }
        if resource.as_binary().is_some() && self.meta.platform == ModPlatform::Universal {
            anyhow::bail!(
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
                    .ok()
            })
            .last();
        log::trace!("Resource {} has a master: {}", &canon, reference.is_some());
        if let Some(ref_res_data) = reference.as_ref()
            && let Some(ref_res) = ref_res_data.as_mergeable()
            && let ResourceData::Mergeable(res) = &resource
        {
            if ref_res == res {
                log::trace!("{} not modded, skipping", &canon);
                return Ok(());
            }
            log::trace!("Diffing {}", &canon);
            resource = ResourceData::Mergeable(ref_res.diff(res));
        } else if let Some(ref_res_data) = reference.as_ref()
            && let Some(ref_sarc) = ref_res_data.as_sarc()
            && let ResourceData::Sarc(sarc) = &resource
        {
            if ref_sarc == sarc && !in_new_sarc {
                log::trace!("{} not modded, skipping", &canon);
                return Ok(());
            }
            log::trace!("Diffing {}", &canon);
            resource = ResourceData::Sarc(ref_sarc.diff(sarc));
        }

        let data = minicbor_ser::to_vec(&resource)
            .map_err(|e| anyhow::format_err!("{:?}", e))
            .with_context(|| jstr!("Failed to serialize {&name}"))?;
        let zip_path = self
            .current_root
            .strip_prefix(&self.source_dir)
            .unwrap()
            .join(canon.as_str());
        {
            log::trace!("Writing {} to ZIP", &canon);
            let mut zip = self.zip.lock();
            zip.start_file(zip_path.to_slash_lossy(), self._zip_opts)?;
            zip.write_all(&zstd::encode_all(&*data, 3)?)?;
        }
        self.built_resources.write().insert(canon);

        Ok(())
    }

    fn process_sarc(&self, sarc: Sarc, path: &Path, is_new_sarc: bool, is_aoc: bool) -> Result<()> {
        for file in sarc.files() {
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

    #[allow(irrefutable_let_patterns)]
    fn pack_root(&self, root: impl AsRef<Path>) -> Result<()> {
        log::debug!("Packing from root of {}", root.as_ref().display());
        self.built_resources.write().clear();
        let (content, aoc) = platform_prefixes(self.endian);
        let content_dir = root.as_ref().join(content);
        log::debug!("Checking for content folder at {}", content_dir.display());
        let content_dir = if content_dir.exists() {
            log::debug!("Found content folder at {}", content_dir.display());
            Some(content_dir)
        } else {
            None
        };
        let aoc_dir = root.as_ref().join(aoc);
        log::debug!("Checking for DLC folder at {}", aoc_dir.display());
        let aoc_dir = if aoc_dir.exists() {
            log::debug!("Found DLC folder at {}", aoc_dir.display());
            Some(aoc_dir)
        } else {
            None
        };
        let manifest = serde_yaml::to_string(&Manifest {
            content_files: content_dir
                .map(|content| {
                    log::info!("Collecting resources");
                    self.collect_resources(content)
                })
                .transpose()?
                .unwrap_or_default(),
            aoc_files:     aoc_dir
                .map(|aoc| {
                    log::info!("Collecting DLC resources");
                    self.collect_resources(aoc)
                })
                .transpose()?
                .unwrap_or_default(),
        })?;
        log::info!("Writing manifest");
        let mut zip = self.zip.lock();
        zip.start_file(
            root.as_ref()
                .strip_prefix(&self.source_dir)
                .unwrap()
                .join("manifest.yml")
                .to_slash_lossy(),
            self._zip_opts,
        )?;
        zip.write_all(manifest.as_bytes())?;
        Ok(())
    }

    fn collect_roots(&self) -> Vec<PathBuf> {
        let opt_root = self.source_dir.join("options");
        let mut roots = Vec::new();
        for group in &self.meta.options {
            roots.extend(group.options().iter().map(|opt| opt_root.join(&opt.path)))
        }
        log::debug!("Detected options:\n{:#?}", &roots);
        roots
    }

    pub fn pack(mut self) -> Result<PathBuf> {
        self.pack_root(&self.source_dir)?;
        if self.source_dir.join("options").exists() {
            log::debug!("Mod contains options");
            self.masters
                .push(Arc::new(uk_reader::ResourceReader::from_unpacked_mod(
                    &self.source_dir,
                )?));
            log::info!("Collecting resources for options");
            for root in self.collect_roots() {
                self.current_root = root.clone();
                self.pack_root(root)?;
            }
        }
        match Arc::try_unwrap(self.zip).map(|z| z.into_inner()) {
            Ok(mut zip) => {
                log::info!("Writing meta");
                zip.start_file("meta.yml", self._zip_opts)?;
                zip.write_all(serde_yaml::to_string(&self.meta)?.as_bytes())?;
                zip.finish()?
            }
            Err(_) => anyhow::bail!("Failed to finish writing zip, this is probably a big deal"),
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
