#![allow(clippy::unwrap_used, unstable_name_collisions)]

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use anyhow_ext::{Context, Result};
use dashmap::DashMap;
use fs_err as fs;
use join_str::jstr;
use parking_lot::RwLock;
use path_slash::PathExt;
use rayon::prelude::*;
use roead::yaz0::{compress, decompress};
use rstb::ResourceSizeTable;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use uk_content::{constants::Language, platform_prefixes};
use uk_mod::{
    unpack::{ModReader, ModUnpacker},
    Manifest,
};

use crate::{
    mods,
    settings::{DeployMethod, Platform, Settings},
    util,
};

#[inline(always)]
fn is_symlink(link: &Path) -> bool {
    #[cfg(windows)]
    {
        junction::exists(link).unwrap_or(false) || link.is_symlink()
    }
    #[cfg(unix)]
    {
        link.is_symlink()
    }
}

#[inline(always)]
fn is_symlink_to(link: &Path, dest: &Path) -> bool {
    #[cfg(windows)]
    {
        (junction::exists(link).unwrap_or(false)
            && junction::get_target(link).expect("Path does not exist") == dest)
            || (link.is_symlink() && link.read_link().expect("Path does not exist") == dest)
    }
    #[cfg(unix)]
    {
        link.is_symlink() && link.read_link().expect("Path does not exist") == dest
    }
}

#[inline(always)]
fn create_symlink(link: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    junction::create(target, link).or_else(|_| std::os::windows::fs::symlink_dir(target, link))?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).with_context(|| {
        format!(
            "Failed to symlink {} to {}",
            target.display(),
            link.display()
        )
    })?;
    Ok(())
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PendingLog {
    files:  Manifest,
    delete: Manifest,
}

#[derive(Debug)]
pub struct Manager {
    settings: Weak<RwLock<Settings>>,
    mod_manager: Weak<RwLock<mods::Manager>>,
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
        mod_manager: &Arc<RwLock<mods::Manager>>,
    ) -> Result<Self> {
        log::info!("Initializing deployment manager");
        let pending = match fs::read_to_string(Self::log_path(&settings.read()))
            .map_err(anyhow_ext::Error::from)
            .and_then(|text| Ok(serde_yaml::from_str::<PendingLog>(&text)?))
        {
            Ok(log) => {
                if !log.files.is_empty() || !log.delete.is_empty() {
                    log::info!("Pending deployment data found");
                    log::debug!("{:#?}", &log);
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

    #[inline]
    pub fn pending_len(&self) -> usize {
        let dels = self.pending_delete.read();
        let files = self.pending_files.read();
        dels.content_files.len()
            + dels.aoc_files.len()
            + files.content_files.len()
            + files.aoc_files.len()
    }

    pub fn reset_pending(&self) -> Result<()> {
        self.pending_delete.write().clear();
        self.pending_files.write().clear();
        let settings = self
            .settings
            .upgrade()
            .expect("YIKES the settings manager is gone");
        let settings = settings.read();
        let source = settings.merged_dir();
        let config = settings
            .platform_config()
            .and_then(|c| c.deploy_config.as_ref())
            .context("No deployment config for current platform")?;
        let dest = &config.output;
        let (content, aoc) = platform_prefixes(settings.current_mode.into());

        let collect_files = |root: &str| -> BTreeSet<String> {
            let source = source.join(root);
            let dest = dest.join(root);
            jwalk::WalkDir::new(&source)
                .into_iter()
                .filter_map(|file| {
                    file.ok().and_then(|file| {
                        file.metadata().ok().and_then(|meta| {
                            let path = file.path();
                            let rel = path.strip_prefix(&source).unwrap();
                            let dest = dest.join(rel);
                            if !dest.exists()
                                || dest.metadata().ok()?.modified().ok()? < meta.modified().ok()?
                            {
                                Some(rel.to_slash_lossy().into())
                            } else {
                                None
                            }
                        })
                    })
                })
                .collect()
        };

        *self.pending_files.write() = Manifest {
            content_files: collect_files(content),
            aoc_files:     collect_files(aoc),
        };

        let collect_deletes = |root: &str| -> BTreeSet<String> {
            let source = source.join(root);
            let dest = dest.join(root);
            jwalk::WalkDir::new(&source)
                .into_iter()
                .filter_map(|file| {
                    file.ok().and_then(|file| {
                        let path = file.path();
                        let rel = path.strip_prefix(&source).unwrap();
                        let dest = dest.join(rel);
                        (dest.exists() && !path.exists()).then_some(rel.to_slash_lossy().into())
                    })
                })
                .collect()
        };

        *self.pending_delete.write() = Manifest {
            content_files: collect_deletes(content),
            aoc_files:     collect_deletes(aoc),
        };

        Ok(())
    }

    fn save(&self) -> Result<()> {
        fs::write(
            Self::log_path(&self.settings.upgrade().unwrap().read()),
            serde_yaml::to_string(&PendingLog {
                delete: self.pending_delete.read().clone(),
                files:  self.pending_files.read().clone(),
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
        let mut lang = Language::USen;
        let mut profile = String::from("");
        let config = settings
            .platform_config()
            .and_then(|c| {
                lang = c.language;
                profile = c.profile.clone();
                c.deploy_config.as_ref()
            })
            .context("No deployment config for current platform")?;
        log::debug!("Deployment config:\n{:#?}", &config);

        // Determine src and dest folders
        let (content, aoc) = uk_content::platform_prefixes(settings.current_mode.into());
        let src_content  = settings.merged_dir().join(content);
        let src_aoc = settings.merged_dir().join(aoc);
        let (dest_content, dest_aoc) = config.final_output_paths(settings.current_mode.into());
        // Remove old behavior
        if is_symlink(&config.output) {
            log::info!("Removing old symlink deployment behavior");
            util::remove_symlink(&config.output)
                .context("Failed to remove old deployment behavior symlink")?;
        }

        if config.method == DeployMethod::Symlink {
            log::info!("Deploy method is symlink, checking for symlink");

            for (src, dest, type_) in [
                (src_content, dest_content.clone(), "content"),
                (src_aoc, dest_aoc, "aoc")
            ] {
                let actual_dest = match (type_, settings.current_mode) {
                    ("aoc", Platform::WiiU) => dest.parent().unwrap(),
                    _ => dest.as_ref(),
                };
                log::info!("Generating {} links", type_);
                let parent = actual_dest.parent().context("Dest has no parent?")?;
                if src.exists() && !parent.exists() {
                    fs::create_dir_all(parent)
                        .context("Failed to create parents for dest folder")?;
                }
                if actual_dest.exists() && !is_symlink(actual_dest) {
                    log::warn!("Removing old stuff from {} deploy folder", type_);
                    util::remove_dir_all(actual_dest)
                        .context("Failed to remove old deployment folder")?;
                }
                if src.exists() && !actual_dest.exists() {
                    log::info!("Creating new symlink for {} folder", type_);
                    create_symlink(actual_dest, &src)
                        .context("Failed to deploy symlink")?;
                } else if !src.exists() && actual_dest.exists() {
                    log::info!("No {} files, removing link", type_);
                    util::remove_symlink(actual_dest)
                        .context("Failed to remove symlink to non-existent folder")?;
                } else if src.exists() && actual_dest.exists() &&
                    !is_symlink_to(actual_dest, &src) {
                    log::info!("Refreshing {} link to correct profile", type_);
                    util::remove_symlink(actual_dest)
                        .context("Failed to remove symlink to incorrect profile")?;
                    create_symlink(actual_dest, &src)
                        .context("Failed to create symlink to correct profile")?;
                } else {
                    log::info!("Symlink exists, no deployment needed")
                }
            }
        } else {
            let deletes = self.pending_delete.read();
            log::debug!("Deployed files to delete:\n{:#?}", &deletes);
            let syncs = self.pending_files.read();
            log::debug!("Files to deploy\n{:#?}", &syncs);
            log::info!("Deploying by {}", match config.method {
                DeployMethod::Copy => "copy",
                DeployMethod::HardLink => "hard links",
                DeployMethod::Symlink => unsafe { std::hint::unreachable_unchecked() },
            });
            log::info!("Deploy layout: {}", config.layout.name());

            let filter_xbootup = |file: &&String| -> bool {
                !file.starts_with("Pack/Bootup_") || **file == lang.bootup_path()
            };

            for (source, dest,  dels, syncs) in [
                (src_content, dest_content.clone(), &deletes.content_files, &syncs.content_files),
                (src_aoc, dest_aoc, &deletes.aoc_files, &syncs.aoc_files),
            ] {
                dels.par_iter()
                    .filter(filter_xbootup)
                    .try_for_each(|f| -> Result<()> {
                        let file = dest.join(f.as_str());
                        if file.exists() {
                            fs::remove_file(file)?;
                        }
                        Ok(())
                    })?;

                syncs.par_iter().filter(filter_xbootup).try_for_each(
                    |f: &String| -> Result<()> {
                        let from = source.join(f.as_str());
                        let out = dest.join(f.as_str());
                        if out.exists() {
                            fs::remove_file(&out)?;
                        }
                        if from.exists() {
                            out.parent().map(fs::create_dir_all).transpose()?;
                            match config.method {
                                DeployMethod::Copy => fs::copy(from, &out).map(|_| ()),
                                DeployMethod::HardLink => fs::hard_link(from, &out),
                                DeployMethod::Symlink => unreachable!(),
                            }
                            .with_context(|| format!("Failed to deploy {} to {}", f, out.display()))
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
                            Ok(())
                        } else {
                            log::warn!(
                                "Source file {} missing, we're assuming it was a deletion lost \
                                 track of",
                                from.display()
                            );
                            Ok(())
                        }
                    },
                )?;
            }
            log::info!("Deployment complete");
        }
        let rules_path = dest_content.parent().unwrap().join("rules.txt");
        if settings.current_mode == Platform::WiiU
            && settings
                .platform_config()
                .and_then(|c| c.deploy_config.as_ref().map(|c| c.cemu_rules))
                .unwrap_or(false)
            && !rules_path.exists()
        {
            fs::write(rules_path, include_str!("../../../assets/rules.txt"))?;
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
            "Orphans to delete:\n{:#?}\n{:#?}",
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
                if std::fs::read_dir(parent)
                    .map(|mut f| f.next().is_none())
                    .unwrap_or(false)
                {
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
        updates: DashMap<String, Option<u32>>,
    ) -> Result<()> {
        static RSTB_PATH: &str = "System/Resource/ResourceSizeTable.product.srsizetable";
        log::debug!("RSTB updates:\n{:#?}", &updates);
        let content = uk_content::platform_content(platform.into());
        let table_path = merged.join(content).join(RSTB_PATH);
        let mut table = if table_path.exists() {
            log::debug!("Updating existing merged RSTB");
            ResourceSizeTable::from_binary(
                decompress(fs::read(&table_path).context("Failed to open merged RSTB")?)
                    .context("Failed to decompress merged RSTB")?,
            )
            .context("Failed to parse merged RSTB")?
        } else {
            log::debug!("Creating new RSTB");
            ResourceSizeTable::new_from_stock(platform.into())
        };
        for (canon, size) in updates {
            match size {
                Some(size) => {
                    if table.get(canon.as_str()).map(|s| s < size).unwrap_or(true) {
                        table.set(canon.as_str(), size);
                    }
                }
                None => {
                    table.remove(canon.as_str());
                }
            }
        }
        log::info!("Updated RSTB");
        fs::create_dir_all(table_path.parent().unwrap())?;
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
                .read()
                .mods_by_manifest(&manifest)
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .inspect(|m| total_manifest.extend(&m.manifest))
                        .with_context(|| jstr!("Failed to open mod: {&m.meta.name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.handle_orphans(
                total_manifest,
                &mut manifest,
                &out_dir,
                settings.current_mode,
            )?;
            log::debug!("Change manifest: {:#?}", &manifest);
            self.pending_files.write().extend(&manifest);
            ModUnpacker::new(
                dump,
                endian,
                settings.platform_config().unwrap().language,
                mods,
                out_dir.clone(),
            )
            .with_manifest(manifest)
        } else {
            log::info!("Manifest not provided, remerging all mods");
            let mut total_manifest = Manifest::default();
            let mods = mod_manager
                .read()
                .mods()
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .inspect(|m| total_manifest.extend(&m.manifest))
                        .with_context(|| jstr!("Failed to open mod: {&m.meta.name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            util::remove_dir_all(&out_dir).context("Failed to clear merged folder")?;
            self.pending_files.write().extend(&total_manifest);
            ModUnpacker::new(
                dump,
                endian,
                settings.platform_config().unwrap().language,
                mods,
                out_dir.clone(),
            )
        };
        log::info!("Applying changes");
        let rstb_updates = unpacker.unpack()?;
        self.apply_rstb(&out_dir, settings.current_mode, rstb_updates)?;
        self.save()?;
        log::info!("All changed applied successfully");
        Ok(())
    }
}
