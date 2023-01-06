use std::collections::BTreeMap;

use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use rayon::prelude::*;
use roead::{
    byml::{Byml, Hash},
    sarc::{FileName, Sarc, SarcWriter},
    yaz0::{compress, decompress},
};
use rustc_hash::{FxHashMap, FxHashSet};
use smartstring::alias::String;
use uk_content::util::merge_byml_shallow;

use super::BnpConverter;

fn merge_map(base: &mut Byml, diff: Byml) -> Result<()> {
    let mut diff = diff.into_hash()?;
    let base = base.as_mut_hash()?;

    fn merge_section(base: &mut Vec<Byml>, diff: &mut Hash) -> Result<()> {
        let hashes = base
            .iter()
            .enumerate()
            .filter_map(|(i, obj)| {
                obj.as_hash()
                    .ok()
                    .and_then(|h| h.get("HashId").and_then(|h| h.as_u32().ok()))
                    .map(|h| (h, i))
            })
            .collect::<FxHashMap<_, _>>();
        if let Some(Byml::Array(adds)) = diff.remove("add") {
            base.extend(adds.into_iter().filter(|obj| {
                obj.as_hash()
                    .ok()
                    .and_then(|h| {
                        h.get("HashId")
                            .and_then(|h| h.as_u32().ok().map(|h| !hashes.contains_key(&h)))
                    })
                    .unwrap_or(false)
            }));
        }
        if let Some(Byml::Array(dels)) = diff.remove("del") {
            base.retain(|obj| {
                obj.as_hash()
                    .ok()
                    .and_then(|h| h.get("HashId").map(|h| !dels.contains(h)))
                    .unwrap_or(false)
            })
        }
        if let Some(Byml::Hash(mods)) = diff.remove("mod") {
            for (hash, entry) in mods {
                let hash: u32 = hash.parse()?;
                if let Some(index) = hashes.get(&hash) {
                    base[*index] = entry;
                }
            }
        }
        Ok(())
    }

    if let Some(Byml::Hash(mut diff_objs)) = diff.remove("Objs")
        && let Some(Byml::Array(ref mut base_objs)) = base.get_mut("Objs")
    {
        merge_section(base_objs, &mut diff_objs)?;
    }
    if let Some(Byml::Hash(mut diff_rails)) = diff.remove("Rails")
        && let Some(Byml::Array(ref mut base_rails)) = base.get_mut("Rails")
    {
        merge_section(base_rails, &mut diff_rails)?;
    }
    Ok(())
}

impl BnpConverter<'_> {
    pub fn handle_maps(&self) -> Result<()> {
        let maps_path = self.path.join("logs/map.yml");
        if maps_path.exists() {
            let diff = Byml::from_text(fs::read_to_string(maps_path)?)?.into_hash()?;
            let base_pack = Sarc::new(
                self.dump()
                    .context("No dump for current mode")?
                    .get_aoc_bytes_uncached("Pack/AocMainField.pack")?,
            )?;
            let mut merged_pack = SarcWriter::from_sarc(&base_pack);
            merged_pack.files.extend(
                diff.into_iter()
                    .map(|(section, diff)| -> Result<(FileName, Vec<u8>)> {
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
                                    unsafe { self.dump().unwrap_unchecked() }
                                        .get_aoc_bytes_uncached(&path)
                                        .context(e)
                                        .with_context(|| jstr!("Game dump missing map {&path}"))
                                })?,
                        )?)?;
                        merge_map(&mut base, diff)?;
                        Ok((path.into(), compress(base.to_binary(self.platform.into()))))
                    })
                    .collect::<Result<BTreeMap<FileName, Vec<u8>>>>()?,
            );
            let dest_path = self.path.join(self.aoc).join("Pack/AocMainField.pack");
            dest_path.parent().iter().try_for_each(fs::create_dir_all)?;
            fs::write(dest_path, merged_pack.to_binary())?;
        }
        Ok(())
    }
}
