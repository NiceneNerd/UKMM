use super::Message;
use crate::{core::Manager, mods::Mod};
use anyhow::{Context, Result};
use fs_err as fs;
use im::Vector;
use std::{io::BufReader, path::Path};
use uk_mod::{unpack::ModReader, Manifest};

fn is_probably_a_mod(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default();
    if ext != "zip" && ext != "7z" {
        false
    } else if ext == "7z" {
        true
    } else {
        match fs::File::open(path)
            .context("")
            .and_then(|f| zip::ZipArchive::new(BufReader::new(f)).context(""))
        {
            Ok(zip) => zip.file_names().any(|n| {
                [
                    "content",
                    "aoc",
                    "romfs",
                    "RomFS",
                    "atmosphere",
                    "contents",
                    "01007EF00011E000",
                    "01007EF00011F001",
                    "BreathOfTheWild",
                ]
                .into_iter()
                .any(|root| n.starts_with(root))
            }),
            Err(_) => false,
        }
    }
}

pub fn open_mod(path: &Path) -> Result<Message> {
    log::info!("Opening mod at {}", path.display());
    let mod_ = match ModReader::open(path, vec![]) {
        Ok(reader) => Mod::from_reader(reader),
        Err(err) => {
            log::warn!("Could not open mod, let's find out why");
            let err_msg = err.to_string();
            if (err_msg.contains("meta file") || err_msg.contains("invalid Zip"))
                && is_probably_a_mod(path)
            {
                log::info!("Maybe it's not a UKMM mod, let's to convert it");
                let converted_path = crate::mods::convert_gfx(path)?;
                Mod::from_reader(
                    ModReader::open(&converted_path, vec![])
                        .context("Failed to open converted mod")?,
                )
            } else {
                return Err(err.context("Failed to open mod"));
            }
        }
    };
    Ok(Message::HandleMod(mod_))
}

pub fn apply_changes(core: &Manager, mods: Vector<Mod>, dirty: Manifest) -> Result<Message> {
    let mod_manager = core.mod_manager();
    log::info!("Applying pending changes to mod configuration");
    log::info!("Updating mod states");
    mods.iter()
        .try_for_each(|m| -> Result<()> {
            let mod_ = mod_manager
                .all_mods()
                .find(|m2| m2.hash == m.hash)
                .unwrap()
                .clone();
            if !mod_.state_eq(m) {
                mod_manager
                    .set_enabled(m.hash, m.enabled)
                    .with_context(|| {
                        format!(
                            "Failed to {} {}",
                            if m.enabled { "enable" } else { "disable" },
                            m.meta.name.as_str()
                        )
                    })?;
                mod_manager
                    .set_enabled_options(m.hash, m.enabled_options.clone())
                    .with_context(|| {
                        format!("Failed to update options on {}", m.meta.name.as_str())
                    })?;
            }
            Ok(())
        })
        .context("Failed to update mod state")?;
    log::info!("Updating load order");
    let order = mods.iter().map(|m| m.hash).collect();
    mod_manager.set_order(order);
    mod_manager
        .save()
        .context("Failed to save mod configuration for current profile")?;
    log::info!("Applying changes");
    let deploy_manager = core.deploy_manager();
    deploy_manager
        .apply(Some(dirty))
        .context("Failed to apply pending mod changes")?;
    Ok(Message::ClearChanges)
}
