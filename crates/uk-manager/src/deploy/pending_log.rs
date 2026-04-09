use std::collections::BTreeSet;
use std::path::PathBuf;
use anyhow::anyhow;
use anyhow_ext::{Result, Error, Context};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_mod::Manifest;
use crate::deploy::folder::Folder;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingLog {
    pub(crate) content_copies: Folder,
    pub(crate) aoc_copies: Folder,
    pub(crate) content_deletes: Folder,
    pub(crate) aoc_deletes: Folder,
}

impl TryFrom<crate::deploy::OldPendingLog> for PendingLog {
    type Error = Error;
    fn try_from(pending: crate::deploy::OldPendingLog) -> Result<Self> {
        fn convert_set(files: BTreeSet<String>) -> Result<Folder> {
            let mut map: Folder = Default::default();
            for f in files {
                let path = PathBuf::from(f.as_str());
                let mut iter = path.iter();
                let name: String = iter.next()
                    .ok_or(anyhow!("{} is empty?", f))?
                    .to_string_lossy()
                    .into();
                if !name.contains('.') {
                    if !map.contains_folder(&name) {
                        map.insert_folder(name.clone());
                    }
                    map.get_folder_mut(&name).expect("Can't happen").extend_iter(iter)?;
                }
            }
            Ok(map)
        }

        Ok(Self {
            content_copies: convert_set(pending.files.content_files)?,
            aoc_copies: convert_set(pending.files.aoc_files)?,
            content_deletes: convert_set(pending.delete.content_files)?,
            aoc_deletes: convert_set(pending.delete.aoc_files)?,
        })
    }
}

impl TryFrom<(PathBuf, PathBuf, PathBuf, PathBuf)> for PendingLog {
    type Error = Error;

    fn try_from(value: (PathBuf, PathBuf, PathBuf, PathBuf)) -> Result<Self> {
        let (source_content, source_aoc, dest_content, dest_aoc) = value;
        Ok(PendingLog {
            content_copies: Folder::compile_moves(&source_content, &dest_content)
                .context("Failed to compile pending content moves")?,
            aoc_copies: Folder::compile_moves(&source_aoc, &dest_aoc)
                .context("Failed to compile pending aoc moves")?,
            content_deletes: Folder::compile_deletes(&dest_content, &source_content)
                .context("Failed to compile pending content deletes")?,
            aoc_deletes: Folder::compile_deletes(&dest_aoc, &source_aoc)
                .context("Failed to compile pending aoc deletes")?,
        })
    }
}

impl PendingLog {
    pub fn extend_copies(&mut self, other: &Manifest) -> Result<()> {
        self.content_copies.extend(other.content_files.clone())?;
        self.aoc_copies.extend(other.aoc_files.clone())?;
        Ok(())
    }

    pub fn extend_deletes(&mut self, other: &Manifest) -> Result<()> {
        self.content_deletes.extend(other.content_files.clone())?;
        self.aoc_deletes.extend(other.aoc_files.clone())?;
        Ok(())
    }

    #[inline(always)]
    pub fn has_some(&self) -> bool {
        self.content_copies.has_some() || self.aoc_copies.has_some() ||
            self.content_deletes.has_some() || self.aoc_deletes.has_some()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.content_copies.len() + self.aoc_copies.len() +
            self.content_deletes.len() + self.aoc_deletes.len()
    }

    pub fn clear(&mut self) {
        self.content_copies = Default::default();
        self.aoc_copies = Default::default();
        self.content_deletes = Default::default();
        self.aoc_deletes = Default::default();
    }

    pub fn add_rstb(&mut self) -> Result<()> {
        self.content_copies.extend_iter(
            PathBuf::from("System/Resource/ResourceSizeTable.product.srsizetable").iter()
        )
    }
}