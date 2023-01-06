use anyhow::{Context, Result};
use fs_err as fs;
use roead::byml::{Byml, Hash};
use rustc_hash::FxHashMap;
use smartstring::alias::String;

use super::BnpConverter;

fn merge_map(base: &mut Byml, diff: Byml) -> Result<()> {
    let mut diff = diff.into_hash()?;
    let base = base.as_mut_hash()?;
    if let Some(mut diff_objs) = diff.remove("Objs").and_then(|o| o.into_hash().ok())
        && let Some(Byml::Array(ref mut base_objs)) = base.get_mut("Objs")
    {
        if let Some(adds) = diff_objs.remove("add").and_then(|a| a.into_array().ok()) {
            base_objs.extend(adds);
        }
    }
    Ok(())
}

impl BnpConverter<'_> {
    fn handle_maps(&self) -> Result<()> {
        let maps_path = self.path.join("logs/map.yml");
        if maps_path.exists() {
            let diff = Byml::from_text(fs::read_to_string(maps_path)?)?.into_hash()?;
        }
        Ok(())
    }
}
