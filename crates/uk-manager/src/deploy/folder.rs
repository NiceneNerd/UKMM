use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use anyhow::anyhow;
use anyhow_ext::{Result, Error, Context};
use rayon::prelude::*;
use smartstring::alias::String;
use serde::{Deserialize, Serialize};
use crate::deploy::file::File;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Folder {
    folders: BTreeMap<String, Folder>,
    files: BTreeSet<File>,
}

impl TryFrom<&PathBuf> for Folder {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        if path.is_file() {
            Err(anyhow!("{:?} is not a valid folder", path))
        }
        else {
            let mut folders: BTreeMap<String, Folder> = BTreeMap::new();
            let mut files: BTreeSet<File> = BTreeSet::new();
            for entry in path.as_path().read_dir()? {
                let p = &entry?.path();
                if p.is_dir() {
                    folders.insert(
                        p.file_name()
                            .ok_or(anyhow!("{} has no file name?", p.display()))?
                            .to_string_lossy()
                            .into(),
                        p.try_into()?
                    );
                }
                else {
                    files.insert(p.try_into()?);
                }
            }
            Ok(Self { folders, files })
        }
    }
}

impl Folder {
    #[inline(always)]
    pub fn has_some(&self) -> bool {
        !self.files.is_empty() || !self.folders.is_empty()
    }

    #[inline(always)]
    pub fn contains_folder(&self, name: &str) -> bool {
        self.folders.contains_key(name)
    }

    #[inline(always)]
    pub fn insert_folder(&mut self, name: String) {
        self.folders.insert(name, Folder::default());
    }

    #[inline(always)]
    pub fn get_folder_mut(&mut self, name: &str) -> Option<&mut Folder> {
        self.folders.get_mut(name)
    }

    pub fn extend(&mut self, other: BTreeSet<String>) -> Result<()> {
        for path in other.into_iter() {
            self.extend_iter(PathBuf::from(path.as_str()).iter())?;
        }
        Ok(())
    }

    pub fn extend_iter(&mut self, mut iter: std::path::Iter) -> Result<()> {
        let name: String = iter.next()
            .ok_or_else(|| anyhow!("{:?} is empty?", iter))?
            .to_string_lossy()
            .into();
        if name.contains('.') {
            self.files.insert(name.into());
        }
        else {
            if !self.folders.contains_key(&name) {
                self.folders.insert(name.clone(), Folder::default());
            }
            self.folders.get_mut(&name)
                .unwrap_or_else(|| unsafe { std::hint::unreachable_unchecked() })
                .extend_iter(iter)?;
        }
        Ok(())
    }

    pub fn compile_moves(from: &PathBuf, to: &PathBuf) -> Result<Self> {
        let mut folders: BTreeMap<String, Folder> = BTreeMap::new();
        let mut files: BTreeSet<File> = BTreeSet::new();
        if from.exists() {
            from.read_dir()?.try_for_each(|f| -> Result<()> {
                let from_path = f?.path();
                if from_path.is_file() {
                    let file: File = (&from_path).try_into().context("Could not create File")?;
                    if file.should_move(from, to)
                        .with_context(|| format!(
                            "Failed to determine if {:?} should move from {:?} to {:?}",
                            file.name(),
                            from,
                            to
                        ))? {
                        files.insert(file);
                    }
                }
                else if from_path.is_dir() {
                    let folder_name = from_path.file_name()
                        .context("Folder should have name")?;
                    let to_path = to.join(&folder_name);
                    let folder: Folder = Self::compile_moves(&from_path, &to_path)
                        .with_context(|| format!(
                            "Failed to compile moves from {:?} to {:?}",
                            from_path,
                            to_path
                        ))?;
                    if folder.has_some() {
                        folders.insert(folder_name.to_string_lossy().into(), folder);
                    }
                }
                Ok(())
            })?;
        }
        Ok(Self { folders, files })
    }

    pub fn compile_deletes(from: &PathBuf, based_on: &PathBuf) -> Result<Self> {
        let mut folders: BTreeMap<String, Folder> = BTreeMap::new();
        let mut files: BTreeSet<File> = BTreeSet::new();
        if from.exists() {
            from.read_dir()?.try_for_each(|f| -> Result<()> {
                let from_path = f?.path();
                if from_path.is_file() {
                    let file: File = (&from_path).try_into().context("Could not create File")?;
                    if file.should_delete(from, based_on)
                        .with_context(|| format!(
                            "Failed to determine if {:?} should be deleted from {:?} based on {:?}",
                            file.name(),
                            from,
                            based_on
                        ))? {
                        files.insert(file);
                    }
                }
                else if from_path.is_dir() {
                    let folder_name = from_path.file_name()
                        .context("Folder should have name")?;
                    let based_on_path = based_on.join(&folder_name);
                    let folder: Folder = Self::compile_deletes(&from_path, &based_on_path)
                        .with_context(|| format!(
                            "Failed to compile deletes from {:?} based on {:?}",
                            from_path,
                            based_on_path
                        ))?;
                    if folder.has_some() {
                        folders.insert(folder_name.to_string_lossy().into(), folder);
                    }
                }
                Ok(())
            })?;
        }
        Ok(Self { folders, files })
    }

    pub fn copy(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        self.files.par_iter().try_for_each(|file| -> Result<()> {
            file.copy(from, to)
        })?;
        self.folders.par_iter().try_for_each(|(folder_name, folder)| -> Result<()> {
            let new_path = to.join(folder_name.as_str());
            if !new_path.exists() {
                std::fs::create_dir(&new_path)?;
            }
            folder.copy(&from.join(folder_name.as_str()), &new_path)
        })?;
        Ok(())
    }

    pub fn hard_link(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        self.files.par_iter().try_for_each(|file| -> Result<()> {
            file.hard_link(from, to)
        })?;
        self.folders.par_iter().try_for_each(|(folder_name, folder)| -> Result<()> {
            let new_path = to.join(folder_name.as_str());
            if !new_path.exists() {
                std::fs::create_dir(&new_path)?;
            }
            folder.hard_link(&from.join(folder_name.as_str()), &new_path)
        })?;
        Ok(())
    }

    pub fn delete(&self, path: &PathBuf) -> Result<()> {
        self.files.par_iter().try_for_each(|file| -> Result<()> {
            let file_path = path.join(file.name());
            if file_path.exists() {
                std::fs::remove_file(&file_path)
                    .with_context(|| format!("Failed to delete file {:?}", file_path))?;
            }
            else {
                log::warn!("File {:?} was not found", file_path);
            }
            Ok(())
        })?;
        self.folders.par_iter().try_for_each(|(folder_name, folder)| -> Result<()> {
            let folder_path = path.join(folder_name.as_str());
            if folder_path.exists() {
                folder.delete(&folder_path)?;
                if folder_path.read_dir()?.next().is_none() {
                    std::fs::remove_dir(&folder_path)
                        .with_context(||
                            format!("Failed to remove empty folder: {}", folder_path.display())
                        )?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.files.len() + self.folders.par_iter().map(|(_, v)| v.len()).sum::<usize>()
    }
}