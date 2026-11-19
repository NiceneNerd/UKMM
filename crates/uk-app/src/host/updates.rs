//! Update checking and self-update installation against GitHub releases.

use anyhow_ext::{Context, Result};
use serde::{Deserialize, Serialize};
use uk_manager::{core::Manager, settings::UpdatePreference, util::get_temp_file};

use crate::{
    error::AppError,
    host::http::{API_TIMEOUT, http_get, http_get_with_timeout},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionResponse {
    pub body: String,
    pub name: String,
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<VersionAsset>,
}

impl VersionResponse {
    pub fn description(&self) -> String {
        format!(
            "# Release {} Notes\n\n**{}**\n\n{}",
            self.tag_name, self.name, self.body
        )
    }
}

/// What the frontend should do after checking for releases.
#[derive(Debug)]
pub enum ReleaseCheck {
    /// A newer release exists; offer it to the user.
    OfferUpdate(VersionResponse),
    /// We are ahead of the newest release; show its changelog.
    Changelog(String),
    /// Nothing to do (up to date, or the check failed).
    None,
}

/// Fetches the GitHub releases and compares them against the running version
/// and the user's update preference. Network/parse failures yield
/// [`ReleaseCheck::None`], never an error.
pub fn check_releases(core: &Manager) -> ReleaseCheck {
    let url = "https://api.github.com/repos/NiceneNerd/UKMM/releases?per_page=10";
    // Small API call: never let a stuck request hold the session worker.
    let mut releases = match http_get_with_timeout(url, API_TIMEOUT).and_then(|bytes| {
        serde_json::from_slice::<Vec<VersionResponse>>(&bytes)
            .context("Failed to parse GitHub response")
    }) {
        Ok(releases) => releases,
        Err(e) => {
            log::warn!("{:?}", e);
            return ReleaseCheck::None;
        }
    };
    let Ok(current_semver) = lenient_semver::parse(env!("CARGO_PKG_VERSION")) else {
        return ReleaseCheck::None;
    };
    let betas = core.settings().check_updates == UpdatePreference::Beta
        || lenient_semver::parse("1.0.0").is_ok_and(|v| current_semver < v);
    releases.retain(|r| !r.prerelease || betas);
    if let Some((release, release_ver)) = releases.first().and_then(|r| {
        lenient_semver::parse(r.tag_name.trim_start_matches('v'))
            .ok()
            .map(|v| (r, v))
    }) {
        match release_ver.cmp(&current_semver) {
            std::cmp::Ordering::Greater => ReleaseCheck::OfferUpdate(release.clone()),
            std::cmp::Ordering::Less => ReleaseCheck::Changelog(release.description()),
            std::cmp::Ordering::Equal => ReleaseCheck::None,
        }
    } else {
        ReleaseCheck::None
    }
}

/// What the frontend should do after installing an update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePlan {
    /// The executable was swapped; restart the app.
    Restart,
}

/// Downloads the platform asset of `version` and swaps it in place of the
/// running executable (the old one is kept as `<exe>.bak`).
pub fn install_update(version: VersionResponse) -> Result<UpdatePlan> {
    log::info!("Updating... UKMM will restart when complete");
    #[cfg(target_os = "windows")]
    let asset_name = "ukmm-x86_64-pc-windows-msvc.zip";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let asset_name = "ukmm-aarch64-apple-darwin.tar.xz";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let asset_name = "ukmm-x86_64-apple-darwin.tar.xz";
    #[cfg(target_os = "linux")]
    let asset_name = "ukmm-x86_64-unknown-linux-gnu.tar.xz";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    let asset_name = "";
    let asset = version
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or(AppError::UpdateUnavailable)?;
    let data = http_get(&asset.browser_download_url)?;
    let temp_file = get_temp_file();
    fs_err::write(temp_file.as_path(), data)?;
    let exe = std::env::current_exe()?;
    if cfg!(windows) {
        let mut arc = zip::ZipArchive::new(fs_err::File::open(temp_file.as_path())?)?;
        arc.extract(temp_file.parent().context("Weird, no temp file parent")?)?;
        fs_err::rename(&exe, exe.with_extension("bak"))?;
        fs_err::copy(temp_file.with_file_name("ukmm.exe"), exe)?;
    } else {
        fs_err::rename(&exe, exe.with_extension("bak"))?;
        let out = std::process::Command::new("tar")
            .arg("xf")
            .arg(temp_file.as_path())
            .arg("-C")
            .arg(exe.parent().context("Weird, no exe parent")?)
            .arg("--overwrite")
            .output()?;
        if !out.stderr.is_empty() {
            anyhow_ext::bail!(String::from_utf8_lossy(&out.stderr).to_string());
        }
    };
    Ok(UpdatePlan::Restart)
}
