use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, LazyLock},
};

#[cfg(windows)]
use anyhow_ext::Context;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
pub use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

pub fn copy_dir<T: AsRef<Path>, U: AsRef<Path>>(src: T, dst: U) -> anyhow_ext::Result<()> {
    for p in jwalk::WalkDir::new(&src) {
        let from = p?.path();
        let to = dst.as_ref().join(from.strip_prefix(&src)?);
        if from.is_dir() {
            std::fs::create_dir(to)?;
        } else if from.is_file() {
            std::fs::copy(from, to)?;
        }
    }
    Ok(())
}

pub fn remove_dir_all(dir: impl AsRef<std::path::Path>) -> anyhow_ext::Result<()> {
    fn inner(dir: &Path) -> anyhow_ext::Result<()> {
        #[cfg(windows)]
        remove_dir_all::remove_dir_all(dir).with_context(|| dir.to_string_lossy().to_string())?;
        #[cfg(not(windows))]
        fs_err::remove_dir_all(dir)?;
        Ok(())
    }
    inner(dir.as_ref())
}

pub fn remove_symlink(link: impl AsRef<std::path::Path>) -> anyhow_ext::Result<()> {
    fn inner(link: &Path) -> anyhow_ext::Result<()> {
        #[cfg(windows)]
        fs_err::remove_dir(link)?;
        #[cfg(not(windows))]
        fs_err::remove_file(link)?;
        Ok(())
    }
    inner(link.as_ref())
}

static TEMP_FS: LazyLock<RwLock<HashSet<PathBuf>>> =
    LazyLock::new(|| RwLock::new(HashSet::default()));

#[allow(clippy::unwrap_used)]
pub fn get_temp_folder() -> MappedRwLockReadGuard<'static, PathBuf> {
    let temp = tempfile::tempdir().unwrap().into_path();
    TEMP_FS.write().insert(temp.clone());
    RwLockReadGuard::map(TEMP_FS.read(), |tmps| unsafe {
        tmps.get(&temp).unwrap_unchecked()
    })
}

#[allow(clippy::unwrap_used)]
pub fn get_temp_file() -> MappedRwLockReadGuard<'static, PathBuf> {
    let temp = tempfile::NamedTempFile::new().unwrap().keep().unwrap().1;
    TEMP_FS.write().insert(temp.clone());
    RwLockReadGuard::map(TEMP_FS.read(), |tmps| unsafe {
        tmps.get(&temp).unwrap_unchecked()
    })
}

pub fn clear_temp() {
    TEMP_FS.write().drain().for_each(|tmp| {
        if tmp.is_file() {
            fs_err::remove_file(tmp).unwrap_or(());
        } else {
            remove_dir_all(tmp).unwrap_or(());
        }
    });
}

pub static USE_SZ: AtomicBool = AtomicBool::new(true);

pub fn extract_7z(file: &Path, folder: &Path) -> anyhow_ext::Result<()> {
    static SZ_EXISTS: LazyLock<bool> = LazyLock::new(|| {
        match std::process::Command::new("7z")
            .stdout(std::process::Stdio::null())
            .spawn()
        {
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

    if *SZ_EXISTS && USE_SZ.load(std::sync::atomic::Ordering::Relaxed) {
        let output = std::process::Command::new("7z")
            .arg("x")
            .arg(file)
            .arg(format!("-o{}", folder.display()))
            .output()?;
        if !output.stderr.is_empty() {
            anyhow_ext::bail!("{}", std::string::String::from_utf8_lossy(&output.stderr))
        } else {
            Ok(())
        }
    } else {
        Ok(sevenz_rust::decompress_file(file, folder)?)
    }
}
