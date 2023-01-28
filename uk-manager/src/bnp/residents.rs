use anyhow::Result;
use fs_err as fs;
use roead::byml::Byml;
use uk_content::{
    actor::residents::ResidentActorData, prelude::Resource, resource::MergeableResource,
};

use super::BnpConverter;

impl BnpConverter {
    pub fn handle_residents(&self) -> Result<()> {
        let residents_path = self.path.join("logs/residents.yml");
        if residents_path.exists() {
            let diff = Byml::from_text(fs::read_to_string(residents_path)?)?.into_hash()?;
            let residents = self
                .dump
                .get_from_sarc(
                    "Actor/ResidentActors.byml",
                    "Pack/Bootup.pack//Actor/ResidentActors.byml",
                )?
                .as_mergeable()
                .cloned();
            if let Some(MergeableResource::ResidentActors(residents)) = residents {
                let mut residents = *residents;
                residents
                    .0
                    .extend(diff.into_iter().filter_map(|(name, data)| {
                        ResidentActorData::try_from(data.as_hash().ok()?)
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
