#![allow(unstable_name_collisions)]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use http_req::request::RedirectPolicy;
use serde::Deserialize;
use strfmt::Format;
use uk_manager::{
    core::Manager,
    mods::Mod,
    settings::UpdatePreference,
    util::get_temp_file,
};
use uk_mod::{pack::sanitise, Manifest, Meta};

use super::{package::ModPackerBuilder, util::response, Message};
use crate::INTERFACE;

mod handlers;

pub use handlers::register_handlers;
use uk_localization::string_ext::LocString;

pub fn open_mod(core: &Manager, path: &Path, meta: Option<Meta>) -> Result<Message> {
    Ok(match uk_app::open_mod(core, path, meta)? {
        uk_app::Opened::Mod(mod_) => Message::HandleMod(mod_),
        uk_app::Opened::NeedsMeta(path) => Message::RequestMeta(path),
    })
}

pub fn apply_changes(core: &Manager, mods: Vec<Mod>, dirty: Option<Manifest>) -> Result<Message> {
    uk_app::apply_changes(core, mods, dirty)?;
    Ok(Message::ResetMods(None))
}

pub fn package_mod(core: &Manager, builder: ModPackerBuilder) -> Result<Message> {
    uk_app::package_mod(core, builder.source, builder.dest, builder.meta)?;
    Ok(Message::ResetPacker)
}

pub fn dev_update_mods(core: &Manager, mods: Vec<Mod>) -> Result<Message> {
    let mut dirty = Manifest::default();
    for mod_ in mods {
        log::info!("Updating {}…", mod_.meta.name.as_str());
        let message = "Mod_Update_Folder".localize();
        let vars = std::collections::HashMap::from(
            [("mod_name".to_string(), mod_.meta.name.to_string())]
        );
        let Some(folder) = rfd::FileDialog::new()
            .set_title(message.format(&vars)?)
            .pick_folder()
        else {
            return Ok(Message::Noop);
        };
        dirty.extend(&uk_app::dev_update_mod(core, mod_, folder)?);
    }
    Ok(Message::ResetMods(Some(dirty)))
}

pub fn extract_mods(core: &Manager, mods: Vec<Mod>) -> Result<Message> {
    if let Some(folder) = rfd::FileDialog::new()
        .set_title("Mod_Unpack_Folder".localize())
        .pick_folder()
    {
        uk_app::extract_mods(core, mods, folder)?;
    }
    Ok(Message::Noop)
}

pub fn parse_meta(file: PathBuf) -> Result<Message> {
    uk_app::parse_meta(file).map(Message::UpdatePackageMeta)
}

pub fn import_cemu_settings(core: &Manager, path: &Path) -> Result<Message> {
    uk_app::import_cemu_settings(core, path)?;
    Ok(Message::ResetSettings)
}

pub fn migrate_bcml(core: Arc<Manager>) -> Result<Message> {
    uk_app::migrate_bcml(&core)?;
    Ok(Message::HandleSettings)
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
    let url = "https://api.github.com/repos/NiceneNerd/UKMM/releases?per_page=10";
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
    let temp_file = get_temp_file();
    dbg!(temp_file.as_path());
    fs::write(temp_file.as_path(), data)?;
    let exe = std::env::current_exe()?;
    if cfg!(windows) {
        let mut arc = zip::ZipArchive::new(fs::File::open(temp_file.as_path())?)?;
        arc.extract(temp_file.parent().context("Weird, no temp file parent")?)?;
        fs::rename(&exe, exe.with_extension("bak"))?;
        fs::copy(temp_file.with_file_name("ukmm.exe"), exe)?;
    } else {
        fs::rename(&exe, exe.with_extension("bak"))?;
        let out = std::process::Command::new("tar")
            .arg("xf")
            .arg(temp_file.as_path())
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

pub static ONECLICK_SENDER: std::sync::OnceLock<flume::Sender<Message>> =
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
            .redirect_policy(RedirectPolicy::Limit(0))
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
        match INTERFACE.claim() {
            Ok(sock) => {
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
            },
            Err(e) => {
                crate::display_error(
                    Box::new(
                        format!(
                            "One-Click listener failed to initialize. Is UKMM already open?\n{}",
                            e.to_string()
                        )
                    )
                )
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
