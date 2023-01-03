use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::aamp::{ParameterIO, ParameterList, ParameterListing};
use rustc_hash::FxHashMap;

use super::BnpConverter;

type DiffMap = FxHashMap<String, DiffEntry>;

enum DiffEntry {
    Sarc(DiffMap),
    Aamp(ParameterList),
}

impl BnpConverter<'_> {
    pub fn handle_deepmerge(&self) -> Result<()> {
        let deepmerge_path = self.path.join("logs/deepmerge.aamp");
        if deepmerge_path.exists() {
            let pio = ParameterIO::from_binary(fs::read(deepmerge_path)?)?;
            let file_list = pio
                .object("FileTable")
                .context("Deepmerge log missing file table")?
                .0
                .values()
                .filter_map(|s| s.as_str().ok())
                .collect::<Vec<_>>();
        }
        Ok(())
    }
}
