mod de;
use crate::{Manifest, Meta, ModOption};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use parking_lot::Mutex;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::SarcWriter, yaz0::compress_if};
use serde::Serialize;
use smartstring::alias::String;
use std::{
    collections::BTreeSet,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{
    canonicalize, platform_prefixes,
    prelude::{Endian, Mergeable},
    resource::{ResourceData, SarcMap},
};
use uk_reader::{ResourceLoader, ResourceReader};
use zip::ZipArchive;

type ZipReader = Arc<Mutex<ZipArchive<BufReader<fs::File>>>>;

#[derive(Debug, Serialize)]
pub struct ModReader {
    pub path: PathBuf,
    options: Vec<ModOption>,
    pub meta: Meta,
    pub manifest: Manifest,
    #[serde(skip_serializing)]
    zip: Option<ZipReader>,
}

#[typetag::serde]
impl ResourceLoader for ModReader {
    fn file_exists(&self, name: &Path) -> bool {
        let name = name.to_slash_lossy();
        self.manifest.content_files.contains(name.as_ref())
            || self.manifest.aoc_files.contains(name.as_ref())
    }

    #[allow(irrefutable_let_patterns)]
    fn get_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(name);
        if let Some(zip) = self.zip.as_ref() {
            if let Ok(mut file) = zip.lock().by_name(&canon) {
                let size = file.size() as usize;
                let mut buffer = vec![0; size];
                let read = file.read(buffer.as_mut_slice())?;
                if read == size {
                    return Ok(zstd::decode_all(buffer.as_slice())?);
                }
            }
        } else if let path = self.path.join(canon.as_str()) && path.exists() {
            return Ok(fs::read(path)?);
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(canon.as_str());
            if let Some(zip) = self.zip.as_ref() {
                if let Ok(mut file) = zip.lock().by_name(path.to_slash_lossy().as_ref()) {
                    let size = file.size() as usize;
                    let mut buffer = vec![0; size];
                    let read = file.read(buffer.as_mut_slice())?;
                    if read == size {
                        return Ok(zstd::decode_all(buffer.as_slice())?);
                    }
                } 
            } else if let path = self.path.join(path) && path.exists() {
                return Ok(fs::read(path)?);
            }
        }
        Err(anyhow::anyhow!(
            "Failed to read file {} (canonical path {}) from mod",
            name.display(),
            canon
        )
        .into())
    }

    #[allow(irrefutable_let_patterns)]
    fn get_aoc_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(jstr!("Aoc/0010/{name.to_str().unwrap_or_default()}"));
        if let Some(zip) = self.zip.as_ref() {
            if let Ok(mut file) = zip.lock().by_name(&canon) {
                let size = file.size() as usize;
                let mut buffer = vec![0; size];
                let read = file.read(buffer.as_mut_slice())?;
                if read == size {
                    return Ok(zstd::decode_all(buffer.as_slice())?);
                }
            }
        } else if let path = self.path.join(canon.as_str()) && path.exists() {
            return Ok(fs::read(path)?);
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(canon.as_str());
            if let Some(zip) = self.zip.as_ref() {
                if let Ok(mut file) = zip.lock().by_name(path.to_slash_lossy().as_ref()) {
                    let size = file.size() as usize;
                    let mut buffer = vec![0; size];
                    let read = file.read(buffer.as_mut_slice())?;
                    if read == size {
                        return Ok(zstd::decode_all(buffer.as_slice())?);
                    }
                }
            }  else if let path = self.path.join(path) && path.exists() {
                return Ok(fs::read(path)?);
            }
        }
        Err(anyhow::anyhow!(
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
    pub fn open(path: impl AsRef<Path>, options: impl Into<Vec<ModOption>>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let options = options.into();
        if path.is_file() {
            Self::open_zipped(path, options)
        } else {
            Self::open_unzipped(path, options)
        }
    }

    fn open_unzipped(path: PathBuf, options: Vec<ModOption>) -> Result<Self> {
        let meta: Meta = toml::from_str(&fs::read_to_string(path.join("meta.toml"))?)?;
        let mut manifest: Manifest = serde_yaml::from_str(&fs::read_to_string(path.join("manifest.yml"))?)?;
        for option in &options {
            let opt_manifest: Manifest = serde_yaml::from_str(&fs::read_to_string(path.join(option.manifest_path()))?)?;
            manifest.content_files.extend(opt_manifest.content_files);
            manifest.aoc_files.extend(opt_manifest.aoc_files);
        }
        Ok(Self {
            path,
            options,
            meta,
            manifest,
            zip: None,
        })
    }

    fn open_zipped(path: PathBuf, options: Vec<ModOption>) -> Result<Self> {
        let mut zip = ZipArchive::new(BufReader::new(fs::File::open(&path)?))?;
        let mut buffer = vec![0; 524288]; // 512kb
        let mut read;
        let mut size;
        let meta: Meta = {
            let mut meta = zip.by_name("meta.toml").context("Mod missing meta file")?;
            size = meta.size() as usize;
            read = meta.read(buffer.as_mut_slice())?;
            if read != size {
                anyhow::bail!("Failed to read meta file from mod {}", path.display());
            }
            toml::from_slice(&buffer[..read]).context("Failed to parse meta file from mod")?
        };
        let mut manifest = {
            let mut manifest = zip
                .by_name("manifest.yml")
                .context("Mod missing manifest file")?;
            size = manifest.size() as usize;
            read = manifest.read(&mut buffer)?;
            if read != size {
                anyhow::bail!("Failed to read manifest file from mod")
            }
            serde_yaml::from_str::<Manifest>(std::str::from_utf8(&buffer[..read])?)
                .context("Failed to parse manifest file")?
        };
        for opt in &options {
            let mut opt_manifest = zip
                .by_name(opt.manifest_path().to_str().unwrap_or(""))
                .context("Mod missing option manifest file")?;
            size = opt_manifest.size() as usize;
            read = opt_manifest.read(&mut buffer)?;
            if read != size {
                anyhow::bail!("Failed to read option manifest file from mod")
            }
            let opt_manifest =
                serde_yaml::from_str::<Manifest>(std::str::from_utf8(&buffer[..read])?)
                    .context("Failed to parse option manifest file")?;
            manifest.content_files.extend(opt_manifest.content_files);
            manifest.aoc_files.extend(opt_manifest.aoc_files);
        }
        Ok(Self {
            path,
            options,
            meta,
            manifest,
            zip: Some(Arc::new(Mutex::new(zip))),
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

#[derive(Debug)]
pub struct ModUnpacker {
    dump: ResourceReader,
    manifest: Option<Manifest>,
    mods: Vec<ModReader>,
    endian: Endian,
    out_dir: PathBuf,
}

impl ModUnpacker {
    pub fn new(
        dump: ResourceReader,
        endian: Endian,
        mods: Vec<ModReader>,
        out_dir: PathBuf,
    ) -> Self {
        Self {
            dump,
            manifest: None,
            mods,
            endian,
            out_dir,
        }
    }

    pub fn with_manifest(mut self, manifest: Manifest) -> Self {
        self.manifest = Some(manifest);
        self
    }

    pub fn unpack(self) -> Result<()> {
        let content_files: BTreeSet<&String>;
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
        let (content, aoc) = platform_prefixes(self.endian);
        self.unpack_files(content_files, self.out_dir.join(content))?;
        self.unpack_files(aoc_files, self.out_dir.join(aoc))?;
        Ok(())
    }

    #[allow(irrefutable_let_patterns)]
    fn unpack_files(&self, files: BTreeSet<&String>, dir: PathBuf) -> Result<()> {
        files.into_par_iter().try_for_each(|file| -> Result<()> {
            let data = self.build_file(file.as_str())?;
            let out_file = dir.join(file.as_str());
            if let parent = out_file.parent().unwrap() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
            let mut writer = std::io::BufWriter::new(fs::File::create(&out_file)?);
            writer.write_all(&compress_if(data.as_ref(), &out_file))?;
            Ok(())
        })?;
        Ok(())
    }

    fn build_file(&self, file: &str) -> Result<Vec<u8>> {
        let mut versions = std::collections::VecDeque::with_capacity(
            (self.mods.len() as f32 / 2.).ceil() as usize,
        );
        if let Ok(ref_res) = self
            .dump
            .get_data(file)
            .or_else(|_| self.dump.get_resource(file))
        {
            versions.push_back(ref_res);
        }
        for (data, mod_) in self.mods.iter().filter_map(|mod_| {
            mod_.get_data(file.as_ref())
                .ok()
                .map(|d| (d, &mod_.meta.name))
        }) {
            versions.push_back(Arc::new(minicbor_ser::from_slice(&data).with_context(
                || jstr!(r#"Failed to parse mod resource {&file} in mod '{mod_}'"#),
            )?));
        }
        let base_version = versions
            .pop_front()
            .expect(&jstr!("No base version for file {&file}"));
        let data = match base_version.as_ref() {
            ResourceData::Binary(_) => {
                let res = versions.pop_back().unwrap_or(base_version);
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
                merged.into_binary(self.endian)
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
                self.build_sarc(merged)?
            }
        };
        Ok(data)
    }

    fn build_sarc(&self, sarc: SarcMap) -> Result<Vec<u8>> {
        let mut writer = SarcWriter::new(self.endian.into());
        for file in sarc.0.into_iter() {
            let data = self.build_file(&file)?;
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
pub fn unzip_mod(mod_path: &Path, out_path: &Path) -> anyhow::Result<()> {
    let mut zip = ZipArchive::new(BufReader::new(fs::File::open(&mod_path)?))
        .context("Failed to open mod ZIP")?;
    zip.extract(&out_path)?;
    WalkDir::new(out_path).into_iter().filter_map(std::result::Result::ok).filter(|f| {
        f.file_type.is_file() && {
            let file_name = f.file_name().to_str().unwrap();
            !file_name.ends_with(".yml") && !file_name.ends_with(".toml")
        }
    }).par_bridge().try_for_each(|f| -> anyhow::Result<()> {
        let f = f.path();
        let data = zstd::decode_all(fs::read(&f)?.as_slice())?;
        fs::write(f, data)?;
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
            dump,
            Endian::Big,
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
