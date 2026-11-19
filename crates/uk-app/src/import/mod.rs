pub mod cemu;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::Deserialize;
use uk_content::constants::Language;
use uk_manager::{
    bnp::convert_bnp,
    core::Manager,
    settings::{DeployConfig, Platform, PlatformSettings},
};
use uk_reader::ResourceReader;
use uk_util::PathExt;

pub fn import_cemu_settings(core: &Manager, path: &Path) -> Result<()> {
    let dirs = cemu::CemuDirs::detect(path);
    let settings_path = cemu::find_settings_xml(&dirs)?;
    let settings_xml =
        fs::read_to_string(&settings_path).context("Failed to open Cemu settings file")?;
    let cemu::DumpPaths {
        base,
        update,
        dlc,
        wua,
        encrypted,
    } = cemu::resolve_dump(&dirs, &settings_xml)?;
    if (base.is_none() || update.is_none()) && wua.is_none() {
        anyhow::bail!(if encrypted {
            "UKMM does not support encrypted dumps."
        } else {
            "Could not find unpacked game dump from Cemu settings."
        });
    }
    let gfx_folder = if let Some(path) = dirs.portable.join("graphicPacks").exists_then() {
        path
    } else {
        #[cfg(windows)]
        {
            settings_path
                .parent()
                .expect("We know it exists")
                .join("graphicPacks")
        }
        #[cfg(not(windows))]
        dirs.config.join("graphicPacks")
    };
    let mut settings = core.settings_mut();
    settings.current_mode = Platform::WiiU;
    let dump = if let Some(path) = wua {
        Arc::new(ResourceReader::from_zarchive(path).context("Failed to validate game dump")?)
    } else {
        Arc::new(
            ResourceReader::from_unpacked_dirs(
                base.as_ref(),
                update,
                dlc,
                uk_content::prelude::Endian::Big,
            )
            .context("Failed to validate game dump")?,
        )
    };
    #[cfg(windows)]
    let exe = path
        .join("Cemu.exe")
        .exists_then()
        .map(|p| format!("\"{}\"", p.display().to_string()));
    #[cfg(not(windows))]
    let exe = path
        .join("Cemu")
        .exists_then()
        .or_else(|| {
            // glob lists in alphabetical order, so we can just return the last one
            glob::glob(&path.join("Cemu-*-*.AppImage").to_string_lossy())
                .ok()?
                .filter_map(Result::ok)
                .last()
        })
        .map(|p| {
            let str_ = p.to_string_lossy();
            if str_.contains(' ') {
                format!("\"{str_}\"")
            } else {
                str_.into()
            }
        });
    let exe_cmd = if exe.is_some() {
        if let Some(item) = base {
            let rpx = item
                .parent()
                .unwrap()
                .join("code")
                .join("U-King.rpx")
                .display()
                .to_string();
            #[cfg(windows)]
            {
                Some(format!(
                    "{} -g \"{}\"",
                    exe.unwrap(),
                    rpx.strip_prefix(r"\\?\").unwrap()
                ))
            }
            #[cfg(not(windows))]
            {
                if rpx.contains(' ') {
                    Some(format!("{} -g \"{}\"", exe.unwrap(), rpx))
                } else {
                    Some(format!("{} -g {}", exe.unwrap(), rpx))
                }
            }
        } else {
            exe
        }
    } else {
        None
    };
    if let Some(wiiu_config) = settings.wiiu_config.as_mut() {
        wiiu_config.dump = dump;
        let deploy_config = wiiu_config.deploy_config.get_or_insert_default();
        deploy_config.auto = true;
        deploy_config.cemu_rules = true;
        deploy_config.output = gfx_folder.clone();
        deploy_config.executable = exe_cmd;
        deploy_config.method = uk_manager::settings::DeployMethod::Symlink;
        deploy_config.layout = uk_manager::settings::DeployLayout::WithName;
    } else {
        settings.wiiu_config = Some(PlatformSettings {
            language: Language::USen,
            profile: "Default".into(),
            dump,
            deploy_config: Some(DeployConfig {
                auto: true,
                method: uk_manager::settings::DeployMethod::Symlink,
                output: gfx_folder.clone(),
                cemu_rules: true,
                executable: exe_cmd,
                layout: uk_manager::settings::DeployLayout::WithName,
            }),
        })
    };
    settings.save()?;
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct BcmlSettings {
    pub lang: Language,
    pub cemu_dir: Option<PathBuf>,
    pub export_dir: Option<PathBuf>,
    pub export_dir_nx: Option<PathBuf>,
    pub game_dir: Option<PathBuf>,
    pub game_dir_nx: Option<PathBuf>,
    pub update_dir: Option<PathBuf>,
    pub dlc_dir: Option<PathBuf>,
    pub dlc_dir_nx: Option<PathBuf>,
    pub store_dir: PathBuf,
}

/// Pure: parses BCML's `settings.json`.
pub fn parse_bcml_settings(text: &str) -> Result<BcmlSettings> {
    serde_json::from_str(text).context("Failed to parse BCML settings file")
}

pub fn migrate_bcml(core: &Manager) -> Result<()> {
    log::info!("Attempting to import BCML settings");
    let current_mode = core.settings().current_mode;
    let settings_path = if cfg!(windows) {
        dirs2::data_local_dir()
    } else {
        dirs2::config_dir()
    }
    .unwrap()
    .join("bcml/settings.json");
    let bcml_settings = parse_bcml_settings(
        &fs::read_to_string(settings_path).context("Failed to read BCML settings file")?,
    )?;
    if let (Some(game_dir), Some(update_dir)) = (
        bcml_settings.game_dir.filter(|d| !d.as_os_str().is_empty()),
        bcml_settings
            .update_dir
            .filter(|d| !d.as_os_str().is_empty()),
    ) {
        {
            log::info!("Import BCML Wii U game dump settings");
            let mut settings = core.settings_mut();
            settings.wiiu_config = Some(PlatformSettings {
                language: bcml_settings.lang,
                profile: "Default".into(),
                deploy_config: bcml_settings
                    .export_dir
                    .map(|export_dir| {
                        DeployConfig {
                            output: export_dir,
                            cemu_rules: bcml_settings.cemu_dir.is_some(),
                            ..Default::default()
                        }
                    })
                    .or_else(|| {
                        bcml_settings.cemu_dir.map(|cemu_dir| {
                            DeployConfig {
                                output: cemu_dir.join("graphicPacks/BreathOfTheWild_UKMM"),
                                cemu_rules: true,
                                ..Default::default()
                            }
                        })
                    }),
                dump: Arc::new(ResourceReader::from_unpacked_dirs(
                    Some(game_dir),
                    Some(update_dir),
                    bcml_settings.dlc_dir,
                    uk_content::prelude::Endian::Big,
                )?),
            });
            settings.current_mode = Platform::WiiU;
            settings.save()?;
        }
        core.reload()?;
        log::info!("Attempting to import BCML Wii U mods");
        import_mods(&core, bcml_settings.store_dir.join("mods"))?;
    }
    if let Some(game_dir) = bcml_settings
        .game_dir_nx
        .filter(|d| !d.as_os_str().is_empty())
    {
        {
            log::info!("Import BCML Switch game dump settings");
            let mut settings = core.settings_mut();
            settings.switch_config = Some(PlatformSettings {
                language: bcml_settings.lang,
                profile: "Default".into(),
                deploy_config: bcml_settings.export_dir_nx.map(|export_dir| {
                    DeployConfig {
                        output: export_dir,
                        ..Default::default()
                    }
                }),
                dump: Arc::new(ResourceReader::from_unpacked_dirs(
                    Some(game_dir),
                    None::<PathBuf>,
                    bcml_settings.dlc_dir_nx,
                    uk_content::prelude::Endian::Little,
                )?),
            });
            settings.current_mode = Platform::Switch;
            settings.save()?;
        }
        core.reload()?;
        log::info!("Attempting to import BCML Switch mods");
        import_mods(&core, bcml_settings.store_dir.join("mods_nx"))?;
    }
    let mode_changed = core.settings().current_mode != current_mode;
    if mode_changed {
        {
            let mut settings = core.settings_mut();
            settings.current_mode = current_mode;
            settings.save()?;
        }
        core.reload()?;
    }
    Ok(())
}

fn import_mods(core: &Manager, mod_dir: PathBuf) -> Result<()> {
    if !mod_dir.exists() {
        Ok(())
    } else {
        for dir in fs::read_dir(mod_dir)?.filter_map(|e| {
            e.ok().and_then(|e| {
                e.file_type().ok().and_then(|t| {
                    (t.is_dir()
                        && !e
                            .file_name()
                            .to_str()
                            .map(|n| n.starts_with("9999"))
                            .unwrap_or(false))
                    .then(|| e.path())
                })
            })
        }) {
            match convert_bnp(core, &dir) {
                Ok(path) => {
                    core.with_mod_manager(|mm| mm.add(&path, None))?;
                }
                Err(e) => log::warn!("Failed to import BCML mod: {}", e),
            }
        }
        core.with_mod_manager(|mm| mm.save())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BCML_SETTINGS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/bcml/settings.json"
    ));

    #[test]
    fn parses_bcml_settings_fixture() {
        let settings = parse_bcml_settings(BCML_SETTINGS).unwrap();
        assert_eq!(settings.lang, Language::USen);
        assert_eq!(settings.game_dir.as_deref(), Some(Path::new("/dumps/botw")));
        assert_eq!(
            settings.update_dir.as_deref(),
            Some(Path::new("/dumps/botw-update"))
        );
        assert_eq!(
            settings.dlc_dir.as_deref(),
            Some(Path::new("/dumps/botw-dlc"))
        );
        assert_eq!(
            settings.game_dir_nx.as_deref(),
            Some(Path::new("/dumps/botw-nx"))
        );
        assert_eq!(settings.cemu_dir.as_deref(), Some(Path::new("/opt/cemu")));
        assert_eq!(
            settings.store_dir,
            PathBuf::from("/home/user/.local/share/bcml")
        );
    }

    #[test]
    fn bcml_settings_default_when_empty() {
        let settings = parse_bcml_settings("{}").unwrap();
        assert_eq!(settings.lang, Language::USen);
        assert!(settings.game_dir.is_none());
        assert!(settings.store_dir.as_os_str().is_empty());
    }

    #[test]
    fn malformed_bcml_settings_is_an_error_not_a_panic() {
        assert!(parse_bcml_settings("{not json").is_err());
        assert!(parse_bcml_settings("[1,2]").is_err());
    }
}
