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
        log::info!("初始化部署管理器");
        let pending = match fs::read_to_string(Self::log_path(&settings.read()))
            .map_err(anyhow_ext::Error::from)
            .and_then(|text| Ok(serde_yaml::from_str::<PendingLog>(&text)?))
        {
            Ok(log) => {
                if !log.files.is_empty() || !log.delete.is_empty() {
                    log::info!("发现待处理的部署数据");
                    log::debug!("{:#?}", &log);
                } else {
                    log::info!("没有待处理的部署文件");
                }
                log
            }
            Err(e) => {
                log::warn!("无法加载待处理的部署数据:\n{}", &e);
                log::info!("没有待处理的部署文件");
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
            .expect("哎呀，设置管理器不见了");
        let settings = settings.read();
        let source = settings.merged_dir();
        let config = settings
            .platform_config()
            .and_then(|c| c.deploy_config.as_ref())
            .context("当前平台没有部署配置")?;
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
            .expect("哎呀，设置管理器不见了");
        let settings = settings.read();
        let mut lang = Language::USen;
        let config = settings
            .platform_config()
            .and_then(|c| {
                lang = c.language;
                c.deploy_config.as_ref()
            })
            .context("当前平台没有部署配置")?;
        log::debug!("部署配置:\n{:#?}", &config);
        if config.method == DeployMethod::Symlink {
            log::info!("部署方法是 symlink, 正在检查 symlink");
            if !is_symlink(&config.output) {
                if config.output.exists() {
                    log::warn!("从部署文件夹中移除旧内容");
                    util::remove_dir_all(&config.output)
                        .context("无法删除旧的部署文件夹")?;
                }
                log::info!("创建新的符号链接（symlink）");
                create_symlink(&config.output, &settings.merged_dir())
                    .context("创建符号链接部署文件夹失败")?;
            } else {
                log::info!("符号链接已存在，无需部署")
            }
        } else {
            if is_symlink(&config.output) {
                anyhow_ext::bail!(
                    "部署文件夹当前为符号链接或连接点，\
                    但当前的部署方法不是创建符号链接。\
                    请手动移除位于 {} 的现有链接，以避免意外结果。.",
                    config.output.display()
                );
            }
            let (content, aoc) = uk_content::platform_prefixes(settings.current_mode.into());
            let deletes = self.pending_delete.read();
            log::debug!("要删除的部署文件:\n{:#?}", &deletes);
            let syncs = self.pending_files.read();
            log::debug!("要部署的文件\n{:#?}", &syncs);
            log::info!("部署方式 {}", match config.method {
                DeployMethod::Copy => "copy",
                DeployMethod::HardLink => "hard links",
                DeployMethod::Symlink => unsafe { std::hint::unreachable_unchecked() },
            });

            let filter_xbootup = |file: &&String| -> bool {
                !file.starts_with("Pack/Bootup_") || **file == lang.bootup_path()
            };

            for (dir, dels, syncs) in [
                (content, &deletes.content_files, &syncs.content_files),
                (aoc, &deletes.aoc_files, &syncs.aoc_files),
            ] {
                let dest = config.output.join(dir);
                let source = settings.merged_dir().join(dir);
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
                            .with_context(|| format!("部署失败 {} 或 {}", f, out.display()))
                            .map_err(|e| {
                                if e.root_cause().to_string().contains("操作系统错误 17") {
                                    e.context(
                                        "硬链接(Hard linking)失败，因为输出文件夹与存储文件夹位于不同的磁盘或分区上。.",
                                    )
                                } else {
                                    e
                                }
                            })?;
                            Ok(())
                        } else {
                            log::warn!(
                                "源文件 {} 丢失，我们假设它是一个丢失跟踪的删除操作。",
                                from.display()
                            );
                            Ok(())
                        }
                    },
                )?;
            }
            log::info!("部署完成");
        }
        let rules_path = config.output.join("rules.txt");
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
            log::debug!("没有孤立文件");
            return Ok(());
        }
        log::debug!(
            "要删除的孤立文件:\n{:#?}\n{:#?}",
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
                        .with_context(|| jstr!("删除孤立文件失败 {f.as_str()}"))?;
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
        log::info!("已删除孤立文件");
        Ok(())
    }

    fn apply_rstb(
        &self,
        merged: &Path,
        platform: Platform,
        updates: DashMap<String, Option<u32>>,
    ) -> Result<()> {
        static RSTB_PATH: &str = "System/Resource/ResourceSizeTable.product.srsizetable";
        log::debug!("RSTB 更新:\n{:#?}", &updates);
        let content = uk_content::platform_content(platform.into());
        let table_path = merged.join(content).join(RSTB_PATH);
        let mut table = if table_path.exists() {
            log::debug!("更新现有的合并 RSTB");
            ResourceSizeTable::from_binary(
                decompress(fs::read(&table_path).context("无法打开合并的 RSTB")?)
                    .context("无法解压合并的 RSTB")?,
            )
            .context("无法解析合并的 RSTB")?
        } else {
            log::debug!("创建新的 RSTB");
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
        log::info!("更新了 RSTB");
        fs::create_dir_all(table_path.parent().unwrap())?;
        fs::write(table_path, compress(table.to_binary(platform.into())))
            .context("无法写入合并的 RSTB")?;
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
            .expect("哎呀，模组管理系统不见了");
        let settings = self
            .settings
            .upgrade()
            .expect("哎呀，设置管理器不见了");
        let settings = settings.try_read().unwrap();
        let dump = settings
            .dump()
            .context("当前平台没有可用的转储")?;
        let endian = settings.current_mode.into();
        let out_dir = settings.merged_dir();
        let unpacker = if let Some(mut manifest) = manifest {
            log::info!("提供了清单，正在应用有限的更改");
            let mut total_manifest = Manifest::default();
            let mods = mod_manager
                .read()
                .mods_by_manifest(&manifest)
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .inspect(|m| total_manifest.extend(&m.manifest))
                        .with_context(|| jstr!("无法打开 Mod: {&m.meta.name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.handle_orphans(
                total_manifest,
                &mut manifest,
                &out_dir,
                settings.current_mode,
            )?;
            log::debug!("更改清单: {:#?}", &manifest);
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
            log::info!("未提供清单，重新合并所有的 Mod");
            let mut total_manifest = Manifest::default();
            let mods = mod_manager
                .read()
                .mods()
                .map(|m| {
                    ModReader::open(&m.path, m.enabled_options.clone())
                        .inspect(|m| total_manifest.extend(&m.manifest))
                        .with_context(|| jstr!("打开 Mod 失败: {&m.meta.name}"))
                })
                .collect::<Result<Vec<_>>>()?;
            util::remove_dir_all(&out_dir).context("F清除合并文件夹失败r")?;
            self.pending_files.write().extend(&total_manifest);
            ModUnpacker::new(
                dump,
                endian,
                settings.platform_config().unwrap().language,
                mods,
                out_dir.clone(),
            )
        };
        log::info!("应用更改中");
        let rstb_updates = unpacker.unpack()?;
        self.apply_rstb(&out_dir, settings.current_mode, rstb_updates)?;
        self.save()?;
        log::info!("所有更改成功应用");
        Ok(())
    }
}
