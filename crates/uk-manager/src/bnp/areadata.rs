use anyhow_ext::{Context, Result};
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    prelude::{Mergeable, Resource},
    resource::AreaData,
};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_areadata(&self) -> Result<()> {
        let path = self.current_root.join("logs/areadata.yml");
        if path.exists() {
            log::debug!("Processing areadata log");
            let diff =
                Byml::from_text(fs::read_to_string(path).context("Failed to read areadata log")?)
                    .context("Failed to parse areadata log")?
                    .into_map()
                    .context("Invalid areadata log: not a map")?
                    .into_iter()
                    .map(|(h, a)| -> Result<(usize, Byml)> {
                        let hash = h.parse::<usize>()?;
                        Ok((hash, a))
                    })
                    .collect::<Result<_>>()
                    .map(AreaData)?;
            let areadata =
                self.get_from_master_sarc("Pack/Bootup.pack//Ecosystem/AreaData.sbyml")?;
            if let Ok(data) = AreaData::from_binary(areadata) {
                self.inject_into_sarc(
                    "Pack/Bootup.pack//Ecosystem/AreaData.sbyml",
                    data.merge(&diff).into_binary(self.platform.into()),
                    false,
                )?;
            }
        }
        Ok(())
    }
}
