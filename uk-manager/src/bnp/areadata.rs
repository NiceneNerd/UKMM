use std::collections::BTreeMap;

use anyhow::{Context, Result};
use fs_err as fs;
use roead::{aamp::hash_name, byml::Byml, yaz0::compress};
use uk_content::{
    prelude::{Mergeable, Resource},
    resource::{AreaData, MergeableResource, ResourceData},
};

use super::BnpConverter;

impl BnpConverter<'_> {
    pub fn handle_areadata(&self) -> Result<()> {
        let path = self.path.join("logs/areadata.yml");
        if path.exists() {
            let diff =
                Byml::from_text(fs::read_to_string(path).context("Failed to read areadata log")?)
                    .context("Failed to parse areadata log")?
                    .into_hash()
                    .context("Invalid areadata log: not a map")?
                    .into_iter()
                    .map(|(h, a)| -> Result<(usize, Byml)> {
                        let hash = h.parse::<usize>()?;
                        Ok((hash, a))
                    })
                    .collect::<Result<_>>()
                    .map(AreaData)?;
            let areadata = self
                .dump()
                .context("No dump for current mode")?
                .get_from_sarc(
                    "Ecosystem/AreaData.byml",
                    "content/Pack/Bootup.pack//Ecosystem/AreaData.sbyml",
                )?;
            if let Some(MergeableResource::AreaData(data)) = areadata.as_mergeable() {
                fs::write(
                    self.path
                        .join(self.content)
                        .join("Actor/AreaData.product.sbyml"),
                    compress(
                        data.merge(&diff)
                            .into_binary(self.core.settings().current_mode.into()),
                    ),
                )?;
            }
        }
        Ok(())
    }
}
