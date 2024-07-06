use anyhow::{Context, Result};

pub fn response(url: &str) -> Result<Vec<u8>> {
    let url = url.try_into()?;
    let mut buf = Vec::new();
    http_req::request::Request::new(&url)
        .header("User-Agent", "UKMM")
        .method(http_req::request::Method::GET)
        .send(&mut buf)
        .context("HTTP request file")
        .and_then(|res| {
            if let Some(url) = res
                .status_code()
                .is_redirect()
                .then(|| res.headers().get("Location"))
                .flatten()
            {
                response(url)
            } else {
                Ok(buf)
            }
        })
}
