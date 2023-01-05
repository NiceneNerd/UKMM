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
    yaz0::compress_if,
};
use tempfile::tempdir;
use uk_mod::Meta;
use uk_reader::ResourceReader;
mod actorinfo;
mod areadata;
mod deepmerge;

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

    fn open_or_create_sarc(&self, base_path: &Path, root_path: &str) -> Result<SarcWriter> {
        let mut sarc = SarcWriter::new(self.core.settings().current_mode.into());
        if !base_path.exists() {
            fs::write(
                base_path,
                self.dump()
                    .context("No dump for current mode")?
                    .get_bytes_uncached(root_path)?,
            )?;
        } else {
            let existing = Sarc::new(fs::read(base_path)?)?;
            sarc.files.extend(
                existing
                    .files()
                    .filter_map(|file| file.name.map(|name| (name.into(), file.data.to_vec()))),
            );
        }
        Ok(sarc)
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
        let mut sarc = self.open_or_create_sarc(&base_path, parts[0])?;
        let mut nested = None;
        if parts.len() == 3 {
            let nested_path = parts[1];
            nested = Some(SarcWriter::from_sarc(&Sarc::new(
                sarc.files.get(nested_path).context("Missing nested SARC")?,
            )?));
        }
        let parent = nested.as_mut().unwrap_or(&mut sarc);
        let dest_path = *parts.iter().last().expect("This exists");
        let data = compress_if(&data, dest_path);
        parent.files.insert(dest_path.into(), data.to_vec());
        if let Some(mut nested) = nested {
            let nested_path = parts[1];
            sarc.files.insert(
                nested_path.into(),
                compress_if(&nested.to_binary(), nested_path).to_vec(),
            );
        }
        fs::write(&base_path, compress_if(&sarc.to_binary(), &base_path))?;
        Ok(())
    }

    fn convert(self) -> Result<PathBuf> {
        println!("Actor info");
        self.handle_actorinfo()?;
        println!("Areadata");
        self.handle_areadata()?;
        println!("Deepmerge");
        self.handle_deepmerge()?;
        Ok(todo!())
    }
}

pub fn convert_bnp(core: &crate::core::Manager, path: &Path) -> Result<PathBuf> {
    let tempdir = tempdir()?.into_path();
    dbg!(&tempdir);
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

#[cfg(test)]
#[test]
fn test_convert() {
    let path = "/home/nn/Downloads/SecondWindv1.9.13.bnp";
    convert_bnp(&super::core::Manager::init().unwrap(), path.as_ref()).unwrap();
}
