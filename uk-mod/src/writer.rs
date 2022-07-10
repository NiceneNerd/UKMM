use crate::{Manifest, Meta};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use minicbor_ser::cbor::encode::Write;
use parking_lot::{Mutex, RwLock};
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::Sarc, yaz0::decompress_if};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{
    canonicalize,
    hashes::{get_hash_table, ROMHashTable},
    platform_prefixes,
    prelude::{Endian, Mergeable},
    resource::{is_mergeable_sarc, ResourceData},
};
use zip::{write::FileOptions, ZipWriter as ZipW};

pub type ZipWriter = Arc<Mutex<ZipW<fs::File>>>;

pub struct ModPacker {
    source_dir: PathBuf,
    content_dir: Option<PathBuf>,
    aoc_dir: Option<PathBuf>,
    meta: Meta,
    zip: ZipWriter,
    endian: Endian,
    built_resources: Arc<RwLock<BTreeSet<String>>>,
    masters: Vec<Arc<uk_reader::ResourceReader>>,
    hash_table: &'static ROMHashTable,
    _zip_opts: FileOptions,
    _out_file: PathBuf,
}

impl std::fmt::Debug for ModPacker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModBuilder")
            .field("source_dir", &self.source_dir)
            .field("content_dir", &self.content_dir)
            .field("aoc_dir", &self.aoc_dir)
            .field(
                "zip",
                &jstr!("zip::ZipWriter at {&self._out_file.to_string_lossy()}"),
            )
            .field("endian", &self.endian)
            .field("built_resources", &self.built_resources)
            .finish()
    }
}

impl ModPacker {
    #[allow(irrefutable_let_patterns)]
    pub fn new(
        source: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        meta: Meta,
        masters: Vec<Arc<uk_reader::ResourceReader>>,
    ) -> Result<Self> {
        let source_dir = source.as_ref().to_path_buf();
        if !source_dir.exists() {
            anyhow::bail!("Source directory does not exist: {}", source_dir.display());
        }

        let (content_u, aoc_u) = platform_prefixes(Endian::Big);
        let (content_nx, aoc_nx) = platform_prefixes(Endian::Little);
        let mut endian: Option<Endian> = None;
        let content_dir = if let content_dir = source_dir.join(content_u) && content_dir.exists() {
            endian = Some(Endian::Big);
            Some(content_dir)
        } else if let content_dir = source_dir.join(content_nx) && content_dir.exists() {
            endian = Some(Endian::Little);
            Some(content_dir)
        } else {
            None
        };
        let aoc_dir = if let aoc_dir = source_dir.join(aoc_u) && aoc_dir.exists() {
            if let Some(endian) = endian {
                if endian != Endian::Big {
                    anyhow::bail!(
                        "Content and DLC folder platforms do not match: {} and {}",
                        content_dir.unwrap().display(),
                        aoc_dir.display()
                    );
                }
            } else {
                endian = Some(Endian::Big);
            }
            Some(aoc_dir)
        } else if let aoc_dir = source_dir.join(aoc_nx) && aoc_dir.exists() {
            if let Some(endian) = endian {
                if endian != Endian::Little {
                    anyhow::bail!(
                        "Content and DLC folder platforms do not match: {} and {}",
                        content_dir.unwrap().display(),
                        aoc_dir.display()
                    );
                }
            } else {
                endian = Some(Endian::Little);
            }
            Some(aoc_dir)
        } else {
            None
        };
        if content_dir.is_none() && aoc_dir.is_none() {
            anyhow::bail!("No content or DLC folder found in {}", source_dir.display());
        }
        let dest_file = dest.as_ref().to_path_buf();
        if dest_file.exists() {
            fs::remove_file(&dest_file)?;
        }
        let zip = Arc::new(Mutex::new(ZipW::new(fs::File::create(&dest_file)?)));
        Ok(Self {
            source_dir,
            content_dir,
            aoc_dir,
            endian: endian.unwrap(),
            zip,
            masters,
            meta,
            built_resources: Arc::new(RwLock::new(BTreeSet::new())),
            hash_table: get_hash_table(endian.unwrap()),
            _zip_opts: FileOptions::default().compression_method(zip::CompressionMethod::Stored),
            _out_file: dest_file,
        })
    }

    fn collect_resources(&self, dir: impl AsRef<Path>) -> Result<BTreeSet<String>> {
        let dir = dir.as_ref();
        let files = WalkDir::new(dir)
            .into_iter()
            .filter_map(|f| {
                f.ok()
                    .and_then(|f| f.file_type().is_file().then(|| f.path()))
            })
            .collect::<Vec<PathBuf>>();
        Ok(files
            .into_par_iter()
            .map(|path| -> Result<Option<String>> {
                let name = path
                    .strip_prefix(&self.source_dir)
                    .unwrap()
                    .to_slash_lossy()
                    .to_string();
                let canon = canonicalize(&name);
                let file_data = fs::read(&path)?;
                let file_data = decompress_if(&file_data)
                    .with_context(|| jstr!("Failed to decompress {&name}"))?;

                if !self.hash_table.is_modified(&canon, &*file_data) {
                    return Ok(None);
                }

                let resource = ResourceData::from_binary(&name, &*file_data)?;
                self.process_resource(name, canon.clone(), resource)?;
                if is_mergeable_sarc(&canon, file_data.as_ref()) {
                    self.process_sarc(Sarc::read(file_data.as_ref())?)?;
                }

                Ok(Some(
                    path.strip_prefix(dir).unwrap().to_slash_lossy().to_string(),
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
    ) -> Result<()> {
        let prefixes = platform_prefixes(self.endian);
        let ref_name = name
            .trim_start_matches(prefixes.0)
            .trim_start_matches(prefixes.1)
            .trim_start_matches('/');
        let reference: Option<ResourceData> = self
            .masters
            .iter()
            .filter_map(|master| {
                master
                    .get_resource(&canon)
                    .or_else(|_| master.get_file(ref_name))
                    .ok()
            })
            .fold(None, |acc, res| match (acc, res.as_ref()) {
                (Some(ResourceData::Mergeable(acc)), ResourceData::Mergeable(next)) => {
                    Some(ResourceData::Mergeable(acc.merge(next)))
                }
                _ => Some((*res).clone()),
            });
        if let Some(ResourceData::Mergeable(ref_res)) =
            reference && let ResourceData::Mergeable(res) = &resource
        {
            resource = ResourceData::Mergeable(ref_res.diff(res));
        }

        let data = minicbor_ser::to_vec(&resource)
            .with_context(|| jstr!("Failed to serialize {&name}"))?;
        let mut zip = self.zip.lock();
        zip.start_file(&canon, self._zip_opts)?;
        zip.write_all(&zstd::encode_all(&*data, 3)?)?;
        self.built_resources.write().insert(canon);

        Ok(())
    }

    fn process_sarc(&self, sarc: Sarc) -> Result<()> {
        for file in sarc.files() {
            let name = file
                .name()
                .with_context(|| jstr!("File in SARC missing name"))?;
            let canon = canonicalize(&name);
            let file_data = decompress_if(file.data())
                .with_context(|| jstr!("Failed to decompress {&name}"))?;

            if !self.hash_table.is_modified(&canon, &*file_data) {
                continue;
            }

            let resource = ResourceData::from_binary(&name, &*file_data)?;
            self.process_resource(name.to_owned(), canon.clone(), resource)?;
            if is_mergeable_sarc(&canon, file_data.as_ref()) {
                self.process_sarc(Sarc::read(file_data.as_ref())?)?;
            }
        }
        Ok(())
    }

    pub fn pack(self) -> Result<()> {
        let manifest = yaml_peg::serde::to_string(&Manifest {
            aoc_files: self
                .aoc_dir
                .as_ref()
                .map(|aoc| self.collect_resources(&aoc))
                .transpose()?
                .unwrap_or_default(),
            content_files: self
                .content_dir
                .as_ref()
                .map(|content| self.collect_resources(&content))
                .transpose()?
                .unwrap_or_default(),
        })?;
        {
            let mut zip = self.zip.lock();
            zip.start_file("manifest.yml", self._zip_opts)?;
            zip.write_all(manifest.as_bytes())?;
            let mut meta = self.meta;
            meta.id = Some(xxhash_rust::xxh3::xxh3_64(
                jstr!("{&meta.name} === {&meta.version.to_string()}").as_bytes(),
            ));
            zip.start_file("meta.toml", self._zip_opts)?;
            zip.write_all(toml::to_string_pretty(&meta)?.as_bytes())?;
        }
        match Arc::try_unwrap(self.zip) {
            Ok(zip) => zip.into_inner().finish()?,
            Err(_) => panic!("Uh oh"),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uk_reader::ResourceReader;

    use super::*;
    #[test]
    fn pack_mod() {
        let source = Path::new("test/wiiu");
        let dest = Path::new("test/wiiu.zip");
        let rom_reader: ResourceReader =
            yaml_peg::serde::from_str(&std::fs::read_to_string("../.vscode/dump.yml").unwrap())
                .unwrap()
                .swap_remove(0);
        let builder = ModPacker::new(
            source,
            dest,
            Meta {
                platform: Endian::Big,
                name: "Test Mod".to_string(),
                version: 0.1,
                author: "Lord Caleb".to_string(),
                description: "A test mod".to_string(),
                id: None,
                masters: vec![],
                url: None,
            },
            vec![Arc::new(rom_reader)],
        )
        .unwrap();
        builder.pack().unwrap();
    }
}
