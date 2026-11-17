use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use uk_manager::{bnp::convert_bnp, core::Manager, mods::Mod};
use uk_mod::{
    Meta,
    unpack::{ModReader, OpenError},
};

use crate::sniff::{is_probably_a_bnp, is_probably_a_mod_and_has_meta};

/// Result of opening an arbitrary path as a mod.
#[derive(Debug)]
pub enum Opened {
    Mod(Mod),
    /// A legacy-format mod that carries no metadata. Call [`open_mod`] again
    /// with the `Meta` the user supplies.
    NeedsMeta(PathBuf),
}

/// Opens a UKMM mod, converting BNP or legacy-format mods on the way.
pub fn open_mod(core: &Manager, path: &Path, meta: Option<Meta>) -> Result<Opened> {
    log::info!("Opening mod at {}", path.display());
    if is_probably_a_bnp(path) {
        let mod_ = convert_bnp(core, path).context("Failed to convert BNP to UKMM mod")?;
        return open_converted(mod_);
    }
    let err = match ModReader::open_peek(path, vec![]) {
        Ok(reader) => return Ok(Opened::Mod(Mod::from_reader(reader))),
        Err(err) => err,
    };
    log::warn!("Could not open mod, let's find out why");
    let Some(has_meta) = OpenError::is_not_ukmm_mod(&err)
        .then(|| is_probably_a_mod_and_has_meta(path))
        .filter(|(is_mod, _has_meta)| *is_mod)
        .map(|(_, has_meta)| has_meta)
    else {
        return Err(err.context(format!("Failed to open mod {}", path.display())));
    };
    log::info!("Maybe it's not a UKMM mod, let's try to convert it");
    if !has_meta && meta.is_none() {
        log::info!("Mod has no meta info, requesting manual input");
        return Ok(Opened::NeedsMeta(path.to_path_buf()));
    }
    if let Some(ref meta) = meta {
        log::info!("Converting mod {}…", meta.name);
    }
    let converted = uk_manager::mods::convert_gfx(core, path, meta).with_context(|| {
        format!(
            "Failed to convert {}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
        )
    })?;
    open_converted(converted)
}

fn open_converted(path: impl AsRef<Path>) -> Result<Opened> {
    ModReader::open_peek(path, vec![])
        .context("Failed to open converted mod")
        .map(|reader| Opened::Mod(Mod::from_reader(reader)))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn zip_with(dir: &Path, name: &str, entries: &[(&str, &str)]) -> PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (entry, body) in entries {
            zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn is_not_ukmm(path: &Path) -> bool {
        let err = ModReader::open_peek(path, vec![]).expect_err("fixture must not open");
        OpenError::is_not_ukmm_mod(&err)
    }

    fn temp_core() -> (tempfile::TempDir, Manager) {
        let dir = tempfile::tempdir().unwrap();
        let settings = uk_manager::settings::Settings::load_from(
            &dir.path().join("config").join("settings.yml"),
        );
        let core = Manager::init_with(settings).unwrap();
        (dir, core)
    }

    fn zip_owned(dir: &Path, name: &str, entries: Vec<(String, String)>) -> PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (entry, body) in entries {
            zip.start_file(entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn ukmm_mod_zip(dir: &Path, name: &str) -> PathBuf {
        let mut meta = Meta::blank(uk_mod::ModPlatform::Universal);
        meta.name = "Test Mod".into();
        zip_owned(dir, name, vec![
            ("meta.yml".into(), serde_yaml::to_string(&meta).unwrap()),
            (
                "manifest.yml".into(),
                serde_yaml::to_string(&uk_mod::Manifest::default()).unwrap(),
            ),
        ])
    }

    fn legacy_mod_zip(dir: &Path, name: &str) -> PathBuf {
        zip_owned(dir, name, vec![
            ("content/a.txt".into(), "x".into()),
            ("rules.txt".into(), "x".into()),
        ])
    }

    #[test]
    fn ukmm_zip_opens_as_mod() {
        let (dir, core) = temp_core();
        let path = ukmm_mod_zip(dir.path(), "mod.zip");
        match open_mod(&core, &path, None).unwrap() {
            Opened::Mod(mod_) => assert_eq!(mod_.meta.name.as_str(), "Test Mod"),
            Opened::NeedsMeta(_) => panic!("UKMM mod should not need meta"),
        }
    }

    #[test]
    fn legacy_zip_without_meta_needs_meta() {
        let (dir, core) = temp_core();
        // a legacy mod with content but no rules.txt needs manual metadata
        let path = zip_owned(dir.path(), "legacy.zip", vec![(
            "content/a.txt".into(),
            "x".into(),
        )]);
        match open_mod(&core, &path, None).unwrap() {
            Opened::NeedsMeta(p) => assert_eq!(p, path),
            Opened::Mod(_) => panic!("legacy mod should not open as UKMM mod"),
        }
    }

    #[test]
    fn legacy_zip_with_rules_txt_converts_without_needing_meta() {
        let (dir, core) = temp_core();
        // rules.txt is the legacy metadata, so no meta prompt — but conversion
        // still needs a dump, which the temp config does not have
        let path = legacy_mod_zip(dir.path(), "legacy.zip");
        let err = open_mod(&core, &path, None).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("No dump available"),
            "conversion should require a dump, got: {msg}"
        );
    }

    #[test]
    fn legacy_zip_with_meta_reaches_conversion_and_needs_dump() {
        let (dir, core) = temp_core();
        let path = legacy_mod_zip(dir.path(), "legacy.zip");
        let meta = Meta::blank(uk_mod::ModPlatform::Universal);
        let err = open_mod(&core, &path, Some(meta)).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("No dump available"),
            "conversion should require a dump, got: {msg}"
        );
    }

    #[test]
    fn junk_file_is_not_a_conversion_candidate() {
        let (dir, core) = temp_core();
        let path = dir.path().join("junk.zip");
        std::fs::write(&path, b"not a zip at all").unwrap();
        let err = open_mod(&core, &path, None).unwrap_err();
        assert!(!err.to_string().contains("Failed to convert"));
    }

    #[test]
    fn missing_path_is_not_a_ukmm_mod() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_not_ukmm(&dir.path().join("nope")));
    }

    #[test]
    fn non_zip_file_is_not_a_ukmm_mod() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junk.zip");
        std::fs::write(&path, b"this is not a zip archive at all").unwrap();
        assert!(is_not_ukmm(&path));
    }

    #[test]
    fn zip_without_meta_is_not_a_ukmm_mod() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with(dir.path(), "legacy.zip", &[("content/a.txt", "x")]);
        assert!(is_not_ukmm(&path));
    }

    #[test]
    fn malformed_meta_is_a_real_error_not_a_conversion_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with(dir.path(), "broken.zip", &[("meta.yml", "name: [unclosed")]);
        assert!(!is_not_ukmm(&path));
    }
}
