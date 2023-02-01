use anyhow::{Context, Result};
use fs_err as fs;
use roead::{
    byml::Byml,
    yaz0::{compress, decompress},
};
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use uk_content::bhash;

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_dungeon_static(&self) -> Result<()> {
        let dstatic_path = self.path.join("logs/dstatic.yml");
        if dstatic_path.exists() {
            log::debug!("Processing dungeon static log");
            let dstatic_diff = Byml::from_text(fs::read_to_string(&dstatic_path)?)?;
            let base = Byml::from_binary(decompress(
                self.dump
                    .get_aoc_bytes_uncached("Map/CDungeon/Static.smubin")?,
            )?)?;
            let mut dstatic: FxHashMap<String, Byml> = base
                .as_hash()?
                .get("StartPos")
                .context("No StartPos")?
                .as_array()?
                .iter()
                .map(|entry| -> Result<(String, Byml)> {
                    let hash = entry.as_hash()?;
                    let id = hash
                        .get("Map")
                        .context("Pos missing Map")?
                        .as_string()?
                        .clone()
                        + "___"
                        + hash
                            .get("PosName")
                            .context("Pos missing PosName")?
                            .as_string()?;
                    Ok((id, entry.clone()))
                })
                .collect::<Result<_>>()?;
            dstatic.extend(
                dstatic_diff
                    .as_hash()?
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
            let dstatic = bhash!(
                "StartPos" => dstatic.into_values().collect()
            );
            let dest_path = self.path.join(self.aoc).join("Map/CDungeon/Static.smubin");
            dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
            fs::write(dest_path, compress(dstatic.to_binary(self.platform.into())))?;
        }
        Ok(())
    }
}
