#![allow(unstable_name_collisions)]
use std::{
    fmt::Write,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow_ext::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use serde::Deserialize;
use strfmt::Format;
use uk_content::constants::Language;
use uk_manager::{
    bnp::convert_bnp,
    core::Manager,
    mods::Mod,
    settings::{DeployConfig, Platform, PlatformSettings, UpdatePreference},
    util::get_temp_file,
};
use uk_mod::{
    pack::{sanitise, ModPacker},
    unpack::{ModReader, ModUnpacker},
    Manifest, Meta,
};
use uk_reader::ResourceReader;
use uk_util::PathExt;

use super::{package::ModPackerBuilder, util::response, Message};
use crate::{gui::LOCALIZATION, INTERFACE};

mod handlers;

pub use handlers::register_handlers;

fn is_probably_a_mod_and_has_meta(path: &Path) -> (bool, bool) {
    if path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        == "rules.txt"
    {
        return (true, true);
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default();
    if ext != "zip" && ext != "7z" {
        (false, false)
    } else if ext == "7z" {
        (true, false)
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
                let has_meta = zip.file_names().any(|n| n.ends_with("rules.txt"));
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
        .unwrap_or(false) ||
       path
        .join("info.json")
        .exists()
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
            if let Some(has_meta) = (err_msg.contains("meta file")
                || err_msg.contains("meta.yml")
                || err_msg.contains("d Zip"))
            .then(|| is_probably_a_mod_and_has_meta(path))
            .filter(|(is_mod, _has_meta)| *is_mod)
            .map(|(_, has_meta)| has_meta)
            {
                log::info!("Maybe it's not a UKMM mod, let's try to convert it");
                if !has_meta && meta.is_none() {
                    log::info!("Mod has no meta info, requesting manual input");
                    return Ok(Message::RequestMeta(path.to_path_buf()));
                }
                if let Some(ref meta) = meta {
                    log::info!("Converting mod {}…", meta.name);
                }
                let converted_path =
                    uk_manager::mods::convert_gfx(core, path, meta).with_context(|| {
                        format!(
                            "Failed to convert {}",
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or_default()
                        )
                    })?;
                Mod::from_reader(
                    ModReader::open_peek(converted_path, vec![])
                        .context("Failed to open converted mod")?,
                )
            } else {
                return Err(err.context(format!("Failed to open mod {}", path.display())));
            }
        }
    };
    Ok(Message::HandleMod(mod_))
}

pub fn apply_changes(core: &Manager, mods: Vec<Mod>, dirty: Option<Manifest>) -> Result<Message> {
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
    Ok(Message::ResetMods(None))
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
    Ok(Message::ResetPacker)
}

pub fn dev_update_mods(core: &Manager, mods: Vec<Mod>) -> Result<Message> {
    let mut dirty = Manifest::default();
    for mod_ in mods {
        log::info!("Updating {}…", mod_.meta.name.as_str());
        let loc = LOCALIZATION.read();
        let message = loc.get("Mod_Update_Folder");
        let vars = std::collections::HashMap::from(
            [("mod_name".to_string(), mod_.meta.name.to_string())]
        );
        if let Some(folder) = rfd::FileDialog::new()
            .set_title(message.format(&vars).unwrap())
            .pick_folder()
        {
            dirty.extend(&mod_.manifest().unwrap_or_default());
            let hash = mod_.hash();
            uk_mod::pack::ModPacker::new(
                folder,
                &mod_.path,
                None,
                [core.settings().dump().unwrap()].into_iter().collect(),
            )
            .context("Failed to initialize mod packager")?
            .pack()
            .context("Failed to package mod")?;
            let new_mod = ModReader::open_peek(mod_.path, vec![])?;
            dirty.extend(new_mod.manifest());
            core.mod_manager_mut()
                .replace(Mod::from_reader(new_mod), hash)?;
        } else {
            return Ok(Message::Noop);
        }
    }
    Ok(Message::ResetMods(Some(dirty)))
}

pub fn extract_mods(core: &Manager, mods: Vec<Mod>) -> Result<Message> {
    let mut errors = vec![];
    let loc = LOCALIZATION.read();
    if let Some(folder) = rfd::FileDialog::new()
        .set_title(loc.get("Mod_Unpack_Folder"))
        .pick_folder()
    {
        let settings = core.settings();
        let config = settings
            .platform_config()
            .context("No config for current platform. Have you configured your settings?")?;
        for mod_ in mods {
            let name = mod_.meta.name.as_str();
            log::info!("Extracting {}…", name);
            let unpacker = ModUnpacker::new(
                config.dump.clone(),
                core.settings().current_mode.into(),
                config.language,
                vec![ModReader::open(&mod_.path, mod_.enabled_options.clone())?],
                folder.join(name),
            );
            if let Err(e) = unpacker.unpack() {
                log::error!("{e:?}");
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(Message::Noop)
        } else {
            anyhow_ext::bail!(
                "One or more mods encountered errors when extracting. Details below:\n{}",
                errors.into_iter().fold(String::new(), |mut acc, e| {
                    writeln!(acc, "{:?}", e).expect("Failed to write to String");
                    acc
                })
            )
        }
    } else {
        Ok(Message::Noop)
    }
}

pub fn parse_meta(file: PathBuf) -> Result<Message> {
    match file.extension().and_then(|x| x.to_str()).unwrap() {
        "txt" => ModPacker::parse_rules(file),
        "yml" => Meta::parse(file),
        "json" => ModPacker::parse_info(file),
        _ => unreachable!(),
    }
    .map(Message::UpdatePackageMeta)
}

pub fn import_cemu_settings(core: &Manager, path: &Path) -> Result<Message> {
    let settings_path = if let Some(path) = path.join("portable/settings.xml").exists_then() {
        path
    } else if let Some(path) = path.join("settings.xml").exists_then() {
        path
    } else if let Some(path) = dirs2::config_dir()
        .expect("YIKES")
        .join("Cemu/settings.xml")
        .exists_then()
    {
        path
    } else if let Some(path) = dirs2::data_local_dir()
        .expect("DOUBLE YIKES")
        .join("Cemu/settings.xml")
        .exists_then()
    {
        path
    } else {
        anyhow::bail!(
            "Could not find Cemu settings file. Please run Cemu at least once to generate it."
        )
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
            path.with_file_name("mlc01").exists_then()
        })
        .or_else(|| {
            dirs2::config_dir()
                .expect("YIKES")
                .join("Cemu")
                .join("mlc01")
                .exists_then()
        })
        .or_else(|| {
            dirs2::data_local_dir()
                .expect("DOUBLE YIKES")
                .join("Cemu")
                .join("mlc01")
                .exists_then()
        });
    static REGIONS: &[&str] = &[
        "101C9400", "101c9400", "101C9500", "101c9500", "101C9300", "101c9300",
    ];
    let (base, update, dlc, wua) = if let Some(cache) = dirs2::config_dir()
        .expect("YIKES")
        .join("Cemu")
        .join("title_list_cache.xml")
        .exists_then()
        .or_else(|| {
            dirs2::data_local_dir()
                .expect("YIKES")
                .join("Cemu")
                .join("title_list_cache.xml")
                .exists_then()
        })
    {
        let title_list = fs::read_to_string(&cache).context("Failed to open Cemu title cache file")?;
        let title_tree = roxmltree::Document::parse(&title_list)
            .context("Failed to parse Cemu title cache file: invalid XML")?;
        let mut base_folder: Option<PathBuf> = None;
        let mut update_folder: Option<PathBuf> = None;
        let mut dlc_folder: Option<PathBuf> = None;
        let mut wua_file: Option<PathBuf> = None;
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
            }).for_each(|(dump_type, format, path)| {
                match (dump_type, format) {
                    ("00050000", 1) => base_folder = Some(path.join("content")),
                    ("0005000c", 1) => dlc_folder = Some(path.join("content").join("0010")),
                    ("0005000e", 1) => update_folder = Some(path.join("content")),
                    ("00050000", 3) => wua_file = Some(path),
                    _ => {},
                }
            });
        (base_folder, update_folder, dlc_folder, wua_file)
    } else {
        mlc_path
            .as_ref()
            .map(|mlc_path| {
                let title_path = mlc_path.join("usr/title");
                let base_folder = REGIONS.iter().find_map(|r| {
                    let path = title_path.join(jstr!("00050000/{r}/content"));
                    path.exists().then_some(path)
                });
                let update_folder = REGIONS.iter().find_map(|r| {
                    let path = title_path.join(jstr!("0005000e/{r}/content"));
                    path.exists().then_some(path)
                });
                let dlc_folder = REGIONS.iter().find_map(|r| {
                    let path = title_path.join(jstr!("0005000c/{r}/content/0010"));
                    path.exists().then_some(path)
                });
                (base_folder, update_folder, dlc_folder, None)
            })
            .expect("Could not find unpacked game dump from Cemu settings.")
    };
    let gfx_folder = if let Some(path) = path.with_file_name("graphicPacks").exists_then() {
        path
    } else if let Some(path) = dirs2::config_dir()
        .expect("YIKES")
        .join("Cemu")
        .join("graphicPacks")
        .exists_then()
    {
        path
    } else if let Some(path) = dirs2::data_local_dir()
        .expect("YIKES")
        .join("Cemu")
        .join("graphicPacks")
        .exists_then()
    {
        path
    } else if let Some(path) = settings_path.parent() {
        log::warn!("Cemu graphic pack folder not found. Defaulting to settings.xml location");
        path.to_path_buf().join("graphicPacks")
    } else {
        anyhow::bail!("We lost our settings path somehow...");
    };
    let mut settings = core.settings_mut();
    settings.current_mode = Platform::WiiU;
    let dump = if wua.is_some() {
        Arc::new(ResourceReader::from_zarchive(unsafe { wua.unwrap_unchecked() })
            .context("Failed to validate game dump")?)
    } else {
        Arc::new(ResourceReader::from_unpacked_dirs(base, update, dlc)
            .context("Failed to validate game dump")?)
    };
    if let Some(wiiu_config) = settings.wiiu_config.as_mut() {
        wiiu_config.dump = dump;
        let deploy_config = wiiu_config.deploy_config.get_or_insert_default();
        deploy_config.auto = true;
        deploy_config.output = gfx_folder.clone();
        deploy_config.executable = path
            .join("Cemu.exe")
            .exists_then()
            .map(|p| p.display().to_string());
        deploy_config.layout = uk_manager::settings::DeployLayout::WithName;
    } else {
        settings.wiiu_config = Some(PlatformSettings {
            language: uk_content::constants::Language::USen,
            profile: "Default".into(),
            dump,
            deploy_config: Some(DeployConfig {
                auto: true,
                method: uk_manager::settings::DeployMethod::Symlink,
                output: gfx_folder.clone(),
                cemu_rules: true,
                executable: path
                    .join("Cemu.exe")
                    .exists_then()
                    .map(|p| p.display().to_string()),
                layout: uk_manager::settings::DeployLayout::WithName,
            }),
        })
    };
    settings.save()?;
    Ok(Message::ResetSettings)
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

pub fn migrate_bcml(core: Arc<Manager>) -> Result<Message> {
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
    Ok(Message::HandleSettings)
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

#[derive(Debug, Deserialize, Clone)]
pub struct VersionAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VersionResponse {
    body: String,
    name: String,
    tag_name: String,
    prerelease: bool,
    assets: Vec<VersionAsset>,
}

impl VersionResponse {
    pub fn description(&self) -> String {
        format!(
            "# Release {} Notes\n\n**{}**\n\n{}",
            self.tag_name, self.name, self.body
        )
    }
}

pub fn get_releases(core: Arc<Manager>, sender: flume::Sender<Message>) {
    let url = "https://api.github.com/repos/GingerAvalanche/UKMM/releases?per_page=10";
    match response(url).and_then(|bytes| {
        serde_json::from_slice::<Vec<VersionResponse>>(&bytes)
            .context("Failed to parse GitHub response")
    }) {
        Ok(mut releases) => {
            let current_semver = lenient_semver::parse(env!("CARGO_PKG_VERSION")).unwrap();
            let betas = core.settings().check_updates == UpdatePreference::Beta
                || current_semver < lenient_semver::parse("1.0.0").unwrap();
            releases.retain(|r| !r.prerelease || betas);
            if let Some((release, release_ver)) = releases.first().and_then(|r| {
                lenient_semver::parse(r.tag_name.trim_start_matches('v'))
                    .ok()
                    .map(|v| (r, v))
            }) {
                match release_ver.cmp(&current_semver) {
                    std::cmp::Ordering::Greater => {
                        sender.send(Message::OfferUpdate(release.clone())).unwrap()
                    }
                    std::cmp::Ordering::Less => {
                        sender
                            .send(Message::SetChangelog(release.description()))
                            .unwrap()
                    }
                    _ => (),
                }
            }
        }
        Err(e) => log::warn!("{:?}", e),
    }
}

pub fn do_update(version: VersionResponse) -> Result<Message> {
    log::info!("Updating... UKMM will restart when complete");
    #[cfg(target_os = "windows")]
    let asset_name = "ukmm-x86_64-pc-windows-msvc.zip";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let asset_name = "ukmm-aarch64-apple-darwin.tar.xz";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let asset_name = "ukmm-x86_64-apple-darwin.tar.xz";
    #[cfg(target_os = "linux")]
    let asset_name = "ukmm-x86_64-unknown-linux-gnu.tar.xz";
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    )))]
    let asset_name = "";
    let asset = version
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .context("No matching platform for update")?;
    let data = response(asset.browser_download_url.as_str())?;
    let tmpfile = get_temp_file();
    dbg!(tmpfile.as_path());
    fs::write(tmpfile.as_path(), data)?;
    let exe = std::env::current_exe().unwrap();
    if cfg!(windows) {
        let mut arc = zip::ZipArchive::new(fs::File::open(tmpfile.as_path())?)?;
        arc.extract(tmpfile.parent().context("Weird, no temp file parent")?)?;
        fs::rename(&exe, exe.with_extension("bak"))?;
        fs::copy(tmpfile.with_file_name("ukmm.exe"), exe)?;
    } else {
        fs::rename(&exe, exe.with_extension("bak"))?;
        let out = std::process::Command::new("tar")
            .arg("xf")
            .arg(tmpfile.as_path())
            .arg("-C")
            .arg(exe.parent().context("Weird, no exe parent")?)
            .arg("--overwrite")
            .output()?;
        if !out.stderr.is_empty() {
            anyhow::bail!(String::from_utf8_lossy(&out.stderr).to_string());
        }
    };
    Ok(Message::Restart)
}

pub static ONECLICK_SENDER: std::sync::OnceLock<flume::Sender<super::Message>> =
    std::sync::OnceLock::new();

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
enum IpcMessage {
    OpenMod(PathBuf),
    Error(String),
    Starting(String),
}

impl From<IpcMessage> for Message {
    fn from(value: IpcMessage) -> Self {
        match value {
            IpcMessage::OpenMod(path) => Message::OpenMod(path),
            IpcMessage::Error(e) => Message::Error(anyhow::anyhow!(e)),
            IpcMessage::Starting(mod_name) => Message::SetDownloading(mod_name),
        }
    }
}

impl IpcMessage {
    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

pub fn oneclick(url: &str) {
    fn process(url: &str) -> IpcMessage {
        let mut parts = url.split(',');
        let url = parts.next().unwrap_or_default().to_owned();
        let cat = parts.next().unwrap_or_default().to_owned();
        let id = parts.next().unwrap_or_default().to_owned();
        log::debug!("Processing GameBanana 1-click URL: {url}");
        log::debug!("Checking mod name from API");
        let mod_name = response(&format!(
            "https://api.gamebanana.com/Core/Item/Data?itemtype={cat}&itemid={id}&fields=name"
        ))
        .and_then(|data| Ok(serde_json::from_slice::<Vec<String>>(&data)?))
        .map(|mut res| sanitise(&res.remove(0)))
        .unwrap_or_else(|_| "oneclick_mod".into());
        log::info!("Downloading {mod_name} from GameBanana 1-click…");
        if let Ok(client) = INTERFACE.connect() {
            let buf = IpcMessage::Starting(mod_name.clone()).into_bytes();
            let _ = client.send(&buf);
        }
        let mut data = vec![];
        let msg = http_req::request::Request::new(&url.as_str().try_into().unwrap())
            .method(http_req::request::Method::GET)
            .header("User-Agent", "UKMM")
            .send(&mut data)
            .with_context(|| format!("Failed to download mod from {url}"))
            .and_then(|res| {
                let redir = res
                    .headers()
                    .get("Location")
                    .context("No location for redirect")?;
                let filename = http_req::uri::Uri::try_from(redir.as_str())?
                    .path()
                    .unwrap_or_default()
                    .split('/')
                    .last()
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| format!("{mod_name}.bnp"));
                let data = response(redir)
                    .with_context(|| format!("Failed to download mod from {redir}"))?;
                let tmp = get_temp_file().with_file_name(filename);
                log::debug!("Saving mod to temp file at {}", tmp.display());
                fs_err::write(tmp.as_path(), data).context("Failed to save mod to temp file")?;
                log::info!("Finished downloading {mod_name}");
                Ok(IpcMessage::OpenMod(tmp.to_path_buf()))
            })
            .map_err(|e| IpcMessage::Error(e.to_string()))
            .unwrap_or_else(|e| e);
        log::debug!("1-click mod downloaded, sending to UI for install");
        msg
    }

    match INTERFACE.connect() {
        Ok(client) => {
            let msg = process(url);
            let buf = msg.into_bytes();
            client
                .send(&buf)
                .expect("Failed to send mod to existing UKMM instance");
            std::process::exit(0);
        }
        Err(_) => {
            let url = url.to_owned();
            std::thread::spawn(move || {
                let msg = process(&url);
                let mut sender = ONECLICK_SENDER.get();
                while sender.is_none() {
                    sender = ONECLICK_SENDER.get();
                }
                sender.unwrap().send(msg.into()).expect("Broken channel")
            });
        }
    }
}

pub fn wait_ipc() {
    std::thread::spawn(|| {
        let sock = INTERFACE
            .claim()
            .expect("Failed to claim single instance interface. Is UKMM already open?");
        let mut buf = [0; 1024];
        loop {
            match sock.recv(&mut buf) {
                Ok(len) => {
                    log::debug!("Received 1-click install message");
                    let msg: IpcMessage = serde_json::from_slice(&buf[..len])
                        .with_context(|| String::from_utf8(buf.to_vec()).unwrap_or_default())
                        .expect("Broken IPC message");
                    log::trace!("{:?}", &msg);
                    let mut sender = ONECLICK_SENDER.get();
                    while sender.is_none() {
                        sender = ONECLICK_SENDER.get();
                    }
                    sender.unwrap().send(msg.into()).expect("Broken channel");
                }
                Err(e) => {
                    log::error!("IPC error: {}", e);
                }
            }
        }
    });
}

pub fn handle_mod_arg(path: PathBuf) {
    if path.exists() {
        std::thread::spawn(|| {
            log::info!("Opening mod at {} for installation…", path.display());
            let mut sender = ONECLICK_SENDER.get();
            while sender.is_none() {
                sender = ONECLICK_SENDER.get();
            }
            sender
                .unwrap()
                .send(Message::OpenMod(path))
                .expect("Broken channel")
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn remerge() {
        let core = uk_manager::core::Manager::init().unwrap();
        super::apply_changes(&core, vec![], None).unwrap();
    }
}
