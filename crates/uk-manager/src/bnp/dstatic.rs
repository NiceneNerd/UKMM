use anyhow_ext::{Context, Result};
use fs_err as fs;
use roead::{
    byml::{map, Byml},
    yaz0::{compress, decompress},
};
use rustc_hash::FxHashMap;
use smartstring::alias::String;

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_dungeon_static(&self) -> Result<()> {
        let dstatic_path = self.current_root.join("logs/dstatic.yml");
        if dstatic_path.exists() {
            log::debug!("Processing dungeon static log");
            let dstatic_diff = Byml::from_text(fs::read_to_string(&dstatic_path)?)?;
            let base = Byml::from_binary(decompress(
                self.get_master_aoc_bytes("Map/CDungeon/Static.smubin")?,
            )?)?;
            let mut dstatic: FxHashMap<String, Byml> = base
                .as_map()?
                .get("StartPos")
                .context("No StartPos")?
                .as_array()?
                .iter()
                .map(|entry| -> Result<(String, Byml)> {
                    let hash = entry.as_map()?;
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
                    .as_map()?
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
            let dstatic = map!(
                "StartPos" => dstatic.into_values().collect()
            );
            let dest_path = self
                .current_root
                .join(self.aoc)
                .join("Map/CDungeon/Static.smubin");
            dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
            fs::write(dest_path, compress(dstatic.to_binary(self.platform.into())))?;
        }
        Ok(())
    }
}
