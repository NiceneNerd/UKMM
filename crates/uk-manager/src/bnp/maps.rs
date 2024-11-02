use std::collections::BTreeMap;

use anyhow_ext::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use rayon::prelude::*;
use roead::{
    byml::{Byml, Map},
    sarc::{Sarc, SarcWriter},
    yaz0::{compress, decompress},
};
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use split_iter::Splittable;

use super::BnpConverter;

fn merge_map(base: &mut Byml, diff: Byml) -> Result<()> {
    let mut diff = diff.into_map()?;
    let base = base.as_mut_map()?;

    fn merge_section(base: &mut Vec<Byml>, diff: &mut Map) -> Result<()> {
        let mut hashes = base
            .iter()
            .enumerate()
            .filter_map(|(i, obj)| {
                obj.as_map()
                    .ok()
                    .and_then(|h| h.get("HashId").and_then(|h| h.as_int().ok()))
                    .map(|h| (h, i))
            })
            .collect::<FxHashMap<u32, _>>();
        if let Some(Byml::Array(dels)) = diff.remove("del") {
            base.retain(|obj| {
                obj.as_map()
                    .ok()
                    .and_then(|h| h.get("HashId").map(|h| !dels.contains(h)))
                    .unwrap_or(false)
            });
            hashes.retain(|hash, _index| !dels.contains(&Byml::U32(*hash)));
        }
        if let Some(Byml::Array(adds)) = diff.remove("add") {
            base.extend(adds.into_iter().filter(|obj| {
                obj.as_map()
                    .ok()
                    .and_then(|h| {
                        h.get("HashId")
                            .and_then(|h| h.as_int().ok().map(|h| !hashes.contains_key(&h)))
                    })
                    .unwrap_or(false)
            }));
        }
        if let Some(Byml::Map(mods)) = diff.remove("mod") {
            for (hash, entry) in mods {
                let hash: u32 = hash.parse()?;
                if let Some(index) = hashes.get(&hash) {
                    base[*index] = entry;
                }
            }
        }
        Ok(())
    }

    if let (Some(Byml::Map(mut diff_objs)), Some(Byml::Array(ref mut base_objs))) =
        (diff.remove("Objs"), base.get_mut("Objs"))
    {
        merge_section(base_objs, &mut diff_objs)?;
    }
    if let (Some(Byml::Map(mut diff_rails)), Some(Byml::Array(ref mut base_rails))) =
        (diff.remove("Rails"), base.get_mut("Rails"))
    {
        merge_section(base_rails, &mut diff_rails)?;
    }
    Ok(())
}

impl BnpConverter {
    pub fn handle_maps(&self) -> Result<()> {
        let maps_path = self.current_root.join("logs/map.yml");
        if maps_path.exists() {
            log::debug!("Processing maps log");
            let diff = Byml::from_text(fs::read_to_string(maps_path)?)?.into_map()?;
            let base_pack = Sarc::new(self.get_master_aoc_bytes("Pack/AocMainField.pack")?)?;
            let mut merged_pack = SarcWriter::from_sarc(&base_pack);
            let (statics, dynamics) = diff
                .into_par_iter()
                .map(|(section, diff)| -> Result<(String, Vec<u8>)> {
                    let parts = section.split('_').collect::<Vec<_>>();
                    let path = jstr!("Map/MainField/{&parts[0]}/{&section}.smubin");
                    if !parts.len() == 2 {
                        anyhow::bail!("Bad map diff");
                    }
                    let mut base = Byml::from_binary(decompress(
                        base_pack
                            .get_data(&path)
                            .map(|d| d.to_vec())
                            .with_context(|| jstr!("AocMainField.pack missing map {&path}"))
                            .or_else(|e| {
                                self.get_master_aoc_bytes(&path)
                                    .context(e)
                                    .with_context(|| jstr!("Game dump missing map {&path}"))
                            })?,
                    )?)?;
                    merge_map(&mut base, diff)?;
                    Ok((path.into(), compress(base.to_binary(self.platform.into()))))
                })
                .collect::<Result<BTreeMap<String, Vec<u8>>>>()?
                .into_iter()
                .split(|(k, _)| k.ends_with("Dynamic.smubin"));
            merged_pack.add_files(statics);
            dynamics
                .collect::<BTreeMap<String, Vec<u8>>>()
                .into_par_iter()
                .try_for_each(|(path, data)| -> Result<()> {
                    let dest_path = self.current_root.join(self.aoc).join(path.as_str());
                    dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
                    fs::write(dest_path, data)?;
                    Ok(())
                })?;
            let dest_path = self
                .current_root
                .join(self.aoc)
                .join("Pack/AocMainField.pack");
            dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
            fs::write(dest_path, merged_pack.to_binary())?;
        }
        Ok(())
    }
}
