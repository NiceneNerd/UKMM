use crate::{Manifest, Meta};
use anyhow::{Context, Result};
use fs_err::File;
use join_str::jstr;
use parking_lot::Mutex;
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::canonicalize;
use uk_reader::ResourceLoader;
use zip::ZipArchive;

type ZipReader = Arc<Mutex<ZipArchive<BufReader<File>>>>;

#[derive(Debug)]
pub struct ModReader {
    path: PathBuf,
    meta: Meta,
    manifest: Manifest,
    zip: ZipReader,
}

impl ResourceLoader for ModReader {
    fn file_exists(&self, name: impl AsRef<Path>) -> bool {
        let name = name.as_ref().to_string_lossy();
        self.manifest.content_files.contains(name.as_ref())
            || self.manifest.aoc_files.contains(name.as_ref())
    }

    fn get_file_data(&self, name: impl AsRef<Path>) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(name);
        Ok(self
            .zip
            .lock()
            .by_name(&canon)
            .map_err(anyhow::Error::from)?
            .bytes()
            .map(|b| b.map_err(anyhow::Error::from))
            .into_iter()
            .collect::<Result<_>>()?)
    }

    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(jstr!("Aoc/0010/{name.as_ref().to_str().unwrap()}"));
        Ok(self
            .zip
            .lock()
            .by_name(&canon)
            .map_err(anyhow::Error::from)?
            .bytes()
            .map(|b| b.map_err(anyhow::Error::from))
            .into_iter()
            .collect::<Result<_>>()?)
    }

    fn host_path(&self) -> &Path {
        &self.path
    }
}

impl ModReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut zip = ZipArchive::new(BufReader::new(File::open(&path)?))?;
        let manifest: Manifest = yaml_peg::serde::from_str(std::str::from_utf8(
            &zip.by_name("manifest.yml")
                .context("Mod missing manifest")?
                .bytes()
                .map(|b| b.map_err(anyhow::Error::from))
                .collect::<Result<Vec<u8>>>()?,
        )?)?
        .swap_remove(0);
        let meta: Meta = toml::from_slice(
            &zip.by_name("meta.toml")
                .context("Mod missing meta")?
                .bytes()
                .map(|b| b.map_err(anyhow::Error::from))
                .collect::<Result<Vec<u8>>>()?,
        )?;
        Ok(Self {
            path,
            meta,
            manifest,
            zip: Arc::new(Mutex::new(zip)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip").unwrap();
        dbg!(&mod_reader.manifest);
        dbg!(mod_reader.meta);
        dbg!(mod_reader.manifest.resources().collect::<Vec<String>>());
    }
}
