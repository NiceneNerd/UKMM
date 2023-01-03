use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use roead::{
    aamp::ParameterIO,
    byml::Byml,
    sarc::{Sarc, SarcWriter},
};
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

    fn inject_into_sarc(&self, nest_path: &str, data: Vec<u8>, dlc: bool) -> Result<()> {
        let parts = nest_path.split("//").collect::<Vec<_>>();
        if parts.len() < 2 {
            anyhow::bail!("Bad nested path: {}", nest_path);
        }
        let base_path = self
            .path
            .join(if dlc { self.aoc } else { self.content })
            .join(parts[0]);
        let mut sarc = SarcWriter::new(self.core.settings().current_mode.into());
        if !base_path.exists() {
            fs::write(
                &base_path,
                self.dump()
                    .context("No dump for current mode")?
                    .get_bytes_uncached(parts[0])?,
            )?;
        } else {
            let existing = Sarc::new(fs::read(&base_path)?)?;
            sarc.files.extend(
                existing
                    .files()
                    .filter_map(|file| file.name.map(|name| (name.into(), file.data.to_vec()))),
            );
        }
        todo!();
        Ok(())
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
