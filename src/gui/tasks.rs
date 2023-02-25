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
use serde_json::Value;
use uk_content::constants::Language;
use uk_manager::{
    bnp::convert_bnp,
    core::Manager,
    mods::Mod,
    settings::{DeployConfig, Platform, PlatformSettings, UpdatePreference},
    util::get_temp_file,
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
            language: uk_content::constants::Language::USen,
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
    let current_mode = core.settings().current_mode;
    let settings_path = dirs2::config_dir().unwrap().join("bcml/settings.json");
    let bcml_settings: BcmlSettings = serde_json::from_str(
        &fs::read_to_string(settings_path).context("Failed to read BCML settings file")?,
    )
    .context("Failed to parse BCML settings file")?;
    if let Some(game_dir) = bcml_settings.game_dir && let Some(update_dir) = bcml_settings.update_dir {
        {
            let mut settings = core.settings_mut();
            settings.wiiu_config = Some(PlatformSettings {
                cemu_rules: bcml_settings.cemu_dir.is_some(),
                language: bcml_settings.lang,
                profile: "Default".into(),
                deploy_config: bcml_settings
                    .export_dir
                    .map(|export_dir| {
                        DeployConfig {
                            output: export_dir,
                            ..Default::default()
                        }
                    })
                    .or_else(|| {
                        bcml_settings.cemu_dir.map(|cemu_dir| {
                            DeployConfig {
                                output: cemu_dir.join("graphicPacks/BreathOfTheWild_UKMM"),
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
        import_mods(&core, bcml_settings.store_dir.join("mods"))?;
    }
    if let Some(game_dir) = bcml_settings.game_dir_nx {
        {
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
                cemu_rules: false,
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
                    core.mod_manager().add(&path)?;
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
    let url = "https://api.github.com/repos/NiceneNerd/ukmm/releases?per_page=10";
    match reqwest::blocking::Client::builder()
        .user_agent("UKMM")
        .build()
        .unwrap()
        .get(url)
        .send()
        .context("Failed to check release notes")
        .and_then(|r| {
            r.json::<Vec<VersionResponse>>()
                .context("Failed to parse release notes")
        }) {
        Ok(mut releases) => {
            let current_semver = lenient_semver::parse(env!("CARGO_PKG_VERSION")).unwrap();
            let betas = core.settings().check_updates == UpdatePreference::Beta
                || current_semver < lenient_semver::parse("1.0.0").unwrap();
            releases.retain(|r| !r.prerelease || betas);
            if let Some(release) = releases.first()
                && let Ok(release_ver) = lenient_semver::parse(release.tag_name.trim_start_matches('v'))
            {
                match release_ver.cmp(&current_semver) {
                    std::cmp::Ordering::Greater => sender.send(Message::OfferUpdate(release.clone())).unwrap(),
                    _ => sender.send(Message::SetChangelog(release.description())).unwrap(),
                }
            }
        }
        Err(e) => log::warn!("{:?}", e),
    }
}

pub fn do_update(version: VersionResponse) -> Result<Message> {
    let platform =
        option_env!("UPDATE_PLATFORM").unwrap_or(if cfg!(windows) { "windows" } else { "linux" });
    let asset = version
        .assets
        .iter()
        .find(|asset| asset.name[..asset.name.len() - 4].ends_with(platform))
        .context("No matching platform for update")?;
    let data = reqwest::blocking::Client::builder()
        .user_agent("UKMM")
        .build()
        .unwrap()
        .get(&asset.browser_download_url)
        .send()
        .context("Failed to download release archive")?
        .bytes()?;
    let tmpfile = get_temp_file();
    fs::write(tmpfile.as_path(), data)?;
    let exe = std::env::current_exe().unwrap();
    if cfg!(windows) {
        let mut arc = zip::ZipArchive::new(fs::File::open(tmpfile.as_path())?)?;
        arc.extract(tmpfile.parent().context("Weird, no temp file parent")?)?;
        fs::copy(tmpfile.with_file_name("ukmm.exe"), exe)?;
    } else {
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

#[cfg(test)]
mod tests {
    use im::vector;

    #[test]
    fn remerge() {
        let core = uk_manager::core::Manager::init().unwrap();
        super::apply_changes(&core, vector![], None).unwrap();
    }
}
