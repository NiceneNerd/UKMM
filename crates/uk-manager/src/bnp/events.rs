use anyhow_ext::Result;
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    prelude::{Mergeable, Resource},
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
            let base =
                self.get_from_master_sarc("Pack/Bootup.pack//Event/EventInfo.product.sbyml")?;
            if let Ok(base) = EventInfo::from_binary(base) {
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
