use anyhow::Result;
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    prelude::MergeableImpl,
    resource::{EventInfo, MergeableResource},
    util::converts::FromByml,
};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_events(&self) -> Result<()> {
        let events_path = self.current_root.join("logs/eventinfo.yml");
        if events_path.exists() {
            log::debug!("Processing eventinfo log");
            let diff = EventInfo::from_byml(&Byml::from_text(fs::read_to_string(events_path)?)?)?;
            let base = self.dump.get_from_sarc(
                "Events/EventInfo.product.byml",
                "Pack/Bootup.pack//Event/EventInfo.product.sbyml",
            )?;
            if let Some(MergeableResource::EventInfo(base)) = base.as_mergeable() {
                let events = base.merge(&diff);
                self.inject_into_sarc(
                    "Pack/Bootup.pack//Event/EventInfo.product.sbyml",
                    MergeableResource::EventInfo(Box::new(events))
                        .into_binary(self.platform.into()),
                    false,
                )?;
            }
        }
        Ok(())
    }
}
