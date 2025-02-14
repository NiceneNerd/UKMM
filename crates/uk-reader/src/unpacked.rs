use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::{ROMError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Unpacked {
    host_path:   PathBuf,
    content_dir: Option<PathBuf>,
    update_dir:  Option<PathBuf>,
    aoc_dir:     Option<PathBuf>,
}

impl Unpacked {
    pub(crate) fn new(
        content_dir: Option<impl AsRef<Path>>,
        update_dir: Option<impl AsRef<Path>>,
        aoc_dir: Option<impl AsRef<Path>>,
        test_valid: bool,
    ) -> Result<Self> {
        log::info!("Loading unpacked game files");
        let content_dir = content_dir
            .as_ref()
            .map(|c| c.as_ref())
            .filter(|p| !p.as_os_str().is_empty());
        let update_dir = update_dir
            .as_ref()
            .map(|d| d.as_ref())
            .filter(|p| !p.as_os_str().is_empty());
        let aoc_dir = aoc_dir
            .as_ref()
            .map(|a| a.as_ref())
            .filter(|p| !p.as_os_str().is_empty());
        log::debug!(
            "Folders:\n{:?}\n{:?}\n{:?}",
            content_dir.map(|p| p.display()),
            update_dir.map(|p| p.display()),
            aoc_dir.map(|p| p.display())
        );
        if test_valid {
            static CONTENT_TEST: &str = "Map/MainField/A-1/A-1.00_Clustering.sblwp";
            static UPDATE_TEST: &str = "Actor/Pack/Enemy_Lynel_Dark.sbactorpack";
            static AOC_TEST: &str = "Pack/AocMainField.pack";
            if let Some(content_dir) = content_dir {
                if !content_dir.join(CONTENT_TEST).exists() {
                    log::error!("Test file {} not found in content folder", CONTENT_TEST);
                    return Err(ROMError::MissingDumpDir(
                        "base game",
                        content_dir.to_path_buf(),
                    ));
                }
            } else if let Some(update_dir) = update_dir {
                if !update_dir.join(UPDATE_TEST).exists() {
                    log::error!("Test file {} not found in update folder", UPDATE_TEST);
                    return Err(ROMError::MissingDumpDir("update", update_dir.to_path_buf()));
                }
            } else if let Some(aoc_dir) = aoc_dir {
                if !aoc_dir.join(AOC_TEST).exists() {
                    log::error!("Test file {} not found in DLC folder", AOC_TEST);
                    return Err(ROMError::MissingDumpDir("DLC", aoc_dir.to_path_buf()));
                }
            } else if content_dir.is_none() && update_dir.is_none() && aoc_dir.is_none() {
                return Err(ROMError::OtherMessage(
                    "No base game, update, or DLC files found",
                ));
            }
        }

        fn common_path<'a>(paths: impl Iterator<Item = &'a Path>) -> Option<PathBuf> {
            let mut path = None;
            let paths = paths.collect::<Vec<_>>();
            match paths.len() {
                0 => None,
                1 => Some(paths[0].to_path_buf()),
                _ => {
                    let first = paths[0];
                    for anc in first.ancestors() {
                        if paths.iter().all(|p| p.starts_with(anc)) {
                            path = Some(anc.to_path_buf());
                            break;
                        }
                    }
                    path
                }
            }
        }

        Ok(Self {
            host_path:   unsafe {
                common_path(
                    content_dir
                        .as_ref()
                        .iter()
                        .chain(update_dir.as_ref().iter())
                        .chain(aoc_dir.as_ref().iter())
                        .map(|d| **d),
                )
                .or_else(|| {
                    content_dir
                        .or(update_dir)
                        .or(aoc_dir)
                        .map(|d| d.to_path_buf())
                })
                // We know this is sound because we provided an infallible `or_else()`.
                .unwrap_unchecked()
            },
            content_dir: content_dir.map(|content| content.to_path_buf()),
            update_dir:  update_dir.map(|update| update.to_path_buf()),
            aoc_dir:     aoc_dir.map(|aoc| aoc.to_path_buf()),
        })
    }
}

#[typetag::serde]
impl super::ResourceLoader for Unpacked {
    fn get_data(&self, name: &Path) -> Result<Vec<u8>> {
        self.aoc_dir
            .iter()
            .chain(self.update_dir.iter())
            .chain(self.content_dir.iter())
            .map(|dir| dir.join(name))
            .find(|path| path.exists())
            .map(fs::read)
            .transpose()?
            .ok_or_else(|| {
                ROMError::FileNotFound(name.to_string_lossy().into(), self.host_path.clone())
            })
    }

    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>> {
        self.aoc_dir
            .as_ref()
            .map(|dir| {
                let dest_file = dir.join(name);
                if dest_file.exists() {
                    Ok(std::fs::read(dest_file)?)
                } else {
                    Err(ROMError::FileNotFound(
                        name.to_string_lossy().into(),
                        self.host_path.clone(),
                    ))
                }
            })
            .unwrap_or_else(|| Err(ROMError::MissingDumpDir("DLC", self.host_path.clone())))
    }

    fn file_exists(&self, name: &Path) -> bool {
        self.update_dir
            .iter()
            .chain(self.content_dir.iter())
            .chain(self.aoc_dir.iter())
            .map(|dir| dir.join(name))
            .any(|path| path.exists())
    }

    fn host_path(&self) -> &std::path::Path {
        &self.host_path
    }
}
