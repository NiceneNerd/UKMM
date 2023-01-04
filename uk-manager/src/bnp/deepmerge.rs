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
            let diff = pio
                .object("FileTable")
                .context("Deepmerge log missing file table")?
                .0
                .values()
                .filter_map(|s| s.as_str().ok())
                .try_fold(
                    FxHashMap::default(),
                    |mut acc, file| -> Result<FxHashMap<String, DiffEntry>> {
                        let parts = file.split("//").collect::<Vec<_>>();
                        if parts.len() < 2 {
                            return Ok(acc);
                        }
                        let root_path = parts[0];
                        let root = acc
                            .entry(root_path.to_string())
                            .or_insert_with(|| DiffEntry::Sarc(Default::default()));
                        let parent = if parts.len() == 3 {
                            if let DiffEntry::Sarc(map) = acc
                                .entry(root_path.to_string())
                                .or_insert_with(|| DiffEntry::Sarc(Default::default()))
                            {
                                map
                            } else {
                                anyhow::bail!("Nonsense")
                            }
                        } else {
                            &mut acc
                        };
                        let plist = pio
                            .list(file)
                            .cloned()
                            .context("Missing entry in deepmerge log")?;
                        parent.insert(parts[parts.len() - 1].to_string(), DiffEntry::Aamp(plist));
                        Ok(acc)
                    },
                )?;
        }
        Ok(())
    }
}
