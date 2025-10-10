use std::path::PathBuf;
use anyhow::anyhow;
use anyhow_ext::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use smartstring::alias::String;

#[serde_as]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct File {
    name: String,
}

impl From<String> for File {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl TryFrom<&PathBuf> for File {
    type Error = anyhow_ext::Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() || path.is_dir() {
            Err(anyhow!("{:?} is not a valid file", path))
        }
        else {
            Ok(Self {
                name: path.file_name()
                    .ok_or(anyhow!("File name is empty"))?
                    .to_string_lossy()
                    .into()
            })
        }
    }
}

impl File {
    #[inline(always)]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn should_move(&self, from: &PathBuf, to: &PathBuf) -> Result<bool> {
        let old = from.join(self.name.as_str());
        let new = to.join(self.name.as_str());
        if !old.exists() {
            Ok(false)
        }
        else if !new.exists() {
            Ok(true)
        }
        else if old.metadata()?.modified()? != new.metadata()?.modified()? {
            Ok(true)
        }
        //else if old.metadata()?.created()? > new.metadata()?.created()? {
        //    Ok(true)
        //}
        else {
            Ok(false)
        }
    }

    #[inline(always)]
    pub fn should_delete(&self, from: &PathBuf, based_on: &PathBuf) -> Result<bool> {
        Ok(from.join(self.name.as_str()).exists() && !based_on.join(self.name.as_str()).exists())
    }

    pub fn copy(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        let old = from.join(self.name.as_str());
        let new = to.join(self.name.as_str());
        if old.exists() {
            std::fs::copy(&old, &new)
                .with_context(|| format!("Failed to deploy {} to {}", self.name, &new.display()))?;
            std::fs::File::options()
                .write(true)
                .open(new)?
                .set_modified(std::fs::metadata(old)?.modified()?)?;
        } else {
            log::warn!(
                "Source file {} missing, we're assuming it was a deletion lost track of",
                old.display()
            );
            if new.exists() {
                std::fs::remove_file(&new)?;
            }
        }
        Ok(())
    }

    pub fn hard_link(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        let old = from.join(self.name.as_str());
        let new = to.join(self.name.as_str());
        if new.exists() {
            std::fs::remove_file(&new)?;
        }
        if old.exists() {
            std::fs::hard_link(old, &new)
                .with_context(|| format!("Failed to deploy {} to {}", self.name, &new.display()))
                .map_err(|e| {
                    if e.root_cause().to_string().contains("os error 17") {
                        e.context(
                            "Hard linking failed because the output folder is on a \
                             different disk or partition than the storage folder.",
                        )
                    } else {
                        e
                    }
                })?;
        } else {
            log::warn!(
                "Source file {} missing, we're assuming it was a deletion lost track of",
                old.display()
            );
        }
        Ok(())
    }
}