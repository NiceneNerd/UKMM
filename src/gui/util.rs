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

pub struct SmartStringWrapper<'a>(pub &'a mut smartstring::alias::String);

impl uk_ui::egui::TextBuffer for SmartStringWrapper<'_> {
    #[inline]
    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn is_mutable(&self) -> bool {
        true
    }

    #[inline]
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let index = self.byte_index_from_char_index(char_index);
        self.0.insert_str(index, text);
        text.chars().count()
    }

    #[inline]
    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        assert!(char_range.start <= char_range.end);
        let start = self.byte_index_from_char_index(char_range.start);
        let end = self.byte_index_from_char_index(char_range.end);
        self.0.drain(start..end);
    }
}

pub fn default_shell() -> &'static std::sync::LazyLock<(std::path::PathBuf, Option<&'static str>)> {
    use which::which_global;
    static SHELL_PATH: std::sync::LazyLock<(std::path::PathBuf, Option<&'static str>)> =
        std::sync::LazyLock::new(|| {
            #[cfg(target_os = "windows")]
            {
                (
                    which_global("cmd.exe")
                        .or_else(|_| which_global("pwsh.exe"))
                        .or_else(|_| which_global("powershell.exe"))
                        .unwrap(),
                    None,
                )
            }
            #[cfg(target_os = "linux")]
            {
                (
                    std::env::var("SHELL")
                        .ok()
                        .and_then(|s| which_global(s).ok())
                        .or_else(|| which_global("sh").ok())
                        .unwrap(),
                    Some("-c"),
                )
            }
            #[cfg(target_os = "macos")]
            {
                (
                    which_global("zsh").or_else(|_| which_global("sh")).unwrap(),
                    Some("-c"),
                )
            }
        });
    &SHELL_PATH
}
