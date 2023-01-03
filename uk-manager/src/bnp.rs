use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use roead::{aamp::ParameterIO, byml::Byml};
use tempfile::tempdir;
use uk_mod::Meta;
use uk_reader::ResourceReader;
mod actorinfo;
mod areadata;

#[derive(Debug)]
struct BnpConverter<'core> {
    core:    &'core crate::core::Manager,
    path:    PathBuf,
    content: &'static str,
    aoc:     &'static str,
}

impl BnpConverter<'_> {
    #[inline(always)]
    fn dump(&self) -> Option<Arc<ResourceReader>> {
        self.core.settings().dump()
    }

    fn convert(self) -> Result<PathBuf> {
        Ok(todo!())
    }
}

pub fn convert_bnp(core: &crate::core::Manager, path: &Path) -> Result<PathBuf> {
    let tempdir = tempdir()?.into_path();
    sevenz_rust::decompress_file(path, &tempdir).context("Failed to extract BNP")?;
    let (content, aoc) = uk_content::platform_prefixes(core.settings().current_mode.into());
    let converter = BnpConverter {
        core,
        path: tempdir,
        content,
        aoc,
    };
    converter.convert()
}
