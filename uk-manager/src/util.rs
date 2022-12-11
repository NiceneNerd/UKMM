use std::path::PathBuf;

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
