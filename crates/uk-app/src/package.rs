use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use uk_manager::{core::Manager, mods::Mod};
use uk_mod::{
    Manifest, Meta,
    pack::ModPacker,
    unpack::{ModReader, ModUnpacker},
};

pub fn package_mod(
    core: &Manager,
    source: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    meta: Meta,
) -> Result<()> {
    let Some(dump) = core.settings().dump() else {
        anyhow::bail!("No dump for current platform")
    };
    ModPacker::new(
        source.as_ref(),
        dest.as_ref(),
        Some(meta),
        [dump].into_iter().collect(),
    )
    .context("Failed to initialize mod packager")?
    .pack()
    .context("Failed to package mod")?;
    Ok(())
}

/// Repacks `mod_` in place from the loose files in `source` and swaps it into
/// the mod manager. Returns the manifest of files affected by the change,
/// which the caller should fold into the pending set for the next merge.
pub fn dev_update_mod(core: &Manager, mod_: Mod, source: impl AsRef<Path>) -> Result<Manifest> {
    let mut dirty = Manifest::default();
    dirty.extend(&mod_.manifest().unwrap_or_default());
    let hash = mod_.hash();
    let Some(dump) = core.settings().dump() else {
        anyhow::bail!("No dump for current platform")
    };
    ModPacker::new(
        source.as_ref(),
        &mod_.path,
        None,
        [dump].into_iter().collect(),
    )
    .context("Failed to initialize mod packager")?
    .pack()
    .context("Failed to package mod")?;
    let new_mod = ModReader::open_peek(mod_.path, vec![])?;
    dirty.extend(new_mod.manifest());
    core.mod_manager_mut()
        .replace(Mod::from_reader(new_mod), hash)?;
    Ok(dirty)
}

/// Repacks each mod in place from the loose folder at the same index in
/// `folders` (collected by the frontend) and swaps it into the mod manager.
/// Returns the manifest of all files affected, for the next merge.
pub fn dev_update_mods(core: &Manager, mods: Vec<Mod>, folders: Vec<PathBuf>) -> Result<Manifest> {
    if mods.len() != folders.len() {
        anyhow::bail!(
            "Got {} mods but {} source folders",
            mods.len(),
            folders.len()
        );
    }
    let mut dirty = Manifest::default();
    for (mod_, folder) in mods.into_iter().zip(folders) {
        log::info!("Updating {}…", mod_.meta.name.as_str());
        dirty.extend(&dev_update_mod(core, mod_, folder)?);
    }
    Ok(dirty)
}

/// Unpacks each mod into `dest/<mod name>`. A failure in one mod does not stop
/// the others; all failures are reported together.
pub fn extract_mods(core: &Manager, mods: Vec<Mod>, dest: impl AsRef<Path>) -> Result<()> {
    let settings = core.settings();
    let config = settings
        .platform_config()
        .context("No config for current platform. Have you configured your settings?")?;
    let mut errors = vec![];
    for mod_ in mods {
        let name = mod_.meta.name.as_str();
        log::info!("Extracting {}…", name);
        let unpacker = ModUnpacker::new(
            config.dump.clone(),
            settings.current_mode.into(),
            config.language,
            vec![ModReader::open(&mod_.path, mod_.enabled_options.clone())?],
            dest.as_ref().join(name),
        );
        if let Err(e) = unpacker.unpack() {
            log::error!("{e:?}");
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "One or more mods encountered errors when extracting. Details below:\n{}",
            errors.into_iter().fold(String::new(), |mut acc, e| {
                writeln!(acc, "{:?}", e).expect("Failed to write to String");
                acc
            })
        )
    }
}

/// Reads mod metadata from a BCML-style `rules.txt`, an `info.json`, or a UKMM
/// `meta.yml`, chosen by file extension.
pub fn parse_meta(file: impl AsRef<Path>) -> Result<Meta> {
    let file = file.as_ref();
    match file.extension().and_then(|x| x.to_str()) {
        Some("txt") => ModPacker::parse_rules(file.to_path_buf()),
        Some("yml") => Meta::parse(file),
        Some("json") => ModPacker::parse_info(file.to_path_buf()),
        _ => anyhow::bail!("Unsupported metadata file: {}", file.display()),
    }
}

#[cfg(test)]
mod tests {
    use uk_mod::ModPlatform;

    use super::*;

    #[test]
    fn unknown_meta_extension_is_an_error_not_a_panic() {
        assert!(parse_meta("meta.toml").is_err());
        assert!(parse_meta("no_extension").is_err());
    }

    #[test]
    fn yml_meta_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut meta = Meta::blank(ModPlatform::Universal);
        meta.name = "Round Trip".into();
        let path = dir.path().join("meta.yml");
        std::fs::write(&path, serde_yaml::to_string(&meta).unwrap()).unwrap();
        assert_eq!(parse_meta(&path).unwrap(), meta);
    }
}
