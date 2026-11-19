//! GameBanana one-click install IPC (ssilide, local TCP port 6666) and
//! handling of `bcml:` URLs passed on the command line.

use std::{path::PathBuf, sync::OnceLock};

use anyhow_ext::{Context, Result};
use uk_manager::util::get_temp_file;
use uk_mod::pack::sanitise;

use crate::host::http::{API_TIMEOUT, http_get, http_get_with_timeout, http_redirect_location};

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
    /// A raw `bcml:` URL; the receiving instance downloads and installs it
    /// itself, as a session job with proper busy/progress reporting.
    OneClick(String),
    Error(String),
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
                                .with_context(|| String::from_utf8_lossy(&buf[..len]).into_owned())
                            {
                                Ok(msg) => msg,
                                Err(e) => {
                                    log::error!("Broken IPC message: {e:?}");
                                    continue;
                                }
                            };
                            log::trace!("{msg:?}");
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
                let msg =
                    format!("One-Click listener failed to initialize. Is UKMM already open?\n{e}");
                log::error!("{msg}");
                if let Some(sender) = ONECLICK_SENDER.get() {
                    let _ = sender.send(IpcMessage::Error(msg));
                }
            }
        }
    });
    rx
}

/// Resolves the display name of the mod a `bcml:` URL points at (best
/// effort — falls back to a generic name).
fn mod_name_for(url: &str) -> String {
    let mut parts = url.split(',');
    parts.next();
    let cat = parts.next().unwrap_or_default().to_owned();
    let id = parts.next().unwrap_or_default().to_owned();
    http_get_with_timeout(
        &format!(
            "https://api.gamebanana.com/Core/Item/Data?itemtype={cat}&itemid={id}&fields=name"
        ),
        API_TIMEOUT,
    )
    .and_then(|data| Ok(serde_json::from_slice::<Vec<String>>(&data)?))
    .map(|mut res| sanitise(&res.remove(0)))
    .unwrap_or_else(|_| "oneclick_mod".into())
}

/// Downloads the mod a `bcml:` URL points at to a temp file. Returns the
/// temp path and the mod's display name. Runs as a session job, so busy and
/// progress are reported through the normal job lifecycle.
pub fn download_oneclick(url: &str) -> Result<(PathBuf, String)> {
    let url = url.split(',').next().unwrap_or_default().to_owned();
    if url.is_empty() {
        anyhow_ext::bail!("Invalid one-click URL");
    }
    log::debug!("Processing GameBanana 1-click URL: {url}");
    let mod_name = mod_name_for(&url);
    log::info!("Downloading {mod_name} from GameBanana 1-click…");
    let redir = http_redirect_location(&url)?;
    let filename = http_req::uri::Uri::try_from(redir.as_str())?
        .path()
        .unwrap_or_default()
        .split('/')
        .next_back()
        .map(|n| n.to_owned())
        .unwrap_or_else(|| format!("{mod_name}.bnp"));
    let data = http_get(&redir).with_context(|| format!("Failed to download mod from {redir}"))?;
    let tmp = get_temp_file().with_file_name(filename);
    log::debug!("Saving mod to temp file at {}", tmp.display());
    fs_err::write(tmp.as_path(), data).context("Failed to save mod to temp file")?;
    log::info!("Finished downloading {mod_name}");
    Ok((tmp.to_path_buf(), mod_name))
}

/// Handles a `bcml:` URL from the command line: if another UKMM instance is
/// running, the URL is forwarded to it and this process exits; otherwise the
/// URL is queued for the instance being started via [`start_listener`].
pub fn oneclick(url: &str) {
    match INTERFACE.connect() {
        Ok(client) => {
            let msg = IpcMessage::OneClick(url.to_owned());
            if client.send(&msg.into_bytes()).is_err() {
                log::error!("Failed to send one-click URL to existing UKMM instance");
            }
            std::process::exit(0);
        }
        Err(_) => {
            let url = url.to_owned();
            std::thread::spawn(move || {
                let msg = IpcMessage::OneClick(url);
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
