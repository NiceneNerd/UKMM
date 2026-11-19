//! Classification of a bare first CLI argument (no subcommand given), e.g. a
//! `bcml:` one-click URL or a mod path from the OS.

use std::path::PathBuf;

/// What the frontend should do with a bare CLI argument.
#[derive(Debug)]
pub enum ArgAction {
    /// A `bcml:` one-click URL (GameBanana).
    OneClick(String),
    /// A path to a mod file/folder to open.
    ModPath(PathBuf),
    /// No usable argument.
    None,
}

/// Classifies `arg` (typically `std::env::args().nth(1)`).
pub fn classify_arg(arg: Option<String>) -> ArgAction {
    match arg {
        Some(arg) => {
            match arg.strip_prefix("bcml:") {
                Some(url) => ArgAction::OneClick(url.to_owned()),
                None => ArgAction::ModPath(arg.into()),
            }
        }
        None => ArgAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bcml_url() {
        match classify_arg(Some("bcml:https://example.com/dl,mod,123".into())) {
            ArgAction::OneClick(url) => assert_eq!(url, "https://example.com/dl,mod,123"),
            other => panic!("expected OneClick, got {other:?}"),
        }
    }

    #[test]
    fn classifies_mod_path() {
        match classify_arg(Some("/tmp/mod.zip".into())) {
            ArgAction::ModPath(p) => assert_eq!(p, PathBuf::from("/tmp/mod.zip")),
            other => panic!("expected ModPath, got {other:?}"),
        }
    }

    #[test]
    fn missing_arg_is_none() {
        assert!(matches!(classify_arg(None), ArgAction::None));
    }
}
