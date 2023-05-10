use anyhow::{Context, Result};
use fs_err as fs;
use roead::byml::{Byml, Hash};
use uk_content::{
    data::gamedata::{FlagData, GameData},
    prelude::Resource,
    resource::{GameDataPack, MergeableResource},
};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_gamedata(&self) -> Result<()> {
        let gamedata_path = self.current_root.join("logs/gamedata.yml");
        if gamedata_path.exists() {
            log::debug!("Processing gamedata log");
            let diff = Byml::from_text(fs::read_to_string(gamedata_path)?)?.into_hash()?;
            let base = self
                .dump
                .get_from_sarc(
                    "GameData/gamedata.sarc",
                    "Pack/Bootup.pack//GameData/gamedata.ssarc",
                )
                .context("Failed to parse gamedata pack from game dump")?;
            if let Some(MergeableResource::GameDataPack(mut base)) = base.as_mergeable().cloned() {
                fn simple_add(base: &mut GameData, diff: &Hash) -> Result<()> {
                    if let Some(Byml::Hash(add)) = diff.get("add") {
                        base.flags.extend(add.iter().filter_map(|(name, flag)| {
                            flag.try_into().ok().map(|f| (name.clone(), f))
                        }));
                    }
                    if let Some(Byml::Array(del)) = diff.get("del") {
                        for name in del {
                            base.flags.set_delete(name.as_string()?);
                        }
                    }
                    Ok(())
                }

                for (base, data_type) in [
                    (&mut base.bool_array_data, "bool_array_data"),
                    (&mut base.f32_array_data, "f32_array_data"),
                    (&mut base.f32_data, "f32_data"),
                    (&mut base.s32_array_data, "s32_array_data"),
                    (&mut base.string64_array_data, "string64_array_data"),
                    (&mut base.string64_data, "string64_data"),
                    (&mut base.string256_array_data, "string256_array_data"),
                    (&mut base.string256_data, "string256_data"),
                    (&mut base.vector2f_array_data, "vector2f_array_data"),
                    (&mut base.vector2f_data, "vector2f_data"),
                    (&mut base.vector3f_array_data, "vector3f_array_data"),
                    (&mut base.vector3f_data, "vector3f_data"),
                    (&mut base.vector4f_data, "vector4f_data"),
                    (&mut base.string32_data, "string_data"),
                ] {
                    if let Some(Byml::Hash(diff)) = diff.get(data_type) {
                        simple_add(base, diff)?;
                    }
                }

                for (base, revival_base, data_type) in [
                    (
                        &mut base.bool_data,
                        &mut base.revival_bool_data,
                        "bool_data",
                    ),
                    (&mut base.s32_data, &mut base.revival_s32_data, "s32_data"),
                ] {
                    if let Some(Byml::Hash(diff)) = diff.get(data_type) {
                        if let Some(Byml::Hash(add)) = diff.get("add") {
                            for (name, flag) in add.iter() {
                                let mut parts = name.split('_');
                                let flag = FlagData::try_from(flag).with_context(|| {
                                    format!(
                                        "Failed to parse gamedata flag from BNP log: {:?}",
                                        flag
                                    )
                                })?;
                                if GameDataPack::STAGES.contains(&parts.next().unwrap_or(""))
                                    && !name.contains("HiddenKorok")
                                {
                                    revival_base.flags.insert(flag.data_name.clone(), flag);
                                } else {
                                    base.flags.insert(flag.data_name.clone(), flag);
                                }
                            }
                        }
                        if let Some(Byml::Array(del)) = diff.get("del") {
                            for name in del {
                                let name = name.as_string()?;
                                base.flags.set_delete(name);
                                revival_base.flags.set_delete(name);
                            }
                        }
                    }
                }

                self.inject_into_sarc(
                    "Pack/Bootup.pack//GameData/gamedata.ssarc",
                    base.into_binary(self.platform.into()),
                    false,
                )?;
            }
        }
        Ok(())
    }
}
