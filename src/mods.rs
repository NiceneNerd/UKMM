use crate::{
    settings::{settings, Platform},
    util::HashMap,
};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::BufReader,
    path::{Path, PathBuf},
};
use uk_mod::{
    unpack::{ModReader, ModUnpacker},
    Manifest, Meta, ModOption,
};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub meta: Meta,
    pub manifest: Manifest,
    pub enabled_options: Vec<ModOption>,
    pub enabled: bool,
    pub path: PathBuf,
    hash: usize,
}

impl std::hash::Hash for Mod {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.hash)
    }
}

#[derive(Debug)]
pub struct Manager {
    dir: PathBuf,
    mods: HashMap<usize, Mod>,
    load_order: BTreeMap<usize, usize>,
}

impl Manager {
    pub fn load_from(mod_dir: &Path) -> Result<Self> {
        if !mod_dir.exists() {
            fs::create_dir_all(&mod_dir)?;
            return Ok(Self {
                dir: mod_dir.to_path_buf(),
                mods: Default::default(),
                load_order: Default::default(),
            });
        }
        let mods = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("mods.yml"))?)
            .context("Failed to parse mod database")?;
        let load_order = serde_yaml::from_str(&fs::read_to_string(mod_dir.join("load.yml"))?)
            .context("Failed to parse load order table")?;
        Ok(Self {
            dir: mod_dir.to_path_buf(),
            mods,
            load_order,
        })
    }

    pub fn load() -> Result<Self> {
        Self::load_from(&settings().mods_dir())
    }

    /// Iterate all enabled mods in load order
    pub fn mods(&self) -> impl Iterator<Item = &Mod> {
        self.load_order.values().filter_map(|h| {
            let mod_ = &self.mods[h];
            mod_.enabled.then_some(mod_)
        })
    }

    /// Iterate all mods, including disabled, in load order
    pub fn all_mods(&self) -> impl Iterator<Item = &Mod> {
        self.load_order.values().map(|h| &self.mods[h])
    }

    /// Add a mod to the list of installed mods. This function assumes that the
    /// mod at the provided path has already been validated.
    pub fn add<'a, 'b>(&'a mut self, mod_path: &'b Path) -> Result<&'a Mod> {
        let stored_path = self.dir.join(mod_path.file_name().unwrap());
        if mod_path.is_file() {
            if settings().unpack_mods {
                uk_mod::unpack::unzip_mod(mod_path, &stored_path)
                    .context("Failed to unpack mod to storage folder")?;
            } else {
                fs::copy(mod_path, &stored_path).context("Failed to copy mod to storage folder")?;
            }
        } else {
            dircpy::copy_dir(mod_path, &stored_path)
                .context("Failed to copy mod to storage folder")?;
        }
        Ok(todo!())
    }
}
