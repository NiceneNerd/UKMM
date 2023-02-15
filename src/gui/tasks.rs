use std::{
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use im::Vector;
use join_str::jstr;
use serde::Deserialize;
use uk_manager::{
    bnp::convert_bnp,
    core::Manager,
    mods::Mod,
    settings::{DeployConfig, Platform, PlatformSettings},
};
use uk_mod::{unpack::ModReader, Manifest, Meta};
use uk_reader::ResourceReader;

use super::{package::ModPackerBuilder, Message};

fn is_probably_a_mod_and_has_meta(path: &Path) -> (bool, bool) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default();
    if ext != "zip" && ext != "7z" {
        (false, false)
    } else if ext == "7z" {
        (true, false)
    } else if path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        == "rules.txt"
    {
        (true, true)
    } else {
        match fs::File::open(path)
            .context("")
            .and_then(|f| zip::ZipArchive::new(BufReader::new(f)).context(""))
        {
            Ok(zip) => {
                let is_a_mod = zip.file_names().any(|n| {
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
                });
                let has_meta = zip.file_names().any(|n| n == "rules.txt");
                (is_a_mod, has_meta)
            }
            Err(_) => (false, false),
        }
    }
}

pub fn open_mod(core: &Manager, path: &Path, meta: Option<Meta>) -> Result<Message> {
    log::info!("Opening mod at {}", path.display());
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase() == "bnp")
        .unwrap_or(false)
    {
        let mod_ = convert_bnp(core, path).context("Failed to convert BNP to UKMM mod")?;
        return Ok(Message::HandleMod(Mod::from_reader(
            ModReader::open_peek(mod_, vec![]).context("Failed to open converted mod")?,
        )));
    }
    let mod_ = match ModReader::open_peek(path, vec![]) {
        Ok(reader) => Mod::from_reader(reader),
        Err(err) => {
            log::warn!("Could not open mod, let's find out why");
            let err_msg = err.to_string();
            if (err_msg.contains("meta file")
                || err_msg.contains("meta.yml")
                || err_msg.contains("d Zip"))
                && let (is_mod, has_meta) = is_probably_a_mod_and_has_meta(path)
                && is_mod
            {
                log::info!("Maybe it's not a UKMM mod, let's to convert it");
                if !has_meta && meta.is_none() {
                    log::info!("Mod has no meta info, requesting manual input");
                    return Ok(Message::RequestMeta(path.to_path_buf()));
                }
                let converted_path = uk_manager::mods::convert_gfx(core, path, meta)?;
                Mod::from_reader(
                    ModReader::open_peek(converted_path, vec![])
                        .context("Failed to open converted mod")?,
                )
            } else {
                return Err(err.context("Failed to open mod"));
            }
        }
    };
    Ok(Message::HandleMod(mod_))
}

pub fn apply_changes(
    core: &Manager,
    mods: Vector<Mod>,
    dirty: Option<Manifest>,
) -> Result<Message> {
    let mod_manager = core.mod_manager();
    log::info!("Applying pending changes to mod configuration");
    if !mods.is_empty() {
        log::info!("Updating mod states");
        mods.iter()
            .try_for_each(|m| -> Result<()> {
                let mod_ = mod_manager
                    .all_mods()
                    .find(|m2| m2.hash() == m.hash())
                    .unwrap();
                if !mod_.state_eq(m) {
                    mod_manager
                        .set_enabled(m.hash(), m.enabled)
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
    Ok(Message::ResetMods)
}

pub fn package_mod(core: &Manager, builder: ModPackerBuilder) -> Result<Message> {
    let Some(dump) = core.settings().dump() else {
        anyhow::bail!("No dump for current platform")
    };
    uk_mod::pack::ModPacker::new(
        builder.source,
        builder.dest,
        Some(builder.meta),
        [dump].into_iter().collect(),
    )
    .context("Failed to initialize mod packager")?
    .pack()
    .context("Failed to package mod")?;
    Ok(Message::Noop)
}

#[allow(irrefutable_let_patterns)]
pub fn import_cemu_settings(core: &Manager, path: &Path) -> Result<Message> {
    let settings_path = if let path = path.with_file_name("settings.xml") && path.exists() {
        path
    } else if let path = dirs2::config_dir().expect("YIKES").join("Cemu/settings.xml") && path.exists() {
        path
    } else {
        anyhow::bail!("Could not find Cemu settings file")
    };
    let text = fs::read_to_string(settings_path).context("Failed to open Cemu settings file")?;
    let tree = roxmltree::Document::parse(&text)
        .context("Failed to parse Cemu settings file: invalid XML")?;
    let mlc_path = tree
        .descendants()
        .find_map(|n| {
            (n.tag_name().name() == "mlc_path")
                .then(|| {
                    n.text()
                        .and_then(|s| (!s.is_empty()).then(|| PathBuf::from(s)))
                })
                .flatten()
        })
        .or_else(|| {
            log::warn!("No MLC folder found in Cemu settings. Let's guess instead…");
            let path = path.with_file_name("mlc01");
            path.exists().then_some(path)
        })
        .or_else(|| {
            let path = dirs2::data_local_dir().expect("YIKES").join("Cemu/mlc01");
            path.exists().then_some(path)
        });
    let (base, update, dlc) = mlc_path
        .as_ref()
        .map(|mlc_path| {
            let title_path = mlc_path.join("usr/title");
            static REGIONS: &[&str] = &[
                "101C9400", "101c9400", "101C9500", "101c9500", "101C9300", "101c9300",
            ];
            let base_folder = REGIONS.iter().find_map(|r| {
                let path = title_path.join(jstr!("00050000/{r}/content"));
                path.exists().then_some(path)
            });
            let update_folder = REGIONS.iter().find_map(|r| {
                let path = title_path.join(jstr!("0005000E/{r}/content"));
                path.exists().then_some(path)
            });
            let dlc_folder = REGIONS.iter().find_map(|r| {
                let path = title_path.join(jstr!("0005000C/{r}/content/0010"));
                path.exists().then_some(path)
            });
            (base_folder, update_folder, dlc_folder)
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find game dump from Cemu settings"))?;
    let gfx_folder = if let path = path.with_file_name("graphicPacks") && path.exists() {
        Some(path)
    } else if let path = dirs2::data_local_dir().expect("YIKES").join("Cemu/graphicPacks") && path.exists() {
        Some(path)
    } else {
        log::warn!("Cemu graphic pack folder not found");
        None
    };
    let mut settings = core.settings_mut();
    settings.current_mode = Platform::WiiU;
    let dump = Arc::new(
        ResourceReader::from_unpacked_dirs(base, update, dlc)
            .context("Failed to validate game dump")?,
    );
    if let Some(wiiu_config) = settings.wiiu_config.as_mut() {
        wiiu_config.cemu_rules = true;
        if mlc_path.is_some() {
            wiiu_config.dump = dump;
        }
        if let Some(gfx_folder) = gfx_folder {
            let mut deploy_config = wiiu_config.deploy_config.get_or_insert_default();
            deploy_config.auto = true;
            deploy_config.output = gfx_folder.join("BreathOfTheWild_UKMM");
        }
    } else {
        settings.wiiu_config = Some(PlatformSettings {
            language: uk_manager::settings::Language::USen,
            profile: "Default".into(),
            dump,
            deploy_config: gfx_folder.map(|gfx_folder| {
                DeployConfig {
                    auto:   true,
                    method: uk_manager::settings::DeployMethod::Copy,
                    output: gfx_folder.join("BreathOfTheWild_UKMM"),
                }
            }),
            cemu_rules: true,
        })
    };
    settings.save()?;
    Ok(Message::ResetSettings)
}

#[derive(Debug, Deserialize)]
struct ChangelogResponse {
    body: String,
    name: String,
}

pub fn get_changelog(version: &str, sender: flume::Sender<Message>) {
    let url = format!("https://api.github.com/repos/NiceneNerd/ukmm/releases/tags/v{version}");
    match reqwest::blocking::Client::builder()
        .user_agent("UKMM")
        .build()
        .unwrap()
        .get(url)
        .send()
        .context("Failed to check release notes")
        .and_then(|r| {
            r.json::<ChangelogResponse>()
                .context("Failed to parse release notes")
        }) {
        Ok(log) => {
            sender
                .send(Message::SetChangelog(format!(
                    "# Release {} Notes\n\n**{}**\n\n{}",
                    version, log.name, log.body
                )))
                .unwrap()
        }
        Err(e) => log::warn!("{:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use im::vector;

    #[test]
    fn remerge() {
        let core = uk_manager::core::Manager::init().unwrap();
        super::apply_changes(&core, vector![], None).unwrap();
    }
}
