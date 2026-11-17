//! Minimal HTTP helpers used by update checks and one-click installs.

use anyhow_ext::{Context, Result};

/// GETs `url`, following redirects, and returns the response body.
pub fn http_get(url: &str) -> Result<Vec<u8>> {
    let url = url.try_into()?;
    let mut buf = Vec::new();
    http_req::request::Request::new(&url)
        .header("User-Agent", "UKMM")
        .method(http_req::request::Method::GET)
        .send(&mut buf)
        .context("HTTP request failed")
        .and_then(|res| {
            if let Some(location) = res
                .status_code()
                .is_redirect()
                .then(|| res.headers().get("Location"))
                .flatten()
            {
                http_get(location)
            } else {
                Ok(buf)
            }
        })
}

/// GETs `url` without following redirects and returns the `Location` header
/// of the redirect response.
pub fn http_redirect_location(url: &str) -> Result<String> {
    let mut data = vec![];
    let res = http_req::request::Request::new(&url.try_into()?)
        .method(http_req::request::Method::GET)
        .header("User-Agent", "UKMM")
        .redirect_policy(http_req::request::RedirectPolicy::Limit(0))
        .send(&mut data)
        .with_context(|| format!("Failed to download from {url}"))?;
    let location = res
        .headers()
        .get("Location")
        .context("No location for redirect")?;
    Ok(location.to_owned())
}
