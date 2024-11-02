use anyhow_ext::{Context, Result};
use fs_err as fs;
use roead::{
    byml::{Byml, Map},
    yaz0::{compress, decompress},
};
use rustc_hash::FxHashMap;
use smartstring::alias::String;

use super::BnpConverter;

fn get_id(item: &Map) -> Result<String> {
    #[inline]
    fn key_from_coords(x: f32, y: f32, z: f32) -> String {
        format!("{}{}{}", x.ceil(), y.ceil(), z.ceil()).into()
    }

    #[inline]
    fn find_name(item: &Map) -> &str {
        item.iter()
            .find_map(|(k, v)| {
                k.to_lowercase()
                    .contains("name")
                    .then(|| v.as_string().ok().map(|v| v.as_str()))
                    .flatten()
            })
            .unwrap_or("")
    }

    let translate = item
        .get("Translate")
        .context("Mainfield static missing entry translation")?
        .as_map()?;

    Ok(key_from_coords(
        translate
            .get("X")
            .context("Translate missing X")?
            .as_float()?,
        translate
            .get("Y")
            .context("Translate missing Y")?
            .as_float()?,
        translate
            .get("Z")
            .context("Translate missing Z")?
            .as_float()?,
    ) + find_name(item))
}

impl BnpConverter {
    pub fn handle_mainfield_static(&self) -> Result<()> {
        let mstatic_path = self.current_root.join("logs/mainstatic.yml");
        if mstatic_path.exists() {
            log::debug!("Processing mainfield static log");
            let diff: FxHashMap<String, Map> = Byml::from_text(fs::read_to_string(mstatic_path)?)?
                .into_map()?
                .into_iter()
                .map(|(cat, entries)| -> Result<(String, Map)> { Ok((cat, entries.into_map()?)) })
                .collect::<Result<_>>()?;
            let mut base: FxHashMap<String, Map> = Byml::from_binary(decompress(
                self.get_master_aoc_bytes("Map/MainField/Static.smubin")?,
            )?)?
            .into_map()?
            .into_iter()
            .map(|(cat, entries)| -> Result<(String, Map)> {
                let entries = entries
                    .into_array()?
                    .into_iter()
                    .map(|entry| -> Result<(String, Byml)> {
                        Ok((get_id(entry.as_map()?)?, entry))
                    })
                    .collect::<Result<_>>()?;
                Ok((cat, entries))
            })
            .collect::<Result<_>>()?;
            for (cat, entries) in diff {
                base.get_mut(&cat)
                    .context("Base mainfield static missing category")?
                    .extend(entries.into_iter());
            }
            let output: Byml = base
                .into_iter()
                .map(|(cat, entries)| (cat, entries.into_values().collect()))
                .collect();
            let dest_path = self
                .current_root
                .join(self.aoc)
                .join("Map/MainField/Static.smubin");
            dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
            fs::write(dest_path, compress(output.to_binary(self.platform.into())))?;
        }
        Ok(())
    }
}
