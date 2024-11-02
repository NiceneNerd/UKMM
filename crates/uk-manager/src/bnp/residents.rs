use anyhow_ext::Result;
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    actor::residents::ResidentActorData, prelude::Resource, resource::ResidentActors,
};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_residents(&self) -> Result<()> {
        let residents_path = self.current_root.join("logs/residents.yml");
        if residents_path.exists() {
            log::debug!("Processing resident actors log");
            let diff = Byml::from_text(fs::read_to_string(residents_path)?)?.into_map()?;
            let data = self.get_from_master_sarc("Pack/Bootup.pack//Actor/ResidentActors.byml")?;
            if let Ok(mut residents) = ResidentActors::from_binary(data) {
                residents
                    .0
                    .extend(diff.into_iter().filter_map(|(name, data)| {
                        ResidentActorData::try_from(data.as_map().ok()?)
                            .ok()
                            .map(|d| (name, d))
                    }));
                self.inject_into_sarc(
                    "Pack/Bootup.pack//Actor/ResidentActors.byml",
                    residents.into_binary(self.platform.into()),
                    false,
                )?;
            }
        }
        Ok(())
    }
}
