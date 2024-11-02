use anyhow_ext::Result;
use fs_err as fs;
use roead::{byml::Byml, yaz0::compress};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_effects(&self) -> Result<()> {
        let effects_path = self.current_root.join("logs/effects.yml");
        if effects_path.exists() {
            log::debug!("Processing status effect log");
            let mut base = Byml::from_binary(
                self.dump
                    .get_bytes_from_sarc("Pack/Bootup.pack//Ecosystem/StatusEffectList.sbyml")?,
            )?
            .into_array()?
            .remove(0)
            .into_map()?;
            let diff = Byml::from_text(fs::read_to_string(effects_path)?)?;
            base.extend(diff.into_map()?);
            self.inject_into_sarc(
                "Pack/Bootup.pack//Ecosystem/StatusEffectList.sbyml",
                compress(Byml::Array(vec![Byml::Map(base)]).to_binary(self.platform.into())),
                false,
            )?;
        }
        Ok(())
    }
}
