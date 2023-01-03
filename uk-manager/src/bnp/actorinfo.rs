use std::collections::BTreeMap;

use anyhow::{Context, Result};
use fs_err as fs;
use roead::{aamp::hash_name, byml::Byml, yaz0::compress};
use uk_content::{
    prelude::{Mergeable, Resource},
    resource::{ActorInfo, MergeableResource, ResourceData},
};

use super::BnpConverter;

impl BnpConverter<'_> {
    pub fn handle_actorinfo(&self) -> Result<()> {
        let path = self.path.join("logs/actorinfo.yml");
        if path.exists() {
            let diff =
                Byml::from_text(fs::read_to_string(path).context("Failed to read actorinfo log")?)
                    .context("Failed to parse actorinfo log")?
                    .into_hash()
                    .context("Invalid actorinfo log: not a map")?
                    .into_iter()
                    .map(|(h, a)| -> Result<(u32, Byml)> {
                        let hash = h.parse::<u32>()?;
                        Ok((hash, a))
                    })
                    .collect::<Result<_>>()
                    .map(ActorInfo)?;
            let actorinfo = self
                .core
                .settings()
                .dump()
                .context("No dump for current platform")?
                .get_resource("Actor/ActorInfo.product.sbyml")?;
            if let Some(MergeableResource::ActorInfo(info)) = actorinfo.as_mergeable() {
                fs::write(
                    self.path
                        .join(self.content)
                        .join("Actor/ActorInfo.product.sbyml"),
                    compress(
                        info.merge(&diff)
                            .into_binary(self.core.settings().current_mode.into()),
                    ),
                )?;
            }
        }
        Ok(())
    }
}
