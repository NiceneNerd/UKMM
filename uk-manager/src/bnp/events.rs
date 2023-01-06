use anyhow::{Context, Result};
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    prelude::Mergeable,
    resource::{EventInfo, MergeableResource, ResourceData},
    util::converts::FromByml,
};

use super::BnpConverter;

impl BnpConverter<'_> {
    pub fn handle_events(&self) -> Result<()> {
        let events_path = self.path.join("logs/eventinfo.yml");
        if events_path.exists() {
            let diff = EventInfo::from_byml(&Byml::from_text(fs::read_to_string(events_path)?)?)?;
            let base = self
                .dump()
                .context("No dump for current mode")?
                .get_from_sarc(
                    "Events/EventInfo.product.byml",
                    "Bootup.pack//Events/Event/EventInfo.product.sbyml",
                )?;
            if let Some(MergeableResource::EventInfo(base)) = base.as_mergeable() {
                let events = base.merge(&diff);
                self.inject_into_sarc(
                    "Pack/Bootup.pack//Events/EventInfo.product.sbyml",
                    MergeableResource::EventInfo(Box::new(events))
                        .into_binary(self.platform.into()),
                    false,
                )?;
            }
        }
        Ok(())
    }
}
