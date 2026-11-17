use anyhow::{Context, Result};
use uk_manager::{core::Manager, mods::Mod};
use uk_mod::Manifest;

/// Commits the given mod states and load order (if any) and re-merges, limited
/// to `dirty` when provided. An empty `mods` leaves the mod configuration
/// untouched and just re-merges.
pub fn apply_changes(core: &Manager, mods: Vec<Mod>, dirty: Option<Manifest>) -> Result<()> {
    let mod_manager = core.mod_manager();
    log::info!("Applying pending changes to mod configuration");
    if !mods.is_empty() {
        log::info!("Updating mod states");
        mods.iter()
            .try_for_each(|m| -> Result<()> {
                let mod_ = mod_manager
                    .all_mods()
                    .find(|m2| m2.hash() == m.hash())
                    .with_context(|| format!("Mod {} is not installed", m.meta.name))?;
                if !mod_.state_eq(m) {
                    mod_manager
                        .set_enabled(m.hash(), m.enabled, None)
                        .with_context(|| {
                            format!(
                                "Failed to {} {}",
                                if m.enabled { "enable" } else { "disable" },
                                m.meta.name.as_str()
                            )
                        })?;
                    mod_manager
                        .set_enabled_options(m.hash(), m.enabled_options.clone())
                        .with_context(|| {
                            format!("Failed to update options on {}", m.meta.name.as_str())
                        })?;
                }
                Ok(())
            })
            .context("Failed to update mod state")?;
        log::info!("Updating load order");
        let order = mods.iter().map(|m| m.hash()).collect();
        mod_manager.set_order(order);
        mod_manager
            .save()
            .context("Failed to save mod configuration for current profile")?;
    }
    log::info!("Applying changes");
    let deploy_manager = core.deploy_manager();
    deploy_manager
        .apply(dirty)
        .context("Failed to apply pending mod changes")?;
    if core
        .settings()
        .platform_config()
        .and_then(|c| c.deploy_config.as_ref().map(|c| c.auto))
        .unwrap_or(false)
    {
        log::info!("Deploying changes");
        deploy_manager
            .deploy()
            .context("Failed to deploy update to merged mod(s)")?;
    }
    log::info!("Done");
    Ok(())
}
