use uk_app::{Inspection, view::load_index_of};
use uk_localization::string_ext::LocString;

use super::*;
use crate::gui::options::OptionsModal;

impl App {
    pub(super) fn handle_update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain ALL session events per frame (fixes the one-message-per-frame
        // lag, GUI_REF §22.8).
        let mut got_events = false;
        while let Ok(event) = self.events.try_recv() {
            got_events = true;
            self.handle_event(event, ctx);
        }
        if got_events {
            self.sync_snapshot(ctx);
        }
        // Drain ALL UI messages per frame.
        while let Ok(msg) = self.channel.1.try_recv() {
            self.handle_message(msg, ctx);
        }
        if self.visible_dirty {
            self.refresh_visible();
        }
        self.handle_drops(ctx);
        if got_events || self.snap.busy.is_some() || self.downloading.is_some() {
            ctx.request_repaint();
        }
    }

    /// Re-reads the snapshot after events: reconciles selection with the
    /// current entries and flags the visible list for re-derivation.
    fn sync_snapshot(&mut self, ctx: &egui::Context) {
        let old = std::mem::replace(&mut self.snap, self.session.snapshot());
        if old.profile != self.snap.profile {
            // Switching profiles clears the selection (GUI_REF §22.39).
            self.selected.clear();
            self.drag = None;
            self.hover = None;
        }
        if old.revision != self.snap.revision {
            self.visible_dirty = true;
            // The mod list changed: reload the profile manager and drop the
            // info tab's cached file trees / markdown (parity with the old
            // ResetMods handler).
            self.profiles_state.borrow_mut().reload(&self.core);
            info::ROOTS.write().clear();
            ctx.data_mut(|d| {
                d.remove::<Arc<Mutex<egui_commonmark::CommonMarkCache>>>(Id::new("md_cache"));
            });
        }
        // Drop selected ids that no longer exist (an update changes the hash).
        self.selected
            .retain(|id| self.snap.entries.iter().any(|e| &e.id == id));
    }

    fn handle_event(&mut self, event: Event, ctx: &egui::Context) {
        match event {
            Event::Revision(_) | Event::Progress { .. } => {
                // The snapshot is re-read after the drain loop.
            }
            Event::Inspection(inspection) => {
                match inspection {
                    Inspection::Ready(candidate) => {
                        self.session.dispatch(Command::Install(candidate));
                    }
                    Inspection::NeedsOptions(prompt) => {
                        let picks = prompt.defaults().to_vec();
                        self.options_modal = Some(OptionsModal::Install { prompt, picks });
                    }
                    Inspection::NeedsMeta { path, .. } => {
                        self.meta_input.open(path, self.platform());
                    }
                    Inspection::WrongPlatform { mod_is, current } => {
                        self.error =
                            Some(uk_app::error::AppError::WrongPlatform(mod_is, current).into());
                    }
                    Inspection::MissingMasters(masters) => {
                        // Advisory only: the install proceeds regardless.
                        let names = masters
                            .iter()
                            .map(|m| format!("{} {}", m.name, m.version))
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.error = Some(
                            uk_app::error::AppError::MissingMasters {
                                name:    names,
                                version: String::new(),
                            }
                            .into(),
                        );
                    }
                }
            }
            Event::Updated { from, to } => {
                // An update changed the mod's hash: carry the selection over
                // to the new id.
                if self.selected.contains(&from) {
                    self.selected.retain(|s| *s != from);
                    if !self.selected.contains(&to) {
                        self.selected.push(to);
                    }
                }
                if self.drag == Some(from) {
                    self.drag = Some(to);
                }
                if self.hover == Some(from) {
                    self.hover = Some(to);
                }
            }
            Event::ProfilesChanged => {
                self.profiles_state.borrow_mut().reload(&self.core);
            }
            Event::SettingsChanged => {
                self.temp_settings = self.core.settings().clone();
                settings::CONFIG.write().clear();
                self.package_builder.borrow_mut().reset(self.platform());
                self.toasts.add({
                    let mut toast = Toast::success("Settings_Saved".localize());
                    toast.duration(Some(Duration::new(2, 0)));
                    toast
                });
            }
            Event::PackageMeta(meta) => {
                self.package_builder.borrow_mut().meta = meta;
            }
            Event::Packaged => {
                self.package_builder.borrow_mut().reset(self.platform());
            }
            Event::Changelog(md) => self.changelog = Some(md),
            Event::UpdateAvailable(version) => {
                let message = "Update_Available".localize();
                self.changelog = Some(format!("{}\n\n{}", message, version.description()));
                self.new_version = Some(version);
            }
            Event::Restart => self.restart(ctx),
            Event::JobDone {
                result: Err(error), ..
            }
            | Event::Error(error) => {
                log::error!("{:?}", &error);
                self.error = Some(error.into());
            }
            _ => {}
        }
    }

    fn restart(&self, ctx: &egui::Context) {
        let mut exe = std::env::current_exe().unwrap();
        if exe.extension().and_then(|x| x.to_str()).contains(&"bak") {
            exe.set_extension("");
        }
        let mut command = std::process::Command::new(exe);
        #[cfg(unix)]
        {
            std::os::unix::process::CommandExt::process_group(&mut command, 0);
        }
        command.spawn().unwrap();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn handle_message(&mut self, msg: Message, ctx: &egui::Context) {
        match msg {
            Message::ChangeSort(sort, rev) => {
                self.list_state.sort = sort;
                self.list_state.descending = rev;
                self.visible_dirty = true;
            }
            Message::SelectOnly(id) => {
                if self.selected.contains(&id) {
                    self.selected.retain(|s| *s == id);
                } else {
                    self.selected.clear();
                    self.selected.push(id);
                }
                self.drag = None;
            }
            Message::SelectAlso(id) => {
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
                self.drag = None;
            }
            Message::SelectThrough(id) => {
                if let Some(index) = self.visible.iter().position(|v| *v == id) {
                    if let Some(start) = self
                        .selected
                        .first()
                        .and_then(|s| self.visible.iter().position(|v| v == s))
                    {
                        let range = if start < index {
                            start..=index
                        } else {
                            index..=start
                        };
                        self.selected = self
                            .visible
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| range.contains(i))
                            .map(|(_, id)| *id)
                            .collect();
                    }
                }
                self.drag = None;
            }
            Message::Deselect(id) => {
                self.selected.retain(|s| *s != id);
                self.drag = None;
            }
            Message::StartDrag(id) => {
                if ctx.input(|i| i.pointer.any_released()) {
                    self.drag = None;
                }
                self.drag = Some(id);
                if !self.selected.contains(&id) {
                    if !ctx.input(|i| i.modifiers.ctrl) {
                        self.selected.clear();
                    }
                    self.selected.push(id);
                }
            }
            Message::CloseError => self.error = None,
            Message::CloseConfirm => self.confirm = None,
            Message::ShowAbout => self.show_about = true,
            Message::CloseAbout => self.show_about = false,
            Message::CloseProfiles => self.profiles_state.borrow_mut().show = false,
            Message::Confirm(cmd, prompt) => self.confirm = Some((cmd, prompt)),
            Message::SelectProfileManage(name) => {
                self.profiles_state.borrow_mut().selected = Some(name);
            }
            Message::SetDownloading(name) => {
                log::info!("Downloading {name} from GameBanana…");
                ctx.request_repaint();
                self.downloading = Some(name);
            }
            Message::SetFocus(pane) => self.focused = pane,
            Message::SetTheme(theme) => {
                theme.set_theme(ctx);
                self.theme = theme;
                self.dock_style = visuals::style_dock(&ctx.style());
            }
            Message::SetLanguage(lang) => {
                LOCALIZATION.write().update_language(&lang);
            }
            Message::FilePickerUp => {
                let has_parent = self.picker_state.path.parent().is_some();
                if has_parent {
                    self.picker_state
                        .history
                        .push(self.picker_state.path.clone());
                    self.picker_state
                        .set_path(self.picker_state.path.parent().unwrap().to_path_buf());
                }
            }
            Message::FilePickerBack => {
                if let Some(prev) = self.picker_state.history.pop() {
                    self.picker_state.set_path(prev);
                }
            }
            Message::FilePickerSet(path) => {
                let path = match path {
                    Some(path) => path,
                    None => self.picker_state.path_input.as_str().into(),
                };
                if path.is_dir() {
                    self.picker_state.selected = None;
                    self.picker_state
                        .history
                        .push(self.picker_state.path.clone());
                    self.picker_state.set_path(path);
                }
            }
            Message::OpenMod(path) => {
                let meta = self.meta_input.take();
                self.downloading = None;
                ctx.request_repaint();
                self.session.dispatch(Command::Open { path, meta });
            }
            Message::SelectFile => {
                if let Some(paths) = rfd::FileDialog::new()
                    .set_title("Mod_Select_Title".localize())
                    .add_filter("Any mod (*.zip, *.rar, *.7z, *.bnp, rules.txt)", &[
                        "zip", "rar", "7z", "bnp", "txt",
                    ])
                    .add_filter("UKMM Mod (*.zip)", &["zip"])
                    .add_filter("BCML Mod (*.bnp)", &["bnp"])
                    .add_filter("Legacy Mod (*.zip, *.rar, *.7z, rules.txt)", &[
                        "zip", "rar", "7z", "txt",
                    ])
                    .add_filter("All files (*.*)", &["*"])
                    .pick_files()
                    .filter(|p| !p.is_empty())
                {
                    self.downloading = None;
                    self.session.dispatch(Command::InstallBatch {
                        paths,
                        replace: None,
                    });
                }
            }
            Message::CheckMeta => {
                let source = self.package_builder.borrow().source.clone();
                for file in ["info.json", "rules.txt", "meta.yml"] {
                    let file = source.join(file);
                    if file.exists() {
                        self.session.dispatch(Command::ParseMeta(file));
                        break;
                    }
                }
            }
            Message::GetPackagingOptions => {
                let folder = self.package_builder.borrow().source.join("options");
                if let Ok(reader) = fs::read_dir(folder) {
                    let files = reader
                        .filter_map(|res| {
                            res.ok().and_then(|e| {
                                e.file_type()
                                    .ok()
                                    .and_then(|t| t.is_dir().then(|| e.path()))
                            })
                        })
                        .collect();
                    self.do_update(Message::ShowPackagingOptions(files));
                }
            }
            Message::ShowPackagingOptions(folders) => {
                self.opt_folders = Some(Mutex::new(folders));
            }
            Message::ShowPackagingDependencies => {
                self.show_package_deps = true;
            }
            Message::ClosePackagingOptions => self.opt_folders = None,
            Message::ClosePackagingDependencies => self.show_package_deps = false,
            Message::PackageMod => {
                let builder = self.package_builder.borrow().clone();
                let default_name = sanitise(&builder.meta.name) + ".zip";
                if let Some(dest) = rfd::FileDialog::new()
                    .add_filter("UKMM Mod", &["zip"])
                    .set_title("Package_Save_Title".localize())
                    .set_file_name(default_name)
                    .save_file()
                {
                    self.session.dispatch(Command::Package {
                        source: builder.source,
                        dest,
                        meta: builder.meta,
                    });
                }
            }
            Message::ImportCemu => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Settings_SelectFolder_Cemu".localize())
                    .pick_folder()
                {
                    self.session.dispatch(Command::ImportCemu(path));
                }
            }
            Message::MigrateBcml => {
                self.session.dispatch(Command::MigrateBcml);
            }
            Message::SaveSettings => {
                let mut needs_reset = false;
                if let Some(old_plat) = self.core.settings().platform_config() {
                    if let Some(old_dep) = old_plat.deploy_config.as_ref() {
                        if let Some(new_plat) = &self.temp_settings.platform_config() {
                            if let Some(new_dep) = new_plat.deploy_config.as_ref() {
                                if (old_dep.layout != new_dep.layout
                                    || old_dep.method != new_dep.method
                                    || old_dep.output != new_dep.output)
                                    && self
                                        .core
                                        .settings()
                                        .wipe_output(self.core.settings().current_mode.into())
                                        .is_ok()
                                {
                                    needs_reset = true;
                                }
                            }
                        }
                    }
                }
                match self.temp_settings.save() {
                    Ok(()) => {
                        // The session reloads the core from disk and answers
                        // with SettingsChanged. Saving also drops the staged
                        // pending files (parity with the old ResetMods(None)).
                        self.session.dispatch(Command::Reload);
                        self.session.dispatch(Command::CancelPending);
                        self.selected.clear();
                        if needs_reset {
                            self.session.dispatch(Command::ResetPending);
                        }
                    }
                    Err(e) => self.error = Some(e),
                };
            }
            Message::ResetSettings => {
                self.temp_settings = self.core.settings().clone();
                settings::CONFIG.write().clear();
            }
            Message::NewProfile => self.new_profile = Some("".into()),
            Message::DoUpdate => {
                if let Some(version) = self.new_version.take() {
                    self.changelog = None;
                    self.session.dispatch(Command::InstallUpdate(version));
                }
            }
            Message::Toast(msg) => {
                self.toasts.add({
                    let mut toast = Toast::info(msg);
                    toast.duration(Some(Duration::new(2, 0)));
                    toast
                });
            }
            Message::CloseChangelog => self.changelog = None,
            Message::Error(error) => self.error = Some(error),
        }
    }

    /// The ids a context-menu action targets: the clicked mod, or the whole
    /// selection when the clicked mod is part of it (GUI_REF §22.10).
    pub(super) fn context_target(&self, clicked: ModId) -> Vec<ModId> {
        if self.selected.contains(&clicked) {
            self.selected.clone()
        } else {
            vec![clicked]
        }
    }

    /// The load-order destination for a drag onto `hovered` (never a row
    /// index — GUI_REF §22.1).
    pub(super) fn drag_dest(&self, hovered: ModId) -> usize {
        load_index_of(&self.snap.entries, hovered).unwrap_or(self.snap.entries.len())
    }
}
