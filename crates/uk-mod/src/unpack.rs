mod de;
use std::{
    collections::BTreeSet,
    io::{BufReader, Read, Write},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow_ext::{bail, Context, Result};
use botw_utils::hashes::StockHashTable;
use dashmap::DashMap;
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use lenient_semver::Version;
use mmap_rs::{Mmap, MmapOptions};
use ouroboros::self_referencing;
use parking_lot::Mutex;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{
    sarc::SarcWriter,
    yaz0::{compress, compress_if},
};
use serde::Serialize;
use smartstring::alias::String;
use uk_content::{
    canonicalize,
    constants::Language,
    platform_content, platform_prefixes,
    prelude::{Endian, Mergeable, Resource},
    resource::{MergeableResource, ResourceData, SarcMap},
    util::{HashMap, IndexSet},
};
use uk_reader::{ResourceLoader, ResourceReader};
use uk_util::PathExt as UkPathExt;

use crate::{Manifest, Meta, ModOption};

pub enum ZipData {
    Owned(Vec<u8>),
    Memory(Mmap),
}

impl std::ops::Deref for ZipData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            ZipData::Owned(d) => d.as_slice(),
            ZipData::Memory(d) => d.as_slice(),
        }
    }
}

impl std::fmt::Debug for ZipData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = match self {
            ZipData::Owned(v) => v.as_ptr() as usize,
            ZipData::Memory(m) => m.as_ptr() as usize,
        };
        f.debug_struct("ZipData")
            .field(
                match self {
                    ZipData::Owned(_) => "Owned",
                    ZipData::Memory(_) => "Memory",
                },
                &format!("0x{:x}", ptr),
            )
            .finish()
    }
}

#[self_referencing]
pub struct ParallelZipReader {
    data:  ZipData,
    #[borrows(data)]
    #[covariant]
    zip:   piz::ZipArchive<'this>,
    #[borrows(zip)]
    #[covariant]
    files: HashMap<&'this Path, &'this piz::read::FileMetadata<'this>>,
}

unsafe impl Send for ParallelZipReader {}
unsafe impl Sync for ParallelZipReader {}

impl std::fmt::Debug for ParallelZipReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParallelZipReader").finish_non_exhaustive()
    }
}

impl ParallelZipReader {
    pub fn open(path: impl AsRef<Path>, peek: bool) -> Result<Self> {
        fn inner(path: &Path, peek: bool) -> Result<ParallelZipReader> {
            let mut file = std::fs::File::open(path)?;
            let len = file.metadata()?.len() as usize;
            let self_ = ParallelZipReaderTryBuilder {
            data: if len > (1024 * 1024 * 256) || peek {
                unsafe { ZipData::Memory(MmapOptions::new(len).expect("Bad map size").with_file(&file, 0).map()?) }
            } else {
                let mut buffer = vec![0u8; len];
                file.read_exact(&mut buffer)?;
                ZipData::Owned(buffer)
            },
            zip_builder: |map: &ZipData| -> Result<piz::ZipArchive<'_>> {
                Ok(piz::ZipArchive::new(map)?)
            },
            files_builder:
                |zip: &piz::ZipArchive<'_>| -> Result<HashMap<&Path, &piz::read::FileMetadata>> {
                    Ok(zip
                        .entries()
                        .iter()
                        .map(|e| (e.path.as_std_path(), e))
                        .collect::<HashMap<_, _>>())
                },
        }
        .try_build()?;
            Ok(self_)
        }
        inner(path.as_ref(), peek)
    }

    pub fn iter(&self) -> impl Iterator<Item = &&std::path::Path> {
        self.borrow_files().iter().map(|(f, _)| f)
    }

    pub fn get_file(&self, file: impl AsRef<Path>) -> Result<Vec<u8>> {
        fn inner(self_: &ParallelZipReader, file: &Path) -> Result<Vec<u8>> {
            self_
                .borrow_files()
                .get(file)
                .with_context(|| format!("File {} not found in ZIP", file.display()))
                .and_then(|file| {
                    let mut reader = self_
                        .borrow_zip()
                        .read(file)
                        .with_context(|| format!("Failed to lookup file {} in ZIP", &file.path))?;
                    let mut buffer = vec![0u8; file.size];
                    reader
                        .read_exact(&mut buffer)
                        .with_context(|| format!("Failed to read file {} from ZIP", &file.path))?;
                    Ok(buffer)
                })
        }
        inner(self, file.as_ref())
    }
}

#[inline]
pub fn init_decompressor() -> Arc<Mutex<zstd::bulk::Decompressor<'static>>> {
    Arc::new(Mutex::new(
        zstd::bulk::Decompressor::with_dictionary(super::DICTIONARY).unwrap(),
    ))
}

#[derive(Serialize)]
pub struct ModReader {
    pub path: PathBuf,
    options: Vec<ModOption>,
    pub meta: Meta,
    pub manifest: Manifest,
    #[serde(skip, default = "init_decompressor")]
    decompressor: Arc<Mutex<zstd::bulk::Decompressor<'static>>>,
    #[serde(skip_serializing)]
    zip: Option<ParallelZipReader>,
}

impl std::fmt::Debug for ModReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModReader")
            .field("path", &self.path)
            .field("options", &self.options)
            .field("meta", &self.meta)
            .field("manifest", &self.manifest)
            .field("decompressor", &"zstd::bulk::Decompressor<'static>")
            .field("zip", &self.zip)
            .finish()
    }
}

#[typetag::serde]
impl ResourceLoader for ModReader {
    fn file_exists(&self, name: &Path) -> bool {
        let name = name.to_slash_lossy();
        self.manifest.content_files.contains(name.as_ref())
            || self.manifest.aoc_files.contains(name.as_ref())
    }

    fn get_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(name);
        if let Some(zip) = self.zip.as_ref() {
            if let Ok(data) = zip.get_file(canon.as_str()) {
                return Ok(self
                    .decompress(data.as_slice())
                    .with_context(|| jstr!("Failed to decompress file {&canon} from mod"))?);
            }
        } else if let Some(path) = self.path.join(canon.as_str()).exists_then() {
            return Ok(fs::read(path)?);
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(canon.as_str());
            if let Some(zip) = self.zip.as_ref() {
                if let Ok(data) = zip.get_file(path) {
                    return Ok(self
                        .decompress(data.as_slice())
                        .with_context(|| jstr!("Failed to decompress file {&canon} from mod"))?);
                }
            } else if let Some(path) = self.path.join(path).exists_then() {
                return Ok(fs::read(path)?);
            }
        }
        self.get_aoc_file_data(name).map_err(|_| {
            anyhow_ext::anyhow!(
                "Failed to read file {} (canonical path {}) from mod",
                name.display(),
                canon
            )
            .into()
        })
    }

    fn get_aoc_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(jstr!("Aoc/0010/{name.to_str().unwrap_or_default()}"));
        if let Some(zip) = self.zip.as_ref() {
            if let Ok(data) = zip.get_file(canon.as_str()) {
                return Ok(self
                    .decompress(data.as_slice())
                    .with_context(|| jstr!("Failed to decompress file {&canon} from mod"))?);
            }
        } else if let Some(path) = self.path.join(canon.as_str()).exists_then() {
            return Ok(fs::read(path)?);
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(canon.as_str());
            if let Some(zip) = self.zip.as_ref() {
                if let Ok(data) = zip.get_file(path) {
                    return Ok(self
                        .decompress(data.as_slice())
                        .with_context(|| jstr!("Failed to decompress file {&canon} from mod"))?);
                }
            } else if let Some(path) = self.path.join(path).exists_then() {
                return Ok(fs::read(path)?);
            }
        }
        Err(anyhow_ext::anyhow!(
            "Failed to read file {} (canonical path {}) from mod",
            name.display(),
            canon
        )
        .into())
    }

    fn host_path(&self) -> &Path {
        &self.path
    }
}

impl ModReader {
    #[inline]
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decomp = self.decompressor.lock();
        let size = zstd::bulk::Decompressor::upper_bound(data).unwrap_or(data.len() * 1024);
        decomp
            .decompress(data, size)
            .or_else(|e| zstd::decode_all(data).context(e))
    }

    pub fn open(path: impl AsRef<Path>, options: impl Into<Vec<ModOption>>) -> Result<Self> {
        fn inner(path: &Path, options: Vec<ModOption>) -> Result<ModReader> {
            let path = path.to_path_buf();
            let result = if path.is_file() {
                ModReader::open_zipped(path, options)
            } else {
                ModReader::open_unzipped(path, options)
            }?;
            let name = result.meta.name.as_str();
            let mod_api = Version::parse(&result.meta.api)
                .map_err(|e| anyhow_ext::anyhow!("{e}"))
                .context("Invalid API version for mod")?;
            let current_api = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if current_api.major != mod_api.major {
                anyhow_ext::bail!("{name} build with unsupported UKMM version: {mod_api}")
            } else if current_api.minor > mod_api.minor && current_api.major == 0 {
                log::warn!(
                    "{name} is from an older UKMM prerelease (v{mod_api}), compatibility not \
                     guaranteed"
                )
            } else if current_api.minor < mod_api.minor {
                log::warn!(
                    "{name} was made with a newer UKMM version (v{mod_api}), might contain \
                     incompatible features"
                )
            }
            Ok(result)
        }
        inner(path.as_ref(), options.into())
    }

    pub fn open_peek(path: impl AsRef<Path>, options: impl Into<Vec<ModOption>>) -> Result<Self> {
        fn inner(path: &Path, options: Vec<ModOption>) -> Result<ModReader> {
            let path = path.to_path_buf();
            let result = if path.is_file() {
                ModReader::open_zipped_peek(path, options)
            } else {
                ModReader::open_unzipped(path, options)
            }?;
            let name = result.meta.name.as_str();
            let mod_api = Version::parse(&result.meta.api)
                .map_err(|e| anyhow_ext::anyhow!("{e}"))
                .context("Invalid API version for mod")?;
            let current_api = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
            if current_api.major != mod_api.major {
                anyhow_ext::bail!("{name} build with unsupported UKMM version: {mod_api}")
            } else if current_api.minor > mod_api.minor && current_api.major == 0 {
                log::warn!(
                    "{name} is from an older UKMM prerelease (v{mod_api}), compatibility not \
                     guaranteed"
                )
            } else if current_api.minor < mod_api.minor {
                log::warn!(
                    "{name} was made with a newer UKMM version (v{mod_api}), might contain \
                     incompatible features"
                )
            }
            Ok(result)
        }
        inner(path.as_ref(), options.into())
    }

    fn open_unzipped(path: PathBuf, options: Vec<ModOption>) -> Result<Self> {
        let meta: Meta = serde_yaml::from_str(&fs::read_to_string(path.join("meta.yml"))?)?;
        let mut manifest: Manifest =
            serde_yaml::from_str(&fs::read_to_string(path.join("manifest.yml"))?)?;
        for option in &options {
            let opt_manifest: Manifest =
                serde_yaml::from_str(&fs::read_to_string(path.join(option.manifest_path()))?)?;
            manifest.content_files.extend(opt_manifest.content_files);
            manifest.aoc_files.extend(opt_manifest.aoc_files);
        }
        Ok(Self {
            path,
            decompressor: init_decompressor(),
            options,
            meta,
            manifest,
            zip: None,
        })
    }

    pub fn from_archive(
        path: PathBuf,
        zip: ParallelZipReader,
        options: Vec<ModOption>,
    ) -> Result<Self> {
        let mut buffer = vec![0; 524288]; // 512kb
        let mut read;
        let mut size;
        let meta: Meta = {
            let meta = zip
                .borrow_files()
                .get(Path::new("meta.yml"))
                .context("Mod missing meta file")?;
            size = meta.size;
            let mut reader = zip.borrow_zip().read(meta)?;
            read = reader.read(&mut buffer)?;
            if read != size {
                anyhow_ext::bail!("Failed to read meta file from mod {}", path.display());
            }
            serde_yaml::from_slice(&buffer[..read]).context("Failed to parse meta file from mod")?
        };
        let mut manifest = {
            let manifest = zip
                .borrow_files()
                .get(Path::new("manifest.yml"))
                .context("Mod missing manifest file")?;
            size = manifest.size;
            let mut reader = zip.borrow_zip().read(manifest)?;
            read = reader.read(&mut buffer)?;
            if read != size {
                anyhow_ext::bail!("Failed to read manifest file from mod")
            }
            serde_yaml::from_str::<Manifest>(std::str::from_utf8(&buffer[..read])?)
                .context("Failed to parse manifest file")?
        };
        for opt in &options {
            let opt_manifest = zip
                .borrow_files()
                .get(opt.manifest_path().as_path())
                .context("Mod missing option manifest file")?;
            size = opt_manifest.size;
            let mut reader = zip.borrow_zip().read(opt_manifest)?;
            read = reader.read(&mut buffer)?;
            if read != size {
                anyhow_ext::bail!("Failed to read option manifest file from mod")
            }
            let opt_manifest =
                serde_yaml::from_str::<Manifest>(std::str::from_utf8(&buffer[..read])?)
                    .context("Failed to parse option manifest file")?;
            manifest.content_files.extend(opt_manifest.content_files);
            manifest.aoc_files.extend(opt_manifest.aoc_files);
        }
        Ok(Self {
            path,
            decompressor: init_decompressor(),
            options,
            meta,
            manifest,
            zip: Some(zip),
        })
    }

    fn open_zipped(path: PathBuf, options: Vec<ModOption>) -> Result<Self> {
        let zip = ParallelZipReader::open(&path, false)?;
        Self::from_archive(path, zip, options)
    }

    fn open_zipped_peek(path: PathBuf, options: Vec<ModOption>) -> Result<Self> {
        let zip = ParallelZipReader::open(&path, true)?;
        Self::from_archive(path, zip, options)
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn get_versions(&self, name: &Path) -> Result<Vec<Vec<u8>>> {
        let canon = canonicalize(name);
        let mut versions = Vec::with_capacity(1);
        if let Some(zip) = self.zip.as_ref() {
            if let Ok(data) = zip.get_file(canon.as_str()) {
                versions.push(
                    self.decompress(data.as_slice())
                        .with_context(|| jstr!("Failed to decompress file {&canon} from mod"))?,
                );
            }
        } else if let Some(path) = self.path.join(canon.as_str()).exists_then() {
            versions.push(fs::read(path)?);
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(canon.as_str());
            if let Some(zip) = self.zip.as_ref() {
                if let Ok(data) = zip.get_file(path) {
                    versions.push(
                        self.decompress(data.as_slice()).with_context(|| {
                            jstr!("Failed to decompress file {&canon} from mod")
                        })?,
                    );
                }
            } else if let Some(path) = self.path.join(path).exists_then() {
                versions.push(fs::read(path)?);
            }
        }
        if let Ok(data) = self.get_aoc_file_data(name) {
            versions.push(data);
        }
        if versions.is_empty() {
            anyhow_ext::bail!(
                "Failed to find file {} (canonical path {}) from mod",
                name.display(),
                canon
            )
        }
        Ok(versions)
    }
}

static RSTB_EXCLUDE_EXTS: &[&str] = &[
    "pack", "bgdata", "txt", "bgsvdata", "yml", "msbt", "bat", "ini", "png", "bfstm", "py", "sh",
];
static RSTB_EXCLUDE_NAMES: &[&str] = &["ActorInfo.product.byml"];

// #[derive(Debug)]
pub struct ModUnpacker {
    dump:     Arc<ResourceReader>,
    manifest: Option<Manifest>,
    mods:     Vec<ModReader>,
    endian:   Endian,
    lang:     Language,
    rstb:     DashMap<String, Option<u32>>,
    hashes:   StockHashTable,
    out_dir:  PathBuf,
}

impl ModUnpacker {
    pub fn new(
        dump: Arc<ResourceReader>,
        endian: Endian,
        lang: Language,
        mods: Vec<ModReader>,
        out_dir: PathBuf,
    ) -> Self {
        Self {
            dump,
            manifest: None,
            mods,
            lang,
            endian,
            rstb: DashMap::new(),
            hashes: StockHashTable::new(&match endian {
                Endian::Little => botw_utils::hashes::Platform::Switch,
                Endian::Big => botw_utils::hashes::Platform::WiiU,
            }),
            out_dir,
        }
    }

    pub fn with_manifest(mut self, manifest: Manifest) -> Self {
        self.manifest = Some(manifest);
        self
    }

    pub fn unpack(self) -> Result<DashMap<String, Option<u32>>> {
        if !self.out_dir.exists() {
            fs::create_dir_all(&self.out_dir)?;
        }
        let mut content_files: BTreeSet<&String>;
        let aoc_files: BTreeSet<&String>;
        if let Some(manifest) = self.manifest.as_ref() {
            content_files = manifest.content_files.iter().collect();
            aoc_files = manifest.aoc_files.iter().collect();
        } else {
            content_files = self
                .mods
                .iter()
                .flat_map(|mod_| mod_.manifest.content_files.iter())
                .collect();
            aoc_files = self
                .mods
                .iter()
                .flat_map(|mod_| mod_.manifest.aoc_files.iter())
                .collect();
        }
        let mut modded_langs: IndexSet<Language> = Default::default();
        for lang in self.mods.iter().flat_map(|m| m.manifest.languages()) {
            modded_langs.insert(lang);
            content_files.remove(&lang.bootup_path());
        }
        let (content, aoc) = platform_prefixes(self.endian);
        let total = content_files.len() + aoc_files.len();
        let current = AtomicUsize::new(0);
        std::thread::scope(|s| -> Result<()> {
            let jobs = [
                s.spawn(|| {
                    log::info!("Unpacking base files…");
                    self.unpack_files(
                        content_files,
                        self.out_dir.join(content),
                        total,
                        &current,
                        false,
                    )
                    .map(|_| "base files")
                }),
                s.spawn(|| {
                    log::info!("Unpacking DLC files…");
                    self.unpack_files(aoc_files, self.out_dir.join(aoc), total, &current, true)
                        .map(|_| "DLC files")
                }),
                s.spawn(|| {
                    log::info!("Unpacking game texts…");
                    self.unpack_texts(modded_langs).map(|_| "game texts")
                }),
            ];
            for job in jobs {
                match job.join() {
                    Ok(Err(e)) => anyhow_ext::bail!(e),
                    Ok(Ok(what)) => log::info!("Finished unpacking {what}"),
                    Err(e) => {
                        anyhow::bail!(
                            e.downcast::<std::string::String>()
                                .or_else(|e| {
                                    e.downcast::<&'static str>().map(|s| Box::new((*s).into()))
                                })
                                .unwrap_or_else(|_| {
                                    Box::new(
                                        "An unknown error occured, check the log for possible \
                                         details."
                                            .to_string(),
                                    )
                                })
                        )
                    }
                }
            }
            log::trace!("CLEARPROGRESS");
            Ok(())
        })?;
        Ok(self.rstb)
    }

    fn unpack_texts(&self, mut langs: IndexSet<Language>) -> Result<()> {
        if !langs.is_empty() {
            log::info!("Unpacking game texts");
            let Some(MergeableResource::MessagePack(mut base)) = ResourceData::clone(
                self.dump
                    .get_data(self.lang.message_path().as_str())?
                    .deref(),
            )
            .take_mergeable() else {
                bail!("Broken stock language pack for {}", self.lang);
            };
            langs.sort_unstable_by(|l1, l2| {
                (*l1 == self.lang).cmp(&(*l2 == self.lang)).then_with(|| {
                    (l1.short() == self.lang.short()).cmp(&(l2.short() == self.lang.short()))
                })
            });
            for mod_ in self.mods.iter() {
                for lang in langs.iter() {
                    if let Ok(packs) = mod_.get_versions(lang.message_path().as_str().as_ref()) {
                        for pack in packs {
                            let Some(MergeableResource::MessagePack(version)) =
                                minicbor_ser::from_slice::<ResourceData>(&pack)?.take_mergeable()
                            else {
                                bail!("Broken mod language pack at {}", lang);
                            };
                            *base = base.merge(&version);
                        }
                        break;
                    }
                }
            }
            let out = self
                .out_dir
                .join(platform_content(self.endian))
                .join(self.lang.bootup_path().as_str());
            out.parent().map(fs::create_dir_all).transpose()?;
            let data = base.into_binary(self.endian);
            self.rstb.insert(
                format!("Message/Msg_{}.product.sarc", self.lang).into(),
                rstb::calc::calc_from_size_and_name(data.len(), "Msg.sarc", self.endian.into()),
            );
            let mut sarc = SarcWriter::new(self.endian.into())
                .with_file(self.lang.message_path(), compress(data));
            fs::write(out, sarc.to_binary())?;
        }
        Ok(())
    }

    fn unpack_files(
        &self,
        files: BTreeSet<&String>,
        dir: PathBuf,
        total_files: usize,
        current_file: &AtomicUsize,
        aoc: bool,
    ) -> Result<()> {
        files.into_par_iter().try_for_each(|file| -> Result<()> {
            let data = self.build_file(file.as_str(), aoc)?;
            let out_file = dir.join(file.as_str());
            out_file.parent().map(fs::create_dir_all).transpose()?;
            let mut writer = std::io::BufWriter::new(fs::File::create(&out_file)?);
            writer.write_all(&compress_if(data.as_ref(), &out_file))?;
            let progress = 1 + current_file.load(Ordering::Relaxed);
            current_file.store(progress, Ordering::Relaxed);
            let percent = (progress as f64 / total_files as f64) * 100.0;
            let fract = percent.fract();
            if fract <= 0.1 || fract >= 0.95 || percent == 100.0 {
                log::trace!(
                    "PROGRESSBuilding {} files: {}%",
                    total_files,
                    percent as usize
                );
            }
            Ok(())
        })
    }

    fn build_file(&self, file: &str, aoc: bool) -> Result<Vec<u8>> {
        let mut versions = std::collections::VecDeque::with_capacity(
            (self.mods.len() as f32 / 2.).ceil() as usize,
        );
        let dump_file = if aoc {
            jstr!("Aoc/0010/{file}")
        } else {
            file.to_string()
        };
        let canon = canonicalize(&dump_file);
        let filename = Path::new(canon.as_str());
        let mut rstb_val = None;
        let can_rstb = !RSTB_EXCLUDE_EXTS.contains(
            &filename
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
        ) && !RSTB_EXCLUDE_NAMES.contains(
            &filename
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default(),
        );
        let mut dump_error: Vec<anyhow_ext::Error> = vec![];
        let res_result = self.dump.get_data(&dump_file).or_else(|e| {
            log::trace!("{e:?}");
            dump_error.push(e.into());
            self.dump.get_data(file).or_else(|e| {
                log::trace!("{e:?}");
                dump_error.push(e.into());
                self.dump.get_data(canon.as_str()).or_else(|e| {
                    log::trace!("{e:?}");
                    dump_error.push(e.into());
                    self.dump.get_resource(canon.as_str())
                })
            })
        });
        match res_result {
            Ok(ref_res) => versions.push_back(ref_res),
            Err(e) => {
                log::trace!("{e}");
                dump_error.push(e.into());
            }
        }
        for (data, mod_) in self
            .mods
            .iter()
            .filter_map(|mod_| {
                mod_.get_versions(dump_file.as_ref())
                    .ok()
                    .map(|d| d.into_iter().map(|d| (d, &mod_.meta.name)))
            })
            .flatten()
        {
            let res = minicbor_ser::from_slice(&data);
            match res {
                Ok(res) => versions.push_back(Arc::new(res)),
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("unknown variant") {
                        anyhow::bail!(
                            "Error deserializing resource {canon} from mod {mod_}. This is \
                             probably because this mod was built with an old, incompatible beta \
                             of UKMM."
                        );
                    } else {
                        anyhow::bail!(
                            "Error deserializing resource {canon} from mod {mod_}. Error: {e}"
                        );
                    }
                }
            }
        }
        let base_version = versions
            .pop_front()
            .with_context(|| {
                let mut err = anyhow_ext::anyhow!(
                    "No copy of {} found in game dump or any mod. Sometimes this is a problem with the mod or\
                    your game dump. However, it can also be a temporary cache error. If you know your dump and its\
                    configuration are sound, try restarting UKMM. If it persists, copy the error details and post \
                    as a comment to https://github.com/NiceneNerd/UKMM/issues/120.", &dump_file
                );
                for e in dump_error {
                    err = e.context(err);
                }
                err
            })?;
        let is_modded = !versions.is_empty() || self.hashes.is_file_new(&canon);
        let data = match base_version.as_ref() {
            ResourceData::Binary(_) => {
                let res = versions.pop_back().unwrap_or(base_version);
                if can_rstb && is_modded {
                    rstb_val = Some(rstb::calc::estimate_from_slice_and_name(
                        res.as_binary().expect("Binary"),
                        dump_file.as_ref(),
                        self.endian.into(),
                    ));
                }
                match Arc::try_unwrap(res) {
                    Ok(res) => res.take_binary().unwrap(),
                    Err(res) => res.as_binary().map(|b| b.to_vec()).unwrap(),
                }
            }
            ResourceData::Mergeable(base_res) => {
                let merged = versions
                    .into_iter()
                    .fold(base_res.clone(), |mut res, version| {
                        if let Some(mergeable) = version.as_mergeable() {
                            res = res.merge(mergeable);
                        }
                        res
                    });
                let data = merged.into_binary(self.endian);
                if can_rstb && (is_modded || self.hashes.is_file_modded(&canon, &data, true)) {
                    rstb_val = Some(rstb::calc::estimate_from_slice_and_name(
                        &data,
                        &canon,
                        self.endian.into(),
                    ));
                    if canon.ends_with("bphysics") || self.endian == Endian::Little {
                        rstb_val = rstb_val.map(|v| v.map(|v| (v as f32 * 1.25) as u32));
                    }
                }
                data
            }
            ResourceData::Sarc(base_sarc) => {
                let merged = versions
                    .into_iter()
                    .fold(base_sarc.clone(), |mut res, version| {
                        if let Some(sarc) = version.as_sarc() {
                            res = res.merge(sarc);
                        }
                        res
                    });
                let data = self
                    .build_sarc(merged, aoc)
                    .with_context(|| jstr!("Failed to build SARC file {&file}"))?;
                if can_rstb {
                    rstb_val = Some(rstb::calc::calc_from_size_and_name(
                        data.len(),
                        &canon,
                        self.endian.into(),
                    ));
                }
                data
            }
        };
        if let Some(val) = rstb_val {
            self.rstb.insert(canon, val);
        }
        Ok(data)
    }

    fn build_sarc(&self, sarc: SarcMap, aoc: bool) -> Result<Vec<u8>> {
        let mut writer = SarcWriter::new(self.endian.into()).with_min_alignment(sarc.alignment);
        for file in sarc.files.into_iter() {
            let data = self
                .build_file(&file, aoc)
                .with_context(|| jstr!("Failed to build file {&file} for SARC"))?;
            writer.add_file(
                file.as_str(),
                compress_if(data.as_ref(), file.as_str()).as_ref(),
            );
        }
        Ok(writer.to_binary())
    }
}

/// Extract a zipped mod, decompressing the binary files, but otherwise
/// leaving the format intact.
pub fn unzip_mod(mod_path: &Path, out_path: &Path) -> anyhow_ext::Result<()> {
    let mut zip = zip::ZipArchive::new(BufReader::new(fs::File::open(mod_path)?))
        .context("Failed to open mod ZIP")?;
    zip.extract(out_path)?;
    WalkDir::new(out_path)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|f| {
            f.file_type.is_file() && {
                f.file_name()
                    .to_str()
                    .map(|n| !n.ends_with(".yml") && !n.starts_with("thumb"))
                    .unwrap_or(true)
            }
        })
        .par_bridge()
        .try_for_each(|f| -> anyhow_ext::Result<()> {
            let f = f.path();
            let data = zstd::decode_all(
                fs::read(&f)
                    .with_context(|| format!("Failed to read file at {}", f.display()))?
                    .as_slice(),
            )
            .with_context(|| format!("Failed to decompress file at {}", f.display()))?;
            fs::write(&f, data)
                .with_context(|| format!("Failed to write unpacked file at {}", f.display()))?;
            Ok(())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn read_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip", vec![]).unwrap();
        dbg!(&mod_reader.manifest);
    }

    #[test]
    fn unpack_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip", vec![]).unwrap();
        let dump = serde_yaml::from_str::<ResourceReader>(
            &std::fs::read_to_string("../.vscode/dump.yml").unwrap(),
        )
        .unwrap();
        ModUnpacker::new(
            Arc::new(dump),
            Endian::Big,
            Language::USen,
            vec![mod_reader],
            "test/wiiu_unpack".into(),
        )
        .unpack()
        .unwrap();
    }

    #[test]
    fn unzip_mod() {
        let mod_path = "test/wiiu.zip";
        let out_path = "test/wiiu_unzip";
        super::unzip_mod(mod_path.as_ref(), out_path.as_ref()).unwrap();
    }
}

#[cfg(test)]
mod bonus {
    use std::{
        path::Path,
        sync::{Arc, Mutex},
    };

    use path_slash::PathExt;
    use rayon::prelude::*;
    use roead::sarc::Sarc;
    use smartstring::alias::String;
    use uk_content::{resource::MergeableResource, util::HashMap};

    #[test]
    fn nest_map() {
        let base =
            Path::new("/media/mrm/Data/Games/Cemu/mlc01/usr/title/00050000/101C9400/content");
        let update =
            Path::new("/media/mrm/Data/Games/Cemu/mlc01/usr/title/0005000E/101C9400/content");
        let dlc =
            Path::new("/media/mrm/Data/Games/Cemu/mlc01/usr/title/0005000C/101C9400/content/0010");

        let nest_map = Arc::new(Mutex::new(HashMap::<String, String>::default()));

        fn get_sarc_paths(sarc: &Sarc, path: &str, map: Arc<Mutex<HashMap<String, String>>>) {
            sarc.files().for_each(|file| {
                if let Some(name) = file.name {
                    let full_path = String::from(path) + "//" + name;
                    if file.data.len() > 0x40
                        && file.is_sarc()
                        && !matches!(
                            Path::new(file.unwrap_name())
                                .extension()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap_or_default(),
                            "sblarc"
                                | "ssarc"
                                | "sstera"
                                | "sstats"
                                | "sarc"
                                | "stera"
                                | "stats"
                                | "blarc"
                        )
                    {
                        let sarc = Sarc::new(file.data).unwrap();
                        get_sarc_paths(&sarc, &full_path, map.clone());
                    }
                    map.lock().unwrap().insert(
                        if path.starts_with("Aoc") {
                            String::from("Aoc/0010/") + &name.replace(".s", ".")
                        } else {
                            name.replace(".s", ".").into()
                        },
                        full_path,
                    );
                }
            });
        }

        for root in [base, update] {
            jwalk::WalkDir::new(root)
                .into_iter()
                .par_bridge()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let ext = p
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default();
                    botw_utils::extensions::SARC_EXTS.contains(&ext)
                        && !matches!(ext, "ssarc" | "sstera" | "sstats")
                })
                .for_each(|path| {
                    let sarc = Sarc::new(fs_err::read(&path).unwrap()).unwrap();
                    get_sarc_paths(
                        &sarc,
                        &path.strip_prefix(root).unwrap().to_slash_lossy(),
                        nest_map.clone(),
                    );
                });
        }
        jwalk::WalkDir::new(dlc)
            .into_iter()
            .par_bridge()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                let ext = p
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default();
                botw_utils::extensions::SARC_EXTS.contains(&ext)
                    && !matches!(ext, "ssarc" | "sstera" | "sstats")
            })
            .for_each(|path| {
                let sarc = Sarc::new(fs_err::read(&path).unwrap()).unwrap();
                get_sarc_paths(
                    &sarc,
                    &path.strip_prefix(dlc).unwrap().to_slash_lossy(),
                    nest_map.clone(),
                );
            });

        fs_err::write(
            "nest_map.json",
            serde_json::to_string_pretty(
                &(Arc::try_unwrap(nest_map).unwrap().into_inner().unwrap()),
            )
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn dump_cache() {
        let base = Path::new(r"D:\Cemu\mlc01\usr\title\00050000\101C9400\content");
        let update = Path::new(r"D:\Cemu\mlc01\usr\title\0005000E\101C9400\content");
        let dlc = Path::new(r"D:\Cemu\mlc01\usr\title\0005000C\101C9400\content\0010");

        let res_map = Arc::new(Mutex::new(HashMap::<String, MergeableResource>::default()));

        fn process_sarc_resources(
            sarc: &Sarc,
            path: &str,
            map: Arc<Mutex<HashMap<String, MergeableResource>>>,
        ) {
            sarc.files().for_each(|file| {
                if let Some(name) = file.name {
                    if let Some(res) =
                        MergeableResource::from_binary(name.as_ref(), file.data).unwrap()
                    {
                        let canon = if path.starts_with("Aoc") {
                            String::from("Aoc/0010/") + &name.replace(".s", ".")
                        } else {
                            name.replace(".s", ".").into()
                        };
                        let mut map = map.lock().unwrap();
                        map.insert(canon, res);
                    } else if file.data.len() > 0x40
                        && file.is_sarc()
                        && !matches!(
                            Path::new(name)
                                .extension()
                                .unwrap_or_default()
                                .to_str()
                                .unwrap_or_default(),
                            "sblarc"
                                | "ssarc"
                                | "sstera"
                                | "sstats"
                                | "sarc"
                                | "stera"
                                | "stats"
                                | "blarc"
                        )
                    {
                        let sarc = Sarc::new(file.data).unwrap();
                        process_sarc_resources(&sarc, path, map.clone())
                    }
                }
            });
        }

        for root in [base, update] {
            jwalk::WalkDir::new(root)
                .into_iter()
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let ext = p
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default();
                    botw_utils::extensions::SARC_EXTS.contains(&ext)
                        && !matches!(ext, "ssarc" | "sstera" | "sstats")
                })
                .for_each(|path| {
                    let sarc = Sarc::new(fs_err::read(&path).unwrap()).unwrap();
                    process_sarc_resources(
                        &sarc,
                        &path.strip_prefix(root).unwrap().to_slash_lossy(),
                        res_map.clone(),
                    );
                });
        }
        jwalk::WalkDir::new(dlc)
            .into_iter()
            .collect::<Vec<_>>()
            .into_par_iter()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                let ext = p
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default();
                botw_utils::extensions::SARC_EXTS.contains(&ext)
                    && !matches!(ext, "ssarc" | "sstera" | "sstats")
            })
            .for_each(|path| {
                let sarc = Sarc::new(fs_err::read(&path).unwrap()).unwrap();
                process_sarc_resources(
                    &sarc,
                    &path.strip_prefix(dlc).unwrap().to_slash_lossy(),
                    res_map.clone(),
                );
            });

        fs_err::write(
            "dump.bin",
            zstd::encode_all(
                minicbor_ser::to_vec(&Arc::try_unwrap(res_map).unwrap().into_inner().unwrap())
                    .unwrap()
                    .as_slice(),
                0,
            )
            .unwrap(),
        )
        .unwrap();
    }
}
