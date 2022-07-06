use crate::{ROMError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct ZArchive {
    archive: zarchive::reader::ZArchiveReader,
    has_dlc: bool,
    title_suffix: String,
    path: PathBuf,
}

impl ZArchive {
    pub(crate) fn new(path: impl AsRef<Path>) -> Result<Self> {
        let archive = zarchive::reader::ZArchiveReader::open(path.as_ref())?;
        let has_dlc = archive
            .iter()?
            .any(|d| d.name().starts_with("0005000c") || d.name().starts_with("0005000C"));
        let title_suffix = archive
            .iter()?
            .next()
            .ok_or(ROMError::OtherMessage("No files in WUA"))?
            .name()
            .trim_start_matches("00050000101c")
            .trim_end_matches("_v0")
            .to_string();
        Ok(Self {
            has_dlc,
            title_suffix,
            path: path.as_ref().to_path_buf(),
            archive,
        })
    }
}

impl super::ROMReader for ZArchive {
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.archive.read_file(name.as_ref()).ok_or_else(|| {
            crate::ROMError::FileNotFound(
                name.as_ref().to_string_lossy().to_string(),
                self.path.to_owned(),
            )
        })
    }

    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        todo!()
    }

    fn file_exists(&self, name: impl AsRef<Path>) -> bool {
        todo!()
    }

    fn host_path(&self) -> &std::path::Path {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_wua() {
        use super::*;
        let arch = ZArchive::new("/data/Downloads/botw.wua").unwrap();
        dbg!(arch
            .archive
            .get_files()
            .unwrap()
            .iter()
            .take(10)
            .collect::<Vec<_>>());
        println!("{}", arch.has_dlc);
    }
}
