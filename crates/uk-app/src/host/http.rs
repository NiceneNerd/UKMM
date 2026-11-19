//! Minimal HTTP helpers used by update checks and one-click installs.

use std::time::Duration;

use anyhow_ext::{Context, Result};

/// Overall timeout for large downloads (mod files, release assets).
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Overall timeout for small API calls (release list, mod name lookup).
pub const API_TIMEOUT: Duration = Duration::from_secs(10);

/// GETs `url`, following redirects, and returns the response body. Bounded by
/// [`DOWNLOAD_TIMEOUT`] so a stuck request cannot hog the caller forever.
pub fn http_get(url: &str) -> Result<Vec<u8>> {
    http_get_with_timeout(url, DOWNLOAD_TIMEOUT)
}

/// Like [`http_get`], with an explicit overall request timeout.
pub fn http_get_with_timeout(url: &str, timeout: Duration) -> Result<Vec<u8>> {
    let url = url.try_into()?;
    let mut buf = Vec::new();
    let mut req = http_req::request::Request::new(&url);
    req.header("User-Agent", "UKMM")
        .method(http_req::request::Method::GET)
        .timeout(timeout);
    req.send(&mut buf)
        .context("HTTP request failed")
        .and_then(|res| {
            if let Some(location) = res
                .status_code()
                .is_redirect()
                .then(|| res.headers().get("Location"))
                .flatten()
            {
                http_get_with_timeout(location, timeout)
            } else {
                Ok(buf)
            }
        })
}

/// GETs `url` without following redirects and returns the `Location` header
/// of the redirect response.
pub fn http_redirect_location(url: &str) -> Result<String> {
    let mut data = vec![];
    let uri = url.try_into()?;
    let mut req = http_req::request::Request::new(&uri);
    req.method(http_req::request::Method::GET)
        .header("User-Agent", "UKMM")
        .redirect_policy(http_req::request::RedirectPolicy::Limit(0))
        .timeout(DOWNLOAD_TIMEOUT);
    let res = req
        .send(&mut data)
        .with_context(|| format!("Failed to download from {url}"))?;
    let location = res
        .headers()
        .get("Location")
        .context("No location for redirect")?;
    Ok(location.to_owned())
}
