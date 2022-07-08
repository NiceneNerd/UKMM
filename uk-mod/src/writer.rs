use crate::{Manifest, Meta};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::yaz0::decompress_if;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};
use uk_content::{
    canonicalize,
    hashes::get_hash_table,
    platform_prefixes,
    prelude::Endian,
    resource::{ResourceData, ResourceRegister},
};

pub type TarWriter<'a> = Arc<Mutex<tar::Builder<zstd::Encoder<'a, fs::File>>>>;
struct TarManager<'a>(TarWriter<'a>, Arc<RwLock<BTreeSet<String>>>);

impl ResourceRegister for TarManager<'_> {
    fn add_resource(&self, canon: &str, resource: ResourceData) -> Result<()> {
        let data = minicbor_ser::to_vec(&resource)?;
        let mut header = tar::Header::new_gnu();
        header.set_path(&canon)?;
        header.set_size(data.len() as u64);
        header.set_mode(0o664);
        self.0
            .lock()
            .unwrap()
            .append_data(&mut header, canon, data.as_slice())?;
        Ok(())
    }

    fn contains_resource(&self, canon: &str) -> bool {
        self.1.read().unwrap().contains(canon)
    }
}

pub struct ModBuilder<'a> {
    source_dir: PathBuf,
    content_dir: Option<PathBuf>,
    aoc_dir: Option<PathBuf>,
    tar: TarWriter<'a>,
    endian: Endian,
    built_resources: Arc<RwLock<BTreeSet<String>>>,
}

impl ModBuilder<'_> {
    #[allow(irrefutable_let_patterns)]
    pub fn new(source: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<Self> {
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
        let tar = Arc::new(Mutex::new(tar::Builder::new(zstd::Encoder::new(
            fs::File::create(&dest_file)?,
            3,
        )?)));
        Ok(Self {
            source_dir,
            content_dir,
            aoc_dir,
            endian: endian.unwrap(),
            tar,
            built_resources: Arc::new(RwLock::new(BTreeSet::new())),
        })
    }

    fn collect_resources(&self, dir: impl AsRef<Path>) -> Result<BTreeSet<String>> {
        let files = WalkDir::new(dir.as_ref())
            .into_iter()
            .filter_map(|f| {
                f.ok()
                    .and_then(|f| f.file_type().is_file().then(|| f.path()))
            })
            .collect::<Vec<PathBuf>>();
        let table = get_hash_table(self.endian);
        Ok(files
            .into_par_iter()
            .map(|path| -> Result<Option<String>> {
                let manager = TarManager(self.tar.clone(), self.built_resources.clone());
                let name = path
                    .strip_prefix(&self.source_dir)
                    .unwrap()
                    .to_slash_lossy()
                    .to_string();
                let canon = canonicalize(&name);
                let file_data = fs::read(&path)?;
                let file_data = decompress_if(&file_data)?;
                if !table.is_modified(&canon, &*file_data) {
                    return Ok(None);
                }
                let resource = ResourceData::from_binary(&name, file_data, &manager)
                    .with_context(|| jstr!("Error parsing resource at {&name}"))?;
                let data = minicbor_ser::to_vec(&resource)?; //bincode::serialize(&resource)?;
                let mut header = tar::Header::new_gnu();
                header.set_path(&canon)?;
                header.set_size(data.as_slice().len() as u64);
                header.set_mode(0o664);
                self.tar
                    .lock()
                    .unwrap()
                    .append_data(&mut header, &canon, data.as_slice())?;
                self.built_resources.write().unwrap().insert(canon);
                Ok(Some(name))
            })
            .collect::<Result<Vec<Option<_>>>>()?
            .into_par_iter()
            .filter_map(|x| x)
            .collect())
    }

    pub fn build(self) -> Result<()> {
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
            let mut tar = self.tar.lock().unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o664);
            header.set_path("manifest.yml")?;
            header.set_size(manifest.as_bytes().len() as u64);
            tar.append_data(&mut header, "manifest.yml", manifest.as_bytes())?;
            let meta = toml::to_string_pretty(&Meta::default())?;
            header.set_path("meta.toml")?;
            header.set_size(meta.as_bytes().len() as u64);
            tar.append_data(&mut header, "meta.toml", meta.as_bytes())?;
        }
        match Arc::try_unwrap(self.tar) {
            Ok(tar) => tar
                .into_inner()
                .unwrap()
                .into_inner()?
                .finish()?
                .sync_all()?,
            Err(_) => panic!("Uh oh"),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uk_content::prelude::Resource;

    use super::*;
    #[test]
    fn pack_mod() {
        let source = Path::new("test/wiiu");
        let dest = Path::new("test/wiiu.tar.zst");
        let builder = ModBuilder::new(source, dest).unwrap();
        builder.build().unwrap();
    }
}
