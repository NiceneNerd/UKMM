//! GameBanana one-click install IPC (ssilide, local TCP port 6666) and
//! handling of `bcml:` URLs passed on the command line.

use std::{path::PathBuf, sync::OnceLock};

use anyhow_ext::{Context, Result};
use uk_manager::util::get_temp_file;
use uk_mod::pack::sanitise;

use crate::host::http::{http_get, http_redirect_location};

/// The local TCP interface used to forward one-click installs to a running
/// instance.
pub const INTERFACE: ssilide::Interface = ssilide::Interface::new(6666);

/// Sender half of the global one-click channel, initialized by
/// [`start_listener`].
pub static ONECLICK_SENDER: OnceLock<flume::Sender<IpcMessage>> = OnceLock::new();

/// A message received over the one-click channel.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum IpcMessage {
    OpenMod(PathBuf),
    Error(String),
    Starting(String),
}

impl IpcMessage {
    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap_or_default()
    }
}

/// Claims the one-click port and forwards every incoming message to the
/// returned channel. Call this before [`oneclick`] / [`open_mod_arg`]; the
/// frontend should drain the receiver and map [`IpcMessage`] to its own
/// message type.
pub fn start_listener() -> flume::Receiver<IpcMessage> {
    let (tx, rx) = flume::unbounded();
    ONECLICK_SENDER.set(tx).unwrap_or(());
    std::thread::spawn(move || {
        match INTERFACE.claim() {
            Ok(sock) => {
                let mut buf = [0; 1024];
                loop {
                    match sock.recv(&mut buf) {
                        Ok(len) => {
                            log::debug!("Received 1-click install message");
                            let msg: IpcMessage = match serde_json::from_slice(&buf[..len])
                                .with_context(|| {
                                    String::from_utf8_lossy(&buf[..len]).into_owned()
                                }) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    log::error!("Broken IPC message: {e:?}");
                                    continue;
                                }
                            };
                            log::trace!("{:?}", &msg);
                            let sent = ONECLICK_SENDER
                                .get()
                                .map(|sender| sender.send(msg))
                                .unwrap_or(Ok(()));
                            if sent.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("IPC error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                let msg = format!(
                    "One-Click listener failed to initialize. Is UKMM already open?\n{e}"
                );
                log::error!("{msg}");
                if let Some(sender) = ONECLICK_SENDER.get() {
                    let _ = sender.send(IpcMessage::Error(msg));
                }
            }
        }
    });
    rx
}

/// Processes a `bcml:` one-click URL: resolves the mod name, downloads the
/// mod to a temp file, and returns the message to deliver to the app.
pub fn process_url(url: &str) -> IpcMessage {
    let mut parts = url.split(',');
    let url = parts.next().unwrap_or_default().to_owned();
    let cat = parts.next().unwrap_or_default().to_owned();
    let id = parts.next().unwrap_or_default().to_owned();
    log::debug!("Processing GameBanana 1-click URL: {url}");
    log::debug!("Checking mod name from API");
    let mod_name = http_get(&format!(
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
    let msg = (|url: String| -> Result<IpcMessage> {
        let redir = http_redirect_location(&url)?;
        let filename = http_req::uri::Uri::try_from(redir.as_str())?
            .path()
            .unwrap_or_default()
            .split('/')
            .last()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| format!("{mod_name}.bnp"));
        let data = http_get(&redir)
            .with_context(|| format!("Failed to download mod from {redir}"))?;
        let tmp = get_temp_file().with_file_name(filename);
        log::debug!("Saving mod to temp file at {}", tmp.display());
        fs_err::write(tmp.as_path(), data).context("Failed to save mod to temp file")?;
        log::info!("Finished downloading {mod_name}");
        Ok(IpcMessage::OpenMod(tmp.to_path_buf()))
    })(url)
    .map_err(|e| IpcMessage::Error(e.to_string()))
    .unwrap_or_else(|e| e);
    log::debug!("1-click mod downloaded, sending to UI for install");
    msg
}

/// Handles a `bcml:` URL from the command line: if another UKMM instance is
/// running, the mod is forwarded to it and this process exits; otherwise the
/// download is queued for the instance being started via [`start_listener`].
pub fn oneclick(url: &str) {
    match INTERFACE.connect() {
        Ok(client) => {
            let msg = process_url(url);
            if client.send(&msg.into_bytes()).is_err() {
                log::error!("Failed to send mod to existing UKMM instance");
            }
            std::process::exit(0);
        }
        Err(_) => {
            let url = url.to_owned();
            std::thread::spawn(move || {
                let msg = process_url(&url);
                match ONECLICK_SENDER.get() {
                    Some(sender) => {
                        if sender.send(msg).is_err() {
                            log::error!("Broken one-click channel");
                        }
                    }
                    None => log::error!("One-click channel not initialized"),
                }
            });
        }
    }
}

/// Queues a mod path from the command line (file drop / OS "open with") for
/// the running instance.
pub fn open_mod_arg(path: PathBuf) {
    if path.exists() {
        std::thread::spawn(move || {
            log::info!("Opening mod at {} for installation…", path.display());
            match ONECLICK_SENDER.get() {
                Some(sender) => {
                    if sender.send(IpcMessage::OpenMod(path)).is_err() {
                        log::error!("Broken one-click channel");
                    }
                }
                None => log::error!("One-click channel not initialized"),
            }
        });
    }
}
