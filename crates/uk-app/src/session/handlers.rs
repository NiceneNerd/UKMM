//! Command handlers: the worker's actual work, one function per command.
//!
//! Every handler runs on the session worker thread. Handlers never block on
//! user input — questions ("which options?", "which metadata?") are answered
//! with [`Event::Inspection`] and the frontend dispatches a follow-up
//! command.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow_ext::{Context, Result, bail};
use uk_content::prelude::Endian;
use uk_manager::mods::Mod;
use uk_mod::{Manifest, Meta, ModOption, ModPlatform};

use super::State;
use crate::{
    apply::apply_changes,
    open::{Opened, open_mod},
    order::move_ids,
    package::{dev_update_mods, extract_mods},
    profile::ProfileName,
    session::{Command, Event, Inspection, InstallCandidate, MasterRef, ModId, OptionsPrompt},
};

/// What the frontend has to do with an opened mod. `MissingMasters` is not
/// part of this: it is reported separately and does not block the install.
enum Classified {
    Ready(InstallCandidate),
    NeedsOptions(OptionsPrompt),
    WrongPlatform { mod_is: String, current: String },
}

impl State {
    /// Runs one command. Errors are reported to the frontend as
    /// `JobDone(Err(..))`; panics are caught by the worker.
    pub(super) fn handle(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::Open { path, meta } => self.inspect_path(&path, meta, None),
            Command::Install(candidate) => self.install(candidate),
            Command::InstallBatch { paths, replace } => self.install_batch(paths, replace),
            Command::SetEnabled { ids, enabled } => self.set_enabled(ids, enabled),
            Command::SetOptions { id, picks } => self.set_options(id, picks),
            Command::MoveTo { ids, index } => self.move_to(ids, index),
            Command::Apply => self.apply(),
            Command::CancelPending => self.cancel_pending(),
            Command::Deploy => self.deploy(),
            Command::Remerge => self.remerge(),
            Command::ResetPending => self.reset_pending(),
            Command::Uninstall(ids) => self.uninstall(ids),
            Command::AddToProfile { ids, profile } => self.add_to_profile(ids, &profile),
            Command::SwitchProfile(profile) => self.switch_profile(profile),
            Command::CreateProfile(profile) => self.create_profile(profile),
            Command::DuplicateProfile(profile) => self.duplicate_profile(profile),
            Command::RenameProfile { from, to } => self.rename_profile(from, to),
            Command::DeleteProfile(profile) => self.delete_profile(profile),
            Command::UpdateFromPath { id, path } => self.inspect_path(&path, None, Some(id)),
            Command::DevUpdate { ids, folders } => self.dev_update(ids, folders),
            Command::Extract { ids, dest } => self.extract(ids, dest),
        }
    }

    /// Opens a path and classifies it. The verdict is data, not control flow:
    /// the frontend gets an [`Event::Inspection`] and answers with a command.
    fn inspect_path(
        &mut self,
        path: &Path,
        meta: Option<Meta>,
        replace: Option<ModId>,
    ) -> Result<()> {
        match open_mod(&self.core, path, meta)? {
            Opened::Mod(mod_) => {
                let inspection = match self.classify(mod_, replace) {
                    Classified::Ready(candidate) => Inspection::Ready(candidate),
                    Classified::NeedsOptions(prompt) => Inspection::NeedsOptions(prompt),
                    Classified::WrongPlatform { mod_is, current } => {
                        Inspection::WrongPlatform { mod_is, current }
                    }
                };
                self.emit(Event::Inspection(inspection));
            }
            Opened::NeedsMeta(path) => {
                let suggested_name = path
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.emit(Event::Inspection(Inspection::NeedsMeta {
                    path,
                    suggested_name,
                }));
            }
        }
        Ok(())
    }

    /// Classifies an opened mod: missing masters first (they do not block the
    /// install), then the platform, then the options.
    fn classify(&mut self, mod_: Mod, replace: Option<ModId>) -> Classified {
        let missing: Vec<MasterRef> = mod_
            .meta
            .masters
            .iter()
            .filter(|(hash, _)| !self.mods.iter().any(|m| m.hash() == **hash))
            .map(|(_, (name, version))| {
                MasterRef {
                    name:    name.to_string(),
                    version: version.to_string(),
                }
            })
            .collect();
        if !missing.is_empty() {
            self.emit(Event::Inspection(Inspection::MissingMasters(missing)));
        }
        let platform = self.core.settings().current_mode;
        if !matches!(mod_.meta.platform, ModPlatform::Universal)
            && mod_.meta.platform != ModPlatform::Specific(platform.into())
        {
            let mod_is = match mod_.meta.platform {
                ModPlatform::Specific(Endian::Little) => "Switch",
                ModPlatform::Specific(Endian::Big) => "Wii U",
                ModPlatform::Universal => "any console",
            };
            return Classified::WrongPlatform {
                mod_is:  mod_is.to_owned(),
                current: platform.to_string(),
            };
        }
        if mod_.meta.options.is_empty() {
            Classified::Ready(InstallCandidate::ready(mod_, replace))
        } else {
            // An update starts from the picks the replaced mod had.
            let picks = replace
                .and_then(|old| self.mods.iter().find(|m| m.hash() == old.0))
                .map(|old| old.enabled_options.clone())
                .unwrap_or_default();
            Classified::NeedsOptions(OptionsPrompt::new(mod_, replace, picks))
        }
    }

    /// Installs a staged mod, either as a new mod or as a replacement for an
    /// installed one. The profile is saved right away, so the change survives
    /// a restart even if the user never applies.
    fn install(&mut self, candidate: InstallCandidate) -> Result<()> {
        let InstallCandidate {
            mod_,
            picks,
            replace,
        } = candidate;
        let core = Arc::clone(&self.core);
        let (id, dirty, updated) = match replace {
            Some(old) => {
                let old_mod = self
                    .mods
                    .iter()
                    .find(|m| m.hash() == old.0)
                    .cloned()
                    .with_context(|| format!("Mod {} is not installed", old.0))?;
                let mut dirty = Manifest::default();
                // The files the old mod had, before it is replaced.
                if let Ok(manifest) = old_mod.manifest() {
                    dirty.extend(&manifest);
                }
                // The staged file takes the place of the installed one.
                if mod_.path != old_mod.path {
                    fs_err::copy(&mod_.path, &old_mod.path)
                        .with_context(|| format!("Failed to replace {}", old_mod.path.display()))?;
                }
                let new_mod = core.mod_manager().replace(mod_, old_mod.hash())?;
                if !picks.is_empty() {
                    core.mod_manager()
                        .set_enabled_options(new_mod.hash(), picks)?;
                }
                if let Ok(manifest) = new_mod.manifest() {
                    dirty.extend(&manifest);
                }
                (ModId(new_mod.hash()), dirty, true)
            }
            None => {
                let manager = core.mod_manager();
                let installed = manager.add(&mod_.path, None)?;
                let id = ModId(installed.hash());
                if !picks.is_empty() {
                    manager.set_enabled_options(id.0, picks)?;
                }
                (id, Manifest::default(), false)
            }
        };
        core.mod_manager().save()?;
        self.refresh_mods();
        if updated {
            self.dirty_current_mut().extend(&dirty);
        } else if let Some(mod_) = self.mods.iter().find(|m| m.hash() == id.0) {
            if let Ok(manifest) = mod_.manifest() {
                self.dirty_current_mut().extend(&manifest);
            }
        }
        self.emit_pending();
        if updated {
            self.emit(Event::Updated { id });
        } else {
            self.emit(Event::Installed { id });
        }
        Ok(())
    }

    /// Installs a list of paths, one after the other. Per-file progress is
    /// reported; all failures are collected and reported together.
    fn install_batch(&mut self, paths: Vec<PathBuf>, replace: Option<ModId>) -> Result<()> {
        let total = paths.len();
        self.installing = total;
        let mut errors: Vec<String> = vec![];
        for (index, path) in paths.iter().enumerate() {
            if let Some(job) = self.busy {
                let message = format!("Installing {} ({}/{})", path.display(), index + 1, total);
                self.progress = Some(message.clone());
                self.emit(Event::Progress { job, message });
                self.publish();
            }
            if let Err(e) = self.install_path(path, replace) {
                errors.push(format!("{}: {e:?}", path.display()));
            }
            self.installing = total - index - 1;
            self.publish();
        }
        self.installing = 0;
        self.progress = None;
        if errors.is_empty() {
            Ok(())
        } else {
            bail!(
                "One or more mods could not be installed. Details below:\n{}",
                errors.join("\n")
            )
        }
    }

    /// Opens and installs one path, without asking the user anything.
    fn install_path(&mut self, path: &Path, replace: Option<ModId>) -> Result<()> {
        match open_mod(&self.core, path, None)? {
            Opened::Mod(mod_) => {
                match self.classify(mod_, replace) {
                    Classified::Ready(candidate) => self.install(candidate),
                    Classified::NeedsOptions(prompt) => {
                        let defaults = prompt.defaults().to_vec();
                        let candidate = prompt.resolve(defaults).with_context(|| {
                            format!(
                                "{} has required options — install it on its own to choose them",
                                path.display()
                            )
                        })?;
                        self.install(candidate)
                    }
                    Classified::WrongPlatform { mod_is, current } => {
                        bail!(
                            "{} is for {mod_is}, but the current mode is {current}",
                            path.display()
                        )
                    }
                }
            }
            Opened::NeedsMeta(path) => {
                bail!(
                    "{} has no mod metadata — install it on its own to provide it",
                    path.display()
                )
            }
        }
    }

    /// Enables or disables mods. Saved immediately, so the enabled state is
    /// never lost.
    fn set_enabled(&mut self, ids: Vec<ModId>, enabled: bool) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let core = Arc::clone(&self.core);
        let mut dirty = Manifest::default();
        {
            let manager = core.mod_manager();
            for id in &ids {
                let manifest = manager.set_enabled(id.0, enabled, None)?;
                dirty.extend(&manifest);
            }
            manager.save()?;
        }
        self.dirty_current_mut().extend(&dirty);
        self.refresh_mods();
        self.emit_pending();
        Ok(())
    }

    /// Sets the enabled options of a mod. Saved immediately.
    fn set_options(&mut self, id: ModId, picks: Vec<ModOption>) -> Result<()> {
        let core = Arc::clone(&self.core);
        let manifest = {
            let manager = core.mod_manager();
            let manifest = manager.set_enabled_options(id.0, picks)?;
            manager.save()?;
            manifest
        };
        self.dirty_current_mut().extend(&manifest);
        self.refresh_mods();
        self.emit_pending();
        Ok(())
    }

    /// Moves mods to a load-order index. `index` is always a load-order
    /// index, never a row index.
    fn move_to(&mut self, ids: Vec<ModId>, index: usize) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut order: Vec<ModId> = self.mods.iter().map(|m| ModId(m.hash())).collect();
        move_ids(&mut order, &ids, index, |id| *id);
        let hashes: Vec<usize> = order.iter().map(|id| id.0).collect();
        let mut dirty = Manifest::default();
        for id in &ids {
            if let Some(mod_) = self.mods.iter().find(|m| m.hash() == id.0) {
                let manifest = mod_.manifest_with_options(&mod_.enabled_options)?;
                dirty.extend(&manifest);
            }
        }
        {
            let core = Arc::clone(&self.core);
            let manager = core.mod_manager();
            manager.set_order(hashes);
            manager.save()?;
        }
        self.dirty_current_mut().extend(&dirty);
        self.refresh_mods();
        self.emit_pending();
        Ok(())
    }

    /// Commits the staged changes and re-merges. A failed merge keeps the
    /// staged files, so nothing is lost.
    fn apply(&mut self) -> Result<()> {
        let mods = self.mods.clone();
        let dirty = std::mem::take(self.dirty_current_mut());
        match apply_changes(&self.core, mods, Some(dirty.clone())) {
            Ok(()) => {
                self.refresh_mods();
                self.emit_pending();
                Ok(())
            }
            Err(e) => {
                self.dirty_current_mut().extend(&dirty);
                self.emit_pending();
                Err(e.context("Failed to apply pending changes"))
            }
        }
    }

    /// Discards the pending files of the current profile.
    fn cancel_pending(&mut self) -> Result<()> {
        self.dirty_current_mut().clear();
        self.refresh_mods();
        self.emit_pending();
        Ok(())
    }

    /// Deploys the merged output.
    fn deploy(&mut self) -> Result<()> {
        let core = Arc::clone(&self.core);
        core.deploy_manager().deploy()?;
        Ok(())
    }

    /// Re-merges without changing the mod configuration.
    fn remerge(&mut self) -> Result<()> {
        let core = Arc::clone(&self.core);
        apply_changes(&core, vec![], None).context("Failed to re-merge mods")?;
        self.refresh_mods();
        Ok(())
    }

    /// Resets the deployment pending log.
    fn reset_pending(&mut self) -> Result<()> {
        let core = Arc::clone(&self.core);
        let manager = core.deploy_manager();
        manager.reset_pending()?;
        manager.save()?;
        Ok(())
    }

    /// Removes mods from the current profile. Saved immediately.
    fn uninstall(&mut self, ids: Vec<ModId>) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let core = Arc::clone(&self.core);
        let mut dirty = Manifest::default();
        {
            let manager = core.mod_manager();
            for id in &ids {
                let manifest = manager.del(id.0, None)?;
                dirty.extend(&manifest);
            }
            manager.save()?;
        }
        self.dirty_current_mut().extend(&dirty);
        self.refresh_mods();
        self.emit(Event::Removed { ids });
        self.emit_pending();
        Ok(())
    }

    /// Adds mods to another profile. The target profile is saved right away,
    /// so the addition is not lost on reload.
    fn add_to_profile(&mut self, ids: Vec<ModId>, profile: &ProfileName) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let core = Arc::clone(&self.core);
        // Make sure the profile exists on disk and in the core.
        core.mod_manager().create_profile_if(profile.as_str())?;
        let name = profile.as_str().to_owned();
        let mut dirty = Manifest::default();
        {
            let manager = core.mod_manager();
            for id in &ids {
                let mod_ = self
                    .mods
                    .iter()
                    .find(|m| m.hash() == id.0)
                    .cloned()
                    .with_context(|| format!("Mod {} is not installed", id.0))?;
                let added = manager.add(&mod_.path, Some(&name.as_str().into()))?;
                if let Ok(manifest) = added.manifest() {
                    dirty.extend(&manifest);
                }
            }
        }
        self.save_profile_named(profile.as_str())?;
        let files = {
            let entry = self.dirty.entry(profile.as_str().to_owned()).or_default();
            entry.extend(&dirty);
            entry.content_files.len() + entry.aoc_files.len()
        };
        self.emit(Event::PendingChanged {
            profile: profile.clone(),
            files,
        });
        Ok(())
    }

    /// Switches to another profile. The selection lives in the frontend and
    /// is cleared there.
    fn switch_profile(&mut self, profile: ProfileName) -> Result<()> {
        let core = Arc::clone(&self.core);
        self.ensure_profile_known(&profile)?;
        core.change_profile(profile.as_str())?;
        self.dirty.entry(profile.as_str().to_owned()).or_default();
        self.refresh_mods();
        self.emit(Event::ProfilesChanged);
        self.emit_pending();
        Ok(())
    }

    /// Teaches the core about a profile whose directory exists but was never
    /// loaded — without it, switching to such a profile would leave the core
    /// without a profile to work with.
    fn ensure_profile_known(&self, profile: &ProfileName) -> Result<()> {
        let core = Arc::clone(&self.core);
        let dir = core.settings().profiles_dir().join(profile.as_str());
        if !dir.is_dir() {
            // `change_profile` creates it.
            return Ok(());
        }
        let file = dir.join("profile.yml");
        if !file.exists() {
            fs_err::write(
                &file,
                serde_yaml::to_string(&uk_manager::mods::Profile::default())?,
            )?;
        }
        core.mod_manager()
            .add_profile(profile.as_str().to_owned().into());
        Ok(())
    }

    /// Creates a profile and switches to it.
    fn create_profile(&mut self, profile: ProfileName) -> Result<()> {
        // `change_profile` creates the profile when it does not exist yet.
        self.switch_profile(profile)
    }

    /// Copies a profile, including its mods and its pending files.
    fn duplicate_profile(&mut self, profile: ProfileName) -> Result<()> {
        let core = Arc::clone(&self.core);
        let profiles_dir = core.settings().profiles_dir();
        let source = profiles_dir.join(profile.as_str());
        if !source.is_dir() {
            bail!("Profile {} does not exist", profile.as_str());
        }
        let mut new_name = format!("{}_copy", profile.as_str());
        let mut counter = 2;
        while profiles_dir.join(&new_name).exists() {
            new_name = format!("{}_copy{counter}", profile.as_str());
            counter += 1;
        }
        uk_manager::util::copy_dir(&source, profiles_dir.join(&new_name))?;
        // The core needs a profile file to load the copy from.
        let profile_file = profiles_dir.join(&new_name).join("profile.yml");
        if !profile_file.exists() {
            fs_err::write(
                &profile_file,
                serde_yaml::to_string(&uk_manager::mods::Profile::default())?,
            )?;
        }
        core.mod_manager().add_profile(new_name.clone().into());
        self.dirty.entry(new_name.clone()).or_default();
        self.emit(Event::ProfilesChanged);
        Ok(())
    }

    /// Renames a profile. The pending files move with it.
    fn rename_profile(&mut self, from: ProfileName, to: ProfileName) -> Result<()> {
        let core = Arc::clone(&self.core);
        let profiles_dir = core.settings().profiles_dir();
        let source = profiles_dir.join(from.as_str());
        let target = profiles_dir.join(to.as_str());
        if !source.is_dir() {
            bail!("Profile {} does not exist", from.as_str());
        }
        if target.exists() {
            bail!("Profile {} already exists", to.as_str());
        }
        fs_err::rename(&source, &target)?;
        // The core keeps its profiles in a map — teach it the new name.
        core.mod_manager()
            .add_profile(to.as_str().to_owned().into());
        if self.current_profile_name().as_str() == from.as_str() {
            core.change_profile(to.as_str())?;
        }
        if let Some(dirty) = self.dirty.remove(from.as_str()) {
            self.dirty.insert(to.as_str().to_owned(), dirty);
        } else {
            self.dirty.entry(to.as_str().to_owned()).or_default();
        }
        self.refresh_mods();
        self.emit(Event::ProfilesChanged);
        self.emit_pending();
        Ok(())
    }

    /// Deletes a profile. The active profile cannot be deleted — switch away
    /// from it first.
    fn delete_profile(&mut self, profile: ProfileName) -> Result<()> {
        let core = Arc::clone(&self.core);
        if self.current_profile_name().as_str() == profile.as_str() {
            bail!("Cannot delete the active profile — switch to another one first");
        }
        let profiles_dir = core.settings().profiles_dir();
        let dir = profiles_dir.join(profile.as_str());
        if !dir.is_dir() {
            bail!("Profile {} does not exist", profile.as_str());
        }
        fs_err::remove_dir_all(&dir)?;
        self.dirty.remove(profile.as_str());
        self.emit(Event::ProfilesChanged);
        self.emit_pending();
        Ok(())
    }

    /// Repacks mods in place from loose folders.
    fn dev_update(&mut self, ids: Vec<ModId>, folders: Vec<PathBuf>) -> Result<()> {
        let mods = self.mods_for(&ids)?;
        let core = Arc::clone(&self.core);
        let dirty = dev_update_mods(&core, mods, folders)?;
        core.mod_manager().save()?;
        self.dirty_current_mut().extend(&dirty);
        self.refresh_mods();
        self.emit_pending();
        Ok(())
    }

    /// Unpacks mods into a folder.
    fn extract(&mut self, ids: Vec<ModId>, dest: PathBuf) -> Result<()> {
        let mods = self.mods_for(&ids)?;
        let core = Arc::clone(&self.core);
        extract_mods(&core, mods, dest)?;
        Ok(())
    }

    /// The installed mods with the given ids, in the order of the ids.
    fn mods_for(&self, ids: &[ModId]) -> Result<Vec<Mod>> {
        ids.iter()
            .map(|id| {
                self.mods
                    .iter()
                    .find(|m| m.hash() == id.0)
                    .cloned()
                    .with_context(|| format!("Mod {} is not installed", id.0))
            })
            .collect()
    }

    /// Writes a profile's data to disk. The core only saves the active
    /// profile, so other profiles are written here.
    fn save_profile_named(&self, name: &str) -> Result<()> {
        let core = Arc::clone(&self.core);
        let yaml = {
            let manager = core.mod_manager();
            let profile = manager.get_profile(Some(&name.into()));
            serde_yaml::to_string(&*profile).context("Failed to serialize profile data")?
        };
        let path = core
            .settings()
            .profiles_dir()
            .join(name)
            .join("profile.yml");
        fs_err::write(&path, yaml)
            .with_context(|| format!("Failed to save profile data to {}", path.display()))?;
        Ok(())
    }
}
