use anyhow::Result;
use fs_err as fs;
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use super::{deepmerge::DiffEntry, BnpConverter};

impl BnpConverter<'_> {
    pub fn handle_packs(&self, merge_diff: &DiffEntry) -> Result<()> {
        let packs_path = self.path.join("logs/packs.json");
        if packs_path.exists() {
            let packs: FxHashMap<String, String> =
                serde_json::from_str(&fs::read_to_string(&packs_path)?)?;
            packs
                .values()
                .par_bridge()
                .try_for_each(|file| -> Result<()> {
                    let base_path = self.path.join(file);
                    let mut sarc =
                        self.open_or_create_sarc(&base_path, self.trim_prefixes(file))?;
                    Ok(())
                })?;
        }
        Ok(())
    }
}
