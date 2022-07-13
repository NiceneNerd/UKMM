use crate::{Manifest, ModOption};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use parking_lot::Mutex;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::SarcWriter, yaz0::compress_if};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{
    canonicalize, platform_prefixes,
    prelude::{Binary, Endian, Mergeable},
    resource::{ResourceData, SarcMap},
};
use uk_reader::{ResourceLoader, ResourceReader};
use zip::ZipArchive;

type ZipReader = Arc<Mutex<ZipArchive<BufReader<fs::File>>>>;

#[derive(Debug, Serialize)]
pub struct ModReader {
    path: PathBuf,
    options: Vec<ModOption>,
    manifest: Manifest,
    #[serde(skip_serializing)]
    zip: ZipReader,
}

#[typetag::serde]
impl ResourceLoader for ModReader {
    fn file_exists(&self, name: &Path) -> bool {
        let name = name.to_slash_lossy();
        self.manifest.content_files.contains(name.as_ref())
            || self.manifest.aoc_files.contains(name.as_ref())
    }

    fn get_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        if !self
            .manifest
            .content_files
            .contains(name.to_slash_lossy().as_ref())
        {
            return Err(uk_reader::ROMError::FileNotFound(
                name.to_slash_lossy().to_string(),
                self.path.clone(),
            ));
        }
        let canon = canonicalize(name);
        if let Ok(mut file) = self.zip.lock().by_name(&canon) {
            let size = file.size() as usize;
            let mut buffer = vec![0; size];
            let read = file.read(buffer.as_mut_slice())?;
            if read == size {
                return Ok(zstd::decode_all(buffer.as_slice())?);
            }
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(&canon);
            if let Ok(mut file) = self.zip.lock().by_name(path.to_slash_lossy().as_ref()) {
                let size = file.size() as usize;
                let mut buffer = vec![0; size];
                let read = file.read(buffer.as_mut_slice())?;
                if read == size {
                    return Ok(zstd::decode_all(buffer.as_slice())?);
                }
            }
        }
        Err(anyhow::anyhow!(
            "Failed to read file {} (canonical path {}) from mod",
            name.display(),
            canon
        )
        .into())
    }

    fn get_aoc_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        if !self
            .manifest
            .aoc_files
            .contains(name.to_slash_lossy().as_ref())
        {
            return Err(uk_reader::ROMError::FileNotFound(
                name.to_slash_lossy().to_string(),
                self.path.clone(),
            ));
        }
        let canon = canonicalize(jstr!("Aoc/0010/{name.to_str().unwrap_or_default()}"));
        if let Ok(mut file) = self.zip.lock().by_name(&canon) {
            let size = file.size() as usize;
            let mut buffer = vec![0; size];
            let read = file.read(buffer.as_mut_slice())?;
            if read == size {
                return Ok(zstd::decode_all(buffer.as_slice())?);
            }
        }
        for opt in &self.options {
            let path = Path::new("options").join(&opt.path).join(&canon);
            if let Ok(mut file) = self.zip.lock().by_name(path.to_slash_lossy().as_ref()) {
                let size = file.size() as usize;
                let mut buffer = vec![0; size];
                let read = file.read(buffer.as_mut_slice())?;
                if read == size {
                    return Ok(zstd::decode_all(buffer.as_slice())?);
                }
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
        let mut zip = ZipArchive::new(BufReader::new(fs::File::open(&path)?))?;
        let mut buffer = vec![0; 524288]; // 512kb
        let mut read;
        let mut size;
        let mut manifest: Manifest = {
            let mut manifest = zip
                .by_name("manifest.yml")
                .context("Mod missing manifest file")?;
            size = manifest.size() as usize;
            read = manifest.read(&mut buffer)?;
            if read != size {
                anyhow::bail!("Failed to read manifest file from mod")
            }
            yaml_peg::serde::from_str(std::str::from_utf8(&buffer[..read])?)
                .context("Failed to parse manifest file")?
                .swap_remove(0)
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
            let opt_manifest: Manifest =
                yaml_peg::serde::from_str(std::str::from_utf8(&buffer[..read])?)
                    .context("Failed to parse option manifest file")?
                    .swap_remove(0);
            manifest.content_files.extend(opt_manifest.content_files);
            manifest.aoc_files.extend(opt_manifest.aoc_files);
        }
        Ok(Self {
            path,
            options,
            manifest,
            zip: Arc::new(Mutex::new(zip)),
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

#[derive(Debug)]
pub struct ModUnpacker {
    dump: ResourceReader,
    mods: Vec<ModReader>,
    endian: Endian,
    out_dir: PathBuf,
}

impl ModUnpacker {
    pub fn unpack(self) -> Result<()> {
        let content_files: BTreeSet<&String> = self
            .mods
            .iter()
            .flat_map(|mod_| mod_.manifest.content_files.iter())
            .collect();
        let aoc_files: BTreeSet<&String> = self
            .mods
            .iter()
            .flat_map(|mod_| mod_.manifest.aoc_files.iter())
            .collect();
        let (content, aoc) = platform_prefixes(self.endian);
        self.unpack_files(content_files, self.out_dir.join(content))?;
        self.unpack_files(aoc_files, self.out_dir.join(aoc))?;
        Ok(())
    }

    #[allow(irrefutable_let_patterns)]
    fn unpack_files(&self, files: BTreeSet<&String>, dir: PathBuf) -> Result<()> {
        files.into_par_iter().try_for_each(|file| -> Result<()> {
            let data = self.build_file(file.as_str())?;
            let out_file = dir.join(file);
            if let parent = out_file.parent().unwrap() && !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            fs::write(&out_file, compress_if(data.as_ref(), &out_file))?;
            Ok(())
        })?;
        Ok(())
    }

    fn build_file(&self, file: &str) -> Result<Binary> {
        let mut versions = std::collections::VecDeque::with_capacity(
            (self.mods.len() as f32 / 2.).ceil() as usize,
        );
        if let Ok(ref_res) = self.dump.get_file(file) {
            versions.push_back((*ref_res).clone());
        }
        for (data, mod_) in self.mods.iter().filter_map(|mod_| {
            mod_.get_file_data(file.as_ref())
                .ok()
                .map(|d| (d, &mod_.path))
        }) {
            versions.push_back(minicbor_ser::from_slice(&data).with_context(|| {
                jstr!("Failed to parse mod resource {&file} in mod at {&mod_.to_string_lossy()}")
            })?);
        }
        let base_version = versions
            .pop_front()
            .expect(&jstr!("No base version for file {&file}"));
        let data: Binary = match base_version {
            ResourceData::Binary(_) => versions
                .pop_back()
                .unwrap_or(base_version)
                .take_binary()
                .unwrap(),
            ResourceData::Mergeable(base_res) => {
                let merged = versions
                    .into_iter()
                    .filter_map(ResourceData::take_mergeable)
                    .fold(base_res, |mut res, version| {
                        res = res.merge(&version);
                        res
                    });
                Binary::Bytes(merged.into_binary(self.endian))
            }
            ResourceData::Sarc(base_sarc) => {
                let merged = versions
                    .into_iter()
                    .filter_map(ResourceData::take_sarc)
                    .fold(base_sarc, |mut res, version| {
                        res = res.merge(&version);
                        res
                    });
                self.build_sarc(merged)?
            }
        };
        Ok(data)
    }

    fn build_sarc(&self, sarc: SarcMap) -> Result<Binary> {
        let mut writer = SarcWriter::new(self.endian.into());
        for (file, _) in sarc.0.into_iter() {
            let data = self.build_file(&file)?;
            writer.add_file(&file, compress_if(data.as_ref(), file.as_str()).as_ref());
        }
        Ok(Binary::Bytes(writer.to_binary()))
    }
}

#[doc(hidden)]
mod de {
    use super::*;
    use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
    use std::fmt;
    use std::path::PathBuf;

    impl<'de> Deserialize<'de> for ModReader {
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum Field {
                path,
                options,
                manifest,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            write!(f, "`path`, `options`, or `manifest`")
                        }

                        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match v {
                                "path" => Ok(Field::path),
                                "options" => Ok(Field::options),
                                "manifest" => Ok(Field::manifest),
                                _ => Err(serde::de::Error::custom(format!("unknown field: {}", v))),
                            }
                        }
                    }
                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct ModReaderVisitor;

            impl<'de> Visitor<'de> for ModReaderVisitor {
                type Value = ModReader;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "struct ModReader")
                }

                fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut path: Option<PathBuf> = None;
                    let mut options: Option<Vec<ModOption>> = None;
                    let mut manifest: Option<Manifest> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::path => {
                                path = Some(map.next_value()?);
                            }
                            Field::options => {
                                options = Some(map.next_value()?);
                            }
                            Field::manifest => {
                                manifest = Some(map.next_value()?);
                            }
                        }
                    }
                    let path = path.ok_or_else(|| serde::de::Error::missing_field("path"))?;
                    let options =
                        options.ok_or_else(|| serde::de::Error::missing_field("options"))?;
                    let manifest =
                        manifest.ok_or_else(|| serde::de::Error::missing_field("manifest"))?;
                    Ok(ModReader {
                        manifest,
                        options,
                        zip: Arc::new(Mutex::new(
                            ZipArchive::new(BufReader::new(
                                fs::File::open(&path).map_err(serde::de::Error::custom)?,
                            ))
                            .map_err(serde::de::Error::custom)?,
                        )),
                        path,
                    })
                }
            }

            const FIELDS: &[&str] = &["path", "options", "manifest"];
            deserializer.deserialize_struct("ModReader", FIELDS, ModReaderVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip", vec![]).unwrap();
        dbg!(&mod_reader.manifest);
    }

    #[test]
    fn unpack_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip", vec![]).unwrap();
        let dump: ResourceReader =
            yaml_peg::serde::from_str(&std::fs::read_to_string("../.vscode/dump.yml").unwrap())
                .unwrap()
                .swap_remove(0);
        ModUnpacker {
            dump,
            endian: Endian::Big,
            mods: vec![mod_reader],
            out_dir: "test/wiiu_unpack".into(),
        }
        .unpack()
        .unwrap();
    }
}
