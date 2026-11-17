use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
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
    let portable = path.join("portable");
    let local_cemu = dirs2::data_local_dir().expect("YIKES").join("Cemu");
    let config_cemu = dirs2::config_dir().expect("YIKES").join("Cemu");
    let settings_path = if portable.join("settings.xml").exists() {
        portable.clone()
    } else {
        #[cfg(windows)]
        if let Some(path) = path.join("settings.xml").exists_then() {
            path
        } else if let Some(path) = config_cemu.join("settings.xml").exists_then() {
            path
        } else {
            anyhow::bail!(
                "Could not find Cemu settings file. Please run Cemu at least once to generate it."
            )
        }
        #[cfg(not(windows))]
        if let Some(path) = config_cemu.join("settings.xml").exists_then() {
            path
        } else {
            anyhow::bail!(
                "Could not find Cemu settings file. Please run Cemu at least once to generate it."
            )
        }
    };
    let text = fs::read_to_string(&settings_path).context("Failed to open Cemu settings file")?;
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
            portable.join("mlc01").exists_then()
        })
        .or_else(|| {
            #[cfg(windows)]
            {
                if let Some(path) = path.join("mlc01").exists_then() {
                    Some(path)
                } else {
                    config_cemu.join("mlc01").exists_then()
                }
            }
            #[cfg(not(windows))]
            {
                local_cemu.join("mlc01").exists_then()
            }
        });
    static REGIONS: &[&str] = &[
        "101C9400", "101c9400", "101C9500", "101c9500", "101C9300", "101c9300",
    ];
    let (base, update, dlc, wua, encrypted) = if let Some(cache) = config_cemu
        .join("title_list_cache.xml")
        .exists_then()
        .or_else(|| local_cemu.join("title_list_cache.xml").exists_then())
    {
        let title_list = fs::read_to_string(&cache).context("Failed to open Cemu title cache file")?;
        let title_tree = roxmltree::Document::parse(&title_list)
            .context("Failed to parse Cemu title cache file: invalid XML")?;
        let mut base_folder: Option<PathBuf> = None;
        let mut update_folder: Option<PathBuf> = None;
        let mut dlc_folder: Option<PathBuf> = None;
        let mut wua_file: Option<PathBuf> = None;
        let mut encrypted = false;
        title_tree.descendants()
            .filter_map(|n| {
                if n.tag_name().name() == "title" {
                    let title_id = n.attribute("titleId").expect("invalid title");
                    if !REGIONS.contains(&&title_id[8..]) {
                        None
                    } else {
                        let format = n.descendants()
                            .find(|c| c.tag_name().name() == "format")
                            .expect("invalid title")
                            .text()
                            .expect("invalid format")
                            .parse::<u32>()
                            .expect("invalid format");
                        if let Ok(path) = PathBuf::from(n.descendants()
                            .find(|c| c.tag_name().name() == "path")
                            .expect("invalid title")
                            .text()
                            .expect("invalid path"))
                            .canonicalize() {
                            Some((&title_id[..8], format, path))
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }
            }).for_each(|(dump_type, format, mut dump_path)| {
                #[allow(irrefutable_let_patterns)]
                {
                    if dump_path.is_relative() {
                        dump_path = if let p = path.join(&dump_path) && p.exists() { p }
                        else if let p = portable.join(&dump_path) && p.exists() { p }
                        else if let p = local_cemu.join(&dump_path) && p.exists() { p }
                        else if let p = config_cemu.join(&dump_path) && p.exists() { p }
                        else { return; };
                    }
                }
                match (dump_type, format) {
                    ("00050000", 1) => base_folder = Some(dump_path.join("content")),
                    ("0005000c", 1) => dlc_folder = Some(dump_path.join("content").join("0010")),
                    ("0005000e", 1) => update_folder = Some(dump_path.join("content")),
                    ("00050000", 4) |
                    ("0005000c", 4) |
                    ("0005000e", 4) => encrypted = true,
                    ("00050000", 3) => wua_file = Some(dump_path),
                    _ => {},
                }
            });
        (base_folder, update_folder, dlc_folder, wua_file, encrypted)
    } else {
        mlc_path
            .as_ref()
            .map(|mlc_path| {
                let title_path = mlc_path.join("usr/title");
                let base_folder = if let Some(base_folder_path) = REGIONS
                    .iter()
                    .find_map(|r| {
                        let path = title_path.join(jstr!("00050000/{r}/content"));
                        path.exists().then_some(path)
                    }
                ) {
                    Some(base_folder_path)
                } else {
                    std::fs::read_to_string(&settings_path)
                        .expect("Cannot read settings file")
                        .split('\n')
                        .find_map(|line| {
                            let line = line.trim();
                            if line.starts_with("<path>") && line.contains("U-King.rpx") {
                                Some(PathBuf::from(&line[6..line.len()-23]).join("content"))
                            } else {
                                None
                            }
                        })
                };
                let update_folder = REGIONS.iter().find_map(|r| {
                    let path = title_path.join(jstr!("0005000e/{r}/content"));
                    path.exists().then_some(path)
                });
                let dlc_folder = REGIONS.iter().find_map(|r| {
                    let path = title_path.join(jstr!("0005000c/{r}/content/0010"));
                    path.exists().then_some(path)
                });
                (base_folder, update_folder, dlc_folder, None, false)
            })
            .context("Could not find unpacked game dump from Cemu settings.")?
    };
    if (base.is_none() || update.is_none()) && wua.is_none() {
        anyhow::bail!(if encrypted {
            "UKMM does not support encrypted dumps."
        } else {
            "Could not find unpacked game dump from Cemu settings."
        });
    }
    let gfx_folder = if let Some(path) = portable
        .join("graphicPacks")
        .exists_then()
    {
        path
    } else {
        #[cfg(windows)]
        {
            settings_path.parent().expect("We know it exists").join("graphicPacks")
        }
        #[cfg(not(windows))]
        config_cemu.join("graphicPacks")
    };
    let mut settings = core.settings_mut();
    settings.current_mode = Platform::WiiU;
    let dump = if let Some(path) = wua {
        Arc::new(ResourceReader::from_zarchive(path)
            .context("Failed to validate game dump")?)
    } else {
        Arc::new(ResourceReader::from_unpacked_dirs(
            base.as_ref(),
            update,
            dlc,
            uk_content::prelude::Endian::Big,
        )
        .context("Failed to validate game dump")?)
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
                Some(format!("{} -g \"{}\"", exe.unwrap(), rpx.strip_prefix(r"\\?\").unwrap()))
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
struct BcmlSettings {
    lang: Language,
    cemu_dir: Option<PathBuf>,
    export_dir: Option<PathBuf>,
    export_dir_nx: Option<PathBuf>,
    game_dir: Option<PathBuf>,
    game_dir_nx: Option<PathBuf>,
    update_dir: Option<PathBuf>,
    dlc_dir: Option<PathBuf>,
    dlc_dir_nx: Option<PathBuf>,
    store_dir: PathBuf,
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
    let bcml_settings: BcmlSettings = serde_json::from_str(
        &fs::read_to_string(settings_path).context("Failed to read BCML settings file")?,
    )
    .context("Failed to parse BCML settings file")?;
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
                    core.mod_manager().add(&path, None)?;
                }
                Err(e) => log::warn!("Failed to import BCML mod: {}", e),
            }
        }
        core.mod_manager().save()?;
        Ok(())
    }
}
