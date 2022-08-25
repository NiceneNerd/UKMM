#[cfg(windows)]
use anyhow::Context;
pub use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

pub fn remove_dir_all(dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    #[cfg(windows)]
    remove_dir_all::remove_dir_all(dir.as_ref())
        .with_context(|| dir.as_ref().to_string_lossy().to_string())?;
    #[cfg(not(windows))]
    fs_err::remove_dir_all(dir.as_ref())?;
    Ok(())
}
