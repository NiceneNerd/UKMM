use crate::{
    mods,
    settings::{DeployMethod, Platform, Settings},
    util::{self, HashMap},
};
use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use parking_lot::RwLock;
use rayon::prelude::*;
use roead::yaz0::{compress, decompress};
use rstb::ResourceSizeTable;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
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

#[derive(Debug, Default, Serialize, Deserialize)]
struct PendingLog {
    files: Manifest,
    delete: Manifest,
}

#[derive(Debug)]
pub struct Manager {
    settings: Weak<RwLock<Settings>>,
    mod_manager: Weak<mods::Manager>,
    pending_files: RwLock<Manifest>,
    pending_delete: RwLock<Manifest>,
}

impl Manager {
    #[inline(always)]
    fn log_path(settings: &Settings) -> PathBuf {
        settings.platform_dir().join("pending.yml")
    }

    pub fn init(
        settings: &Arc<RwLock<Settings>>,
        mod_manager: &Arc<mods::Manager>,
    ) -> Result<Self> {
        log::info!("Initializing deployment manager");
        let pending = match fs::read_to_string(Self::log_path(&settings.read()))
            .map_err(anyhow::Error::from)
            .and_then(|text| Ok(serde_yaml::from_str::<PendingLog>(&text)?))
        {
            Ok(log) => {
                if !log.files.is_empty() || !log.delete.is_empty() {
                    log::info!("Pending deployment data found");
                    log::debug!("{:?}", &log);
                } else {
                    log::info!("No files pending deployment");
                }
                log
            }
            Err(e) => {
                log::warn!("Could not load pending deployment data:\n{}", &e);
                log::info!("No files pending deployment");
                Default::default()
            }
        };
        Ok(Self {
            settings: Arc::downgrade(settings),
            mod_manager: Arc::downgrade(mod_manager),
            pending_files: RwLock::new(pending.files),
            pending_delete: RwLock::new(pending.delete),
        })
    }

    #[inline]
    pub fn pending(&self) -> bool {
        !(self.pending_delete.read().is_empty() && self.pending_files.read().is_empty())
    }

    fn save(&self) -> Result<()> {
        fs::write(
            Self::log_path(&self.settings.upgrade().unwrap().read()),
            &serde_yaml::to_string(&PendingLog {
                delete: self.pending_delete.read().clone(),
                files: self.pending_files.read().clone(),
            })?,
        )?;
        Ok(())
    }

    pub fn deploy(&self) -> Result<()> {
        let settings = self
            .settings
            .upgrade()
            .expect("YIKES, the settings manager is gone");
        let settings = settings.read();
        let config = settings
            .platform_config()
            .and_then(|c| c.deploy_config.as_ref())
            .context("No deployment config for current platform")?;
        log::debug!("Deployment config:\n{:?}", &config);
        if config.method == DeployMethod::Symlink {
            log::info!("Depoyment method is symlink, no action needed");
        } else {
            let (content, aoc) = uk_content::platform_prefixes(settings.current_mode.into());
            let deletes = self.pending_delete.read();
            log::debug!("Deployed files to delete:\n{:?}", &deletes);
            let syncs = self.pending_files.read();
            log::debug!("Files to deploy\n{:?}", &syncs);
            for (dir, dels, syncs) in [
                (content, &deletes.content_files, &syncs.content_files),
                (aoc, &deletes.aoc_files, &syncs.aoc_files),
            ] {
                let dest = config.output.join(dir);
                let source = settings.merged_dir().join(dir);
                dels.par_iter().try_for_each(|f| -> Result<()> {
                    fs::remove_file(dest.join(f.as_str()))?;
                    Ok(())
                })?;
                match config.method {
                    DeployMethod::Copy => {
                        log::info!("Deploying by copy");
                        syncs.par_iter().try_for_each(|f: &String| -> Result<()> {
                            let out = dest.join(f.as_str());
                            fs::create_dir_all(out.parent().unwrap())?;
                            fs::copy(source.join(f.as_str()), &out).with_context(|| {
                                format!("Failed to deploy {} to {}", f, out.display())
                            })?;
                            Ok(())
                        })?;
                    }
                    DeployMethod::HardLink => {
                        log::info!("Deploying by hard links");
                        syncs.par_iter().try_for_each(|f: &String| -> Result<()> {
                            let out = dest.join(f.as_str());
                            fs::create_dir_all(out.parent().unwrap())?;
                            fs::hard_link(source.join(f.as_str()), dest.join(f.as_str()))
                                .with_context(|| {
                                    format!("Failed to deploy {} to {}", f, out.display())
                                })?;
                            Ok(())
                        })?;
                    }
                    DeployMethod::Symlink => unreachable!(),
                }
            }
            log::info!("Deployment complete");
        }
        self.pending_delete.write().clear();
        self.pending_files.write().clear();
        self.save()?;
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
                let file = out_dir.join(f.as_str());
                if file.exists() {
                    fs::remove_file(&file)
                        .with_context(|| jstr!("Failed to delete orphan file {f.as_str()}"))?;
                }
                let parent = file.parent().unwrap();
                if std::fs::read_dir(parent).unwrap().next().is_none() {
                    util::remove_dir_all(parent).unwrap_or(())
                }
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
        const RSTB_PATH: &str = "System/Resource/ResourceSizeTable.product.srsizetable";
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
        let settings = settings.try_read().unwrap();
        let dump = settings
            .dump()
            .context("No dump available for current platform")?;
        let endian = settings.current_mode.into();
        let out_dir = settings.merged_dir();
        let unpacker = if let Some(mut manifest) = manifest {
            log::info!("Manifest provided, applying limited changes");
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
            log::info!("Manifest not provided, remerging all mods");
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
        self.save()?;
        Ok(())
    }
}
