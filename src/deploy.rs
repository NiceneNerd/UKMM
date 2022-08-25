use crate::{
    mods,
    settings::{DeployConfig, DeployMethod, Platform, Settings},
    util::{self, HashMap},
};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use parking_lot::RwLock;
use rayon::prelude::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use roead::yaz0::{compress, decompress};
use rstb::ResourceSizeTable;
use smartstring::alias::String;
use std::{
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::Weak,
    time::SystemTime,
};
use uk_mod::{
    unpack::{ModReader, ModUnpacker},
    Manifest,
};

#[inline(always)]
fn create_symlink(link: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

pub struct Manager {
    settings: Weak<Settings>,
    mod_manager: Weak<mods::Manager>,
    pending_files: RwLock<Manifest>,
    pending_delete: RwLock<Manifest>,
    last_deploy: RwLock<SystemTime>,
}

impl Manager {
    pub fn deploy(&self) -> Result<()> {
        let settings = self
            .settings
            .upgrade()
            .expect("YIKES, the settings manager is gone");
        let config = settings
            .platform_config()
            .and_then(|c| c.deploy_config.as_ref())
            .context("No deployment config for current platform")?;
        log::debug!("Deployment config:\n{:?}", &config);
        if config.method == DeployMethod::Symlink {
            log::info!("Depoyment method is symlink, no action needed");
        } else {
            let mut deletes = self.pending_delete.write();
            todo!()
        }

        *self.last_deploy.write() = std::time::SystemTime::now();
        Ok(())
    }

    fn handle_orphans(
        &self,
        total_manifest: Manifest,
        manifest: &mut Manifest,
        out_dir: &Path,
        platform: Platform,
    ) -> Result<()> {
        let (orphans_content, orphans_aoc): (Vec<_>, Vec<_>) = (
            manifest
                .content_files
                .difference(&total_manifest.content_files)
                .cloned()
                .collect(),
            manifest
                .aoc_files
                .difference(&total_manifest.aoc_files)
                .cloned()
                .collect(),
        );
        if orphans_content.is_empty() && orphans_aoc.is_empty() {
            log::debug!("No orphans");
            return Ok(());
        }
        log::debug!(
            "Orphans to delete:\n{:?}\n{:?}",
            &orphans_content,
            &orphans_aoc
        );
        manifest
            .content_files
            .retain(|f| !orphans_content.contains(f));
        manifest.aoc_files.retain(|f| !orphans_aoc.contains(f));
        let mut dels = self.pending_delete.write();
        dels.content_files.extend(orphans_content.iter().cloned());
        dels.aoc_files.extend(orphans_aoc.iter().cloned());
        let (content, dlc) = uk_content::platform_prefixes(platform.into());
        for (dir, orphans) in [(content, orphans_content), (dlc, orphans_aoc)] {
            let out_dir = out_dir.join(dir);
            orphans.into_par_iter().try_for_each(|f| -> Result<()> {
                fs::remove_file(&out_dir.join(f.as_str()))
                    .with_context(|| jstr!("Failed to delete orphan file {f.as_str()}"))?;
                Ok(())
            })?;
        }
        log::info!("Deleted orphans");
        Ok(())
    }

    fn apply_rstb(
        &self,
        merged: &Path,
        platform: Platform,
        updates: HashMap<String, Option<u32>>,
    ) -> Result<()> {
        const RSTB_PATH: &str = "ResourceSizeTable.product.srsizetable";
        log::debug!("RSTB updates:\n{:?}", &updates);
        let content = uk_content::platform_content(platform.into());
        let table_path = merged.join(content).join(RSTB_PATH);
        let mut table = if table_path.exists() {
            ResourceSizeTable::from_binary(
                decompress(fs::read(&table_path).context("Failed to open merged RSTB")?)
                    .context("Failed to decompress merged RSTB")?,
            )
            .context("Failed to parse merged RSTB")?
        } else {
            ResourceSizeTable::new_from_stock(platform.into())
        };
        for (canon, size) in updates {
            match size {
                Some(size) => table.set(canon.as_str(), size),
                None => {
                    table.remove(canon.as_str());
                }
            }
        }
        log::info!("Updated RSTB");
        fs::write(table_path, compress(table.to_binary(platform.into())))
            .context("Failed to write merged RSTB")?;
        self.pending_files
            .write()
            .content_files
            .insert(RSTB_PATH.into());
        Ok(())
    }

    pub fn apply(&self, manifest: Option<Manifest>) -> Result<()> {
        let mod_manager = self
            .mod_manager
            .upgrade()
            .expect("YIKES, the mod manager system is gone");
        let settings = self
            .settings
            .upgrade()
            .expect("YIKES, the settings manager is gone");
        let dump = settings
            .dump()
            .context("No dump available for current platform")?;
        let endian = settings.current_mode.into();
        let out_dir = settings.merged_dir();
        let unpacker = if let Some(mut manifest) = manifest {
            let mut total_manifest = Manifest::default();
            let mods = mod_manager
                .mods_by_manifest(&manifest)
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .with_context(|| jstr!("Failed to open mod: {&m.meta.name}"))
                        .map(|m| {
                            total_manifest.extend(&m.manifest);
                            m
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            self.handle_orphans(
                total_manifest,
                &mut manifest,
                &out_dir,
                settings.current_mode,
            )?;
            log::debug!("Change manifest: {:?}", &manifest);
            self.pending_files.write().extend(&manifest);
            ModUnpacker::new(dump, endian, mods, out_dir.clone()).with_manifest(manifest)
        } else {
            let mods = mod_manager
                .mods()
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .with_context(|| jstr!("Failed to open mod: {&m.meta.name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            util::remove_dir_all(&out_dir).context("Failed to clear merged folder")?;
            ModUnpacker::new(dump, endian, mods, out_dir.clone())
        };
        log::info!("Applying changes");
        let rstb_updates = unpacker.unpack()?;
        self.apply_rstb(&out_dir, settings.current_mode, rstb_updates)?;
        Ok(())
    }
}
