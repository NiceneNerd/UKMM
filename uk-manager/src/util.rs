use std::path::{Path, PathBuf};

#[cfg(windows)]
use anyhow::Context;
use once_cell::sync::Lazy;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
pub use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

pub fn remove_dir_all(dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    #[cfg(windows)]
    remove_dir_all::remove_dir_all(dir.as_ref())
        .with_context(|| dir.as_ref().to_string_lossy().to_string())?;
    #[cfg(not(windows))]
    fs_err::remove_dir_all(dir.as_ref())?;
    Ok(())
}

static TEMP_FOLDERS: Lazy<RwLock<HashSet<PathBuf>>> = Lazy::new(|| RwLock::new(HashSet::default()));

pub fn get_temp_folder() -> MappedRwLockReadGuard<'static, PathBuf> {
    let temp = tempfile::tempdir().unwrap().into_path();
    TEMP_FOLDERS.write().insert(temp.clone());
    RwLockReadGuard::map(TEMP_FOLDERS.read(), |tmps| unsafe {
        tmps.get(&temp).unwrap_unchecked()
    })
}

pub fn clear_temp() {
    TEMP_FOLDERS.write().iter().for_each(|tmp| {
        remove_dir_all(tmp).unwrap_or(());
    });
}

pub fn extract_7z(file: &Path, folder: &Path) -> anyhow::Result<()> {
    static SX_EXISTS: Lazy<bool> = Lazy::new(|| {
        match std::process::Command::new("7z").spawn() {
            Ok(_) => {
                log::trace!("7z found");
                true
            }
            Err(_) => {
                log::trace!("7z not found");
                false
            }
        }
    });

    if *SX_EXISTS {
        let output = std::process::Command::new("7z")
            .arg("x")
            .arg(file)
            .arg(format!("-o{}", folder.display()))
            .output()?;
        if !output.stderr.is_empty() {
            anyhow::bail!("{}", std::string::String::from_utf8_lossy(&output.stderr))
        } else {
            Ok(())
        }
    } else {
        Ok(sevenz_rust::decompress_file(file, folder)?)
    }
}
