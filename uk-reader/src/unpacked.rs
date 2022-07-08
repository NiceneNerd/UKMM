use crate::{ROMError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Unpacked {
    host_path: PathBuf,
    content_dir: PathBuf,
    update_dir: PathBuf,
    aoc_dir: Option<PathBuf>,
}

impl Unpacked {
    pub(crate) fn new(
        content_dir: impl AsRef<Path>,
        update_dir: impl AsRef<Path>,
        aoc_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        let content_dir = content_dir.as_ref();
        let update_dir = update_dir.as_ref();
        if !content_dir
            .join("Map/MainField/A-1/A-1.00_Clustering.sblwp")
            .exists()
        {
            Err(ROMError::MissingDumpDir(
                "base game",
                content_dir.to_path_buf(),
            ))
        } else if !update_dir
            .join("Actor/Pack/Enemy_Lynel_Dark.sbactorpack")
            .exists()
        {
            Err(ROMError::MissingDumpDir("update", update_dir.to_path_buf()))
        } else if let Some(aoc_dir) =
            aoc_dir.as_ref() && !aoc_dir.as_ref().join("Pack/AocMainField.pack").exists()
        {
            Err(ROMError::MissingDumpDir(
                "DLC",
                aoc_dir.as_ref().to_path_buf(),
            ))
        } else {
            Ok(Self {
                host_path: common_path::common_path_all([content_dir, update_dir].into_iter())
                    .unwrap_or_else(|| content_dir.to_path_buf()),
                content_dir: content_dir.to_path_buf(),
                update_dir: update_dir.to_path_buf(),
                aoc_dir: aoc_dir.map(|d| d.as_ref().to_path_buf()),
            })
        }
    }
}

impl super::ResourceLoader for Unpacked {
    #[allow(irrefutable_let_patterns)]
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        let dest_file = self.update_dir.join(name.as_ref());
        if dest_file.exists() {
            Ok(std::fs::read(dest_file)?)
        } else if let dest_file = self.content_dir.join(name.as_ref()) && dest_file.exists() {
            Ok(std::fs::read(dest_file)?)
        } else {
            Err(ROMError::FileNotFound(
                name.as_ref().to_string_lossy().to_string(),
                self.host_path.to_owned(),
            ))
        }
    }

    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        self.aoc_dir
            .as_ref()
            .map(|dir| {
                let dest_file = dir.join(name.as_ref());
                if dest_file.exists() {
                    Ok(std::fs::read(dest_file)?)
                } else {
                    Err(ROMError::FileNotFound(
                        name.as_ref().to_string_lossy().to_string(),
                        self.host_path.to_owned(),
                    ))
                }
            })
            .unwrap_or_else(|| Err(ROMError::MissingDumpDir("DLC", self.host_path.to_owned())))
    }

    fn file_exists(&self, name: impl AsRef<Path>) -> bool {
        let name = name.as_ref();
        self.update_dir.join(name).exists()
            || self.content_dir.join(name).exists()
            || self
                .aoc_dir
                .as_ref()
                .map(|dir| dir.join(name).exists())
                .unwrap_or(false)
    }

    fn host_path(&self) -> &std::path::Path {
        &self.host_path
    }
}
