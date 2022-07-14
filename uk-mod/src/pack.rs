use crate::{Manifest, Meta, ModOptionGroup};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use parking_lot::{Mutex, RwLock};
use path_slash::PathExt;
use rayon::prelude::*;
use roead::{sarc::Sarc, yaz0::decompress_if};
use std::{
    collections::BTreeSet,
    io::Write,
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
    current_root: PathBuf,
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

impl ModPacker {
    pub fn new(
        source: impl AsRef<Path>,
        dest: impl AsRef<Path>,
        meta: Meta,
        masters: Vec<Arc<uk_reader::ResourceReader>>,
    ) -> Result<Self> {
        let source_dir = source.as_ref().to_path_buf();
        if !(source_dir.exists() && source_dir.is_dir()) {
            anyhow::bail!("Source directory does not exist: {}", source_dir.display());
        }
        let dest_file = dest.as_ref().to_path_buf();
        if dest_file.exists() {
            fs::remove_file(&dest_file)?;
        }
        let zip = Arc::new(Mutex::new(ZipW::new(fs::File::create(&dest_file)?)));
        Ok(Self {
            current_root: source_dir.clone(),
            source_dir,
            endian: meta.platform,
            zip,
            masters,
            hash_table: get_hash_table(meta.platform),
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
        Ok(files
            .into_par_iter()
            .map(|path| -> Result<Option<String>> {
                let name = path
                    .strip_prefix(&self.current_root)
                    .unwrap()
                    .to_slash_lossy()
                    .to_string();
                // We know this is sound because we got `path` by iterating the contents of `root`.
                let canon = canonicalize(unsafe { &path.strip_prefix(root).unwrap_unchecked() });
                let file_data = fs::read(&path)?;
                let file_data = decompress_if(&file_data)
                    .with_context(|| jstr!("Failed to decompress {&name}"))?;

                if !self.hash_table.is_modified(&canon, &*file_data) {
                    return Ok(None);
                }

                let resource = ResourceData::from_binary(&name, &*file_data)
                    .with_context(|| jstr!("Failed to parse resouece {&name}"))?;
                self.process_resource(name, canon.clone(), resource, false)
                    .with_context(|| jstr!("Failed to process resouece {&canon}"))?;
                if is_mergeable_sarc(&canon, file_data.as_ref()) {
                    self.process_sarc(
                        Sarc::read(file_data.as_ref())?,
                        !self.hash_table.contains(&canon),
                    )?;
                }

                Ok(Some(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_slash_lossy()
                        .to_string(),
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
            return Ok(());
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
                    .get_resource(&canon)
                    .or_else(|_| master.get_file(ref_name))
                    .ok()
            })
            .last();
        // .fold(None, |acc, res| match (acc, res.as_ref()) {
        //     (Some(ResourceData::Mergeable(acc)), ResourceData::Mergeable(next)) => {
        //         Some(ResourceData::Mergeable(acc.merge(next)))
        //     }
        //     _ => Some((*res).clone()),
        // });
        if let Some(ref_res_data) = reference
            && let Some(ref_res) = ref_res_data.as_mergeable()
            && let ResourceData::Mergeable(res) = &resource
        {
            if ref_res == res && !in_new_sarc {
                return Ok(());
            }
            resource = ResourceData::Mergeable(ref_res.diff(res));
        }

        let data = minicbor_ser::to_vec(&resource)
            .with_context(|| jstr!("Failed to serialize {&name}"))?;
        let zip_path = self
            .current_root
            .strip_prefix(&self.source_dir)
            .unwrap()
            .join(&canon);
        {
            let mut zip = self.zip.lock();
            zip.start_file(zip_path.to_slash_lossy(), self._zip_opts)?;
            zip.write_all(&zstd::encode_all(&*data, 3)?)?;
        }
        self.built_resources.write().insert(canon);

        Ok(())
    }

    fn process_sarc(&self, sarc: Sarc, is_new_sarc: bool) -> Result<()> {
        for file in sarc.files() {
            let name = file
                .name()
                .with_context(|| jstr!("File in SARC missing name"))?;
            let canon = canonicalize(&name);
            let file_data = decompress_if(file.data())
                .with_context(|| jstr!("Failed to decompress {&name}"))?;

            if !self.hash_table.is_modified(&canon, &*file_data) && !is_new_sarc {
                continue;
            }

            let resource = ResourceData::from_binary(&name, &*file_data)
                .with_context(|| jstr!("Failed to parse resource {&canon}"))?;
            self.process_resource(name.to_owned(), canon.clone(), resource, is_new_sarc)?;
            if is_mergeable_sarc(&canon, file_data.as_ref()) {
                self.process_sarc(Sarc::read(file_data.as_ref())?, is_new_sarc)?;
            }
        }
        Ok(())
    }

    #[allow(irrefutable_let_patterns)]
    fn pack_root(&self, root: impl AsRef<Path>) -> Result<()> {
        self.built_resources.write().clear();
        let (content, aoc) = platform_prefixes(self.endian);
        let content_dir = if let content_dir = root.as_ref().join(content) && content_dir.exists() {
            Some(content_dir)
        } else {
            None
        };
        let aoc_dir = if let aoc_dir = root.as_ref().join(aoc) && aoc_dir.exists() {
            Some(aoc_dir)
        } else {
            None
        };
        let manifest = yaml_peg::serde::to_string(&Manifest {
            aoc_files: aoc_dir
                .map(|aoc| self.collect_resources(&aoc))
                .transpose()?
                .unwrap_or_default(),
            content_files: content_dir
                .map(|content| self.collect_resources(&content))
                .transpose()?
                .unwrap_or_default(),
        })?;
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
        roots
    }

    pub fn pack(mut self) -> Result<()> {
        self.pack_root(&self.source_dir)?;
        if self.source_dir.join("options").exists() {
            self.masters
                .push(Arc::new(uk_reader::ResourceReader::from_unpacked_mod(
                    &self.source_dir,
                )?));
            for root in self.collect_roots() {
                self.current_root = root.clone();
                self.pack_root(root)?;
            }
        }
        match Arc::try_unwrap(self.zip).map(|z| z.into_inner()) {
            Ok(mut zip) => {
                zip.start_file("meta.toml", self._zip_opts)?;
                zip.write_all(toml::to_string_pretty(&self.meta)?.as_bytes())?;
                zip.finish()?
            }
            Err(_) => panic!("Failed to finish writing zip, this is probably a big deal"),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ModOption, MultipleOptionGroup, OptionGroup};
    use indexmap::IndexMap;
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
                masters: IndexMap::new(),
                url: None,
                options: vec![OptionGroup::Multiple(MultipleOptionGroup {
                    name: "Test Option Group".to_string(),
                    description: "A test option group".to_string(),
                    defaults: ["option1".into()].into_iter().collect(),
                    options: [ModOption {
                        name: "Test Option".to_string(),
                        description: "An option".to_string(),
                        path: "option1".into(),
                        requires: vec![],
                    }]
                    .into_iter()
                    .collect(),
                })],
            },
            vec![Arc::new(rom_reader)],
        )
        .unwrap();
        builder.pack().unwrap();
    }
}
