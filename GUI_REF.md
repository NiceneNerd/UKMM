# UKMM GUI — Complete Behavioural Reference

This document describes **everything the current UKMM desktop frontend is and does**, as implemented in `src/gui.rs` and `src/gui/*.rs` on top of eframe/egui, plus the parts of `uk-manager`, `uk-mod`, `uk-ui`, and `uk-localization` that the GUI directly exposes.

Its purpose is to be a parity specification for a replacement frontend, and an inventory for extracting an application layer. It therefore separates:

| Tag | Meaning |
|---|---|
| **[UI-ONLY]** | Selection, focus, sort/filter choice, docking, modals, theme, toasts. Can differ per frontend. |
| **[APP]** | User-level operation and domain rule that **must** be reproduced for parity, but should live below the frontend. |
| **[CORE]** | Behaviour already owned by `uk-manager` / `uk-mod` / `uk-reader` that the GUI merely triggers or displays. |
| **[EGUI]** | An egui/eframe implementation technique. A new frontend may replace it, but it explains current behaviour. |
| **[HOST]** | OS integration (dialogs, file manager, registry, update swap, IPC). |
| **[GAP]** | Current behaviour that is accidental, lossy, unsafe, or a known bug. Preserve only if strict bug-for-bug parity is wanted. |

The GUI is **immediate mode**. There is no retained widget tree. Every frame re-reads `App` state and re-emits widgets; interaction results are turned into `Message`s sent through a channel and applied on a later frame.

This snapshot reflects the working tree as inspected: some operations have already been mechanically extracted into `crates/uk-app` (`open_mod`, `apply_changes`, `package_mod`, `dev_update_mod`, `extract_mods`, `parse_meta`, Cemu/BCML import), while the state, staging, dialogs, and orchestration described below still live in the GUI. Where a rule currently lives in the reducer or a view, that is called out as a migration target.

---

## 1. Process and Window Lifecycle

### 1.1 Binary entry and GUI selection [HOST]

`src/main.rs`:

1. Installs `mimalloc` as the global allocator.
2. Installs the rustls crypto provider.
3. Parses CLI arguments with `xflags` (`Ukmm::from_env()`).
4. If parsing **succeeds**, runs the CLI runner. Therefore a recognized subcommand (`install`, `package`, `uninstall`, `remerge`, `deploy`, `mode`) never starts the GUI.
5. If parsing **fails and it is not a help request**, treats the launch as a GUI launch:
   - If the first argument starts with `bcml:`, hands the remainder to the one-click GameBanana path.
   - Otherwise, if the first argument names an existing path, queues it as a mod to install.
   - Starts the one-click IPC listener.
   - Installs a panic hook that prints a backtrace.
   - Runs `gui::main()` under `catch_unwind`; an uncaught panic becomes a native/console error dialog, and the log file location is printed.
6. `--debug` enables `env_logger` debug logging in this fallback path.

### 1.2 Native window [EGUI]/[HOST]

- Title: `U-King Mod Manager`.
- Window icon: `assets/ukmm.png`, decoded at startup into 256×256 RGBA.
- Minimum inner size: 850×500 logical pixels.
- Initial inner size: 1200×800.
- Backend: eframe with `glow`, X11 and Wayland support.
- On non-Windows systems, if `XDG_SESSION_TYPE=wayland`, the DPI scale is forced from `WINIT_X11_SCALE_FACTOR` (default `1.0`) via `Context::set_pixels_per_point`. [EGUI]/[HOST]
- The GUI log file is `config/log.txt`; it is deleted/truncated each launch.

### 1.3 Startup sequence [APP]

On first construction (`App::new`):

1. Load fonts and icon textures into the egui context. [EGUI]
2. `Manager::init()` loads settings, mod manager, and deploy manager. Failure is a panic (`unwrap`) before any window state exists.
3. Read `ui.json` from `Settings::state_file()` (`<config_dir>/ui.json`). Any read or parse failure silently falls back to defaults. [UI-ONLY]
4. Apply the saved theme.
5. Read all mods in current-profile load order into `mods`; build `displayed_mods` by applying the saved sort and direction.
6. Create an unbounded flume channel for `Message`s. Publish its sender to the one-click subsystem.
7. Clone core `Settings` into `temp_settings` (the settings-tab draft).
8. Set the UI localization language from `settings.lang`.
9. Select the first mod in load order, if any.
10. Create the package builder with a blank `Meta` for the current platform.
11. Load profile-manager data by reading each profile's `profile.yml`.
12. If `last_version` is `None`, **or** the current platform has no configuration, show the localized first-run `Intro_Message` in the changelog window. Otherwise, synchronously query GitHub releases before the UI opens.
13. Initialize the per-profile `dirty` manifest map with an empty manifest for every profile found on disk.
14. Initialize toasts at bottom-right and compute the dock style from the active egui style.

Notable properties:

- There is no splash screen or loading UI; release checking and all profile/mod scanning happen before the first frame. [GAP]/[EGUI]
- The first-run condition is `last_version` missing **or** no platform config present.
- `settings.show_changelog` is stored and editable but is **never consulted**. [GAP]
- `settings.check_updates` (`None`/`Stable`/`Beta`) is consulted by release filtering but has **no UI control**. [GAP]

### 1.4 Shutdown sequence [APP]

On normal window close (`on_exit`):

1. Flush the log to `config/log.txt`.
2. Set `settings.last_version` to the running crate version and save settings.
3. Serialize UI state to `ui.json`: theme, mod-list sort/direction, file-picker path/history/input, and the whole `DockState`.
4. Delete the manager temp directory (`uk_manager::util::clear_temp()`).

A crash or forced kill skips all of this. [GAP]

### 1.5 First-run introduction [UI-ONLY]

The intro is markdown shown in the same centered "What's New" window used for changelogs. English text explains: choose platform, set game dump path, configure deployment, then go to Settings. It is shown when `last_version` is absent or the active platform has no config, and suppresses the update check.

---

## 2. Frame Cycle and Event Model

### 2.1 Per-frame order [EGUI]

Every `eframe::App::update` does, in order:

1. **Consume at most one `Message`** from the channel via `try_recv()`. If none, process OS file drops.
2. Render the top menu bar.
3. Render, in order: error window, confirm window, new-profile window, About window, option-picker window, profile-manager window, changelog window, metadata-input window.
4. Render the dock area in a synthetic background `Ui` covering the available rect, with 8×8 item spacing.
5. Render the busy overlay.
6. Show toasts.

Consequences:

- Dialog windows are rendered before the dock, but exist in egui's middle window layer, so they appear above docked content.
- `busy` and toasts are rendered after everything and are visually topmost.
- **Only one queued message is processed per frame.** A task that enqueues multiple consequences (e.g. `ResetMods` → `RefreshModsDisplay` → `ChangeSort`) takes multiple frames and can build a backlog. This is an implementation artifact, not a requirement. [EGUI]/[GAP]
- UI-originated messages are handled on the *next* frame, so every interaction has at least one frame of latency. [EGUI]

### 2.2 File drag-and-drop [APP]/[HOST]

If no modal is open and the OS reports dropped files:

1. Take the first path; enqueue all remaining paths in `install_queue`.
2. Clear `error_queue`.
3. Start an `open_mod` job for the first file.

The first path is `unwrap()`ed; a drop with no path panics. [GAP] Drops are ignored entirely while any modal/busy state is active.

### 2.3 Worker jobs [APP]

`do_task` runs a closure on a new OS thread:

- Closure type: `FnOnce(Arc<Manager>) -> Result<Message>`, `Send + Sync + UnwindSafe`.
- `busy` is set **before** spawning, disabling the UI.
- Panics are caught and converted into a generic error chain with a captured backtrace.
- `Err` becomes `Message::Error`.
- After the closure, the platform dump cache is cleared unconditionally.
- The resulting message is sent on the channel.

There is exactly **one** `busy` guard. Completions clear it (`Noop`, `ResetMods`, `AddMod`, `RemoveMods`, `ReloadProfiles`, `CleanProfile`, etc.). Some actions start jobs without going through `do_task` (release check, emulator launch), and some messages set `busy` manually (`SetDownloading`). [GAP]

---

## 3. Application State Inventory

`App` fields, and whether they are frontend state or application state in disguise:

| Field | Type | Role | Classification |
|---|---|---|---|
| `core` | `Arc<Manager>` | Facade to settings, mods, deployment | [CORE] |
| `channel` | `(Sender<Message>, Receiver<Message>)` | UI event bus | [EGUI] |
| `mods` | `Vec<Mod>` | **Authoritative working copy of load order, enabled flags, options** until Apply | [APP] |
| `displayed_mods` | `Vec<Mod>` | Sorted clone used for rendering | [UI-ONLY] |
| `mod_list_filter` | `String` | Name substring filter | [UI-ONLY] |
| `selected` | `Vec<Mod>` | Selection by cloned value/hash | [UI-ONLY] |
| `install_queue` | `VecDeque<PathBuf>` | Pending batch installs | [APP] |
| `update_mod` | `Option<Mod>` | “Next opened mod replaces this one” marker | [APP] |
| `error_queue` | `VecDeque<Error>` | Errors stashed during batch install | [APP] |
| `drag_index` | `Option<usize>` | Index of dragged row | [UI-ONLY] |
| `hover_index` | `Option<usize>` | Drop target row | [UI-ONLY] |
| `list_state` | `ModListState` | Sort + direction | [UI-ONLY] |
| `picker_state` | `FilePickerState` | Built-in file browser state | [UI-ONLY] |
| `profiles_state` | `RefCell<ProfileManagerState>` | Profile-manager data | [APP] |
| `meta_input` | `MetaInputModal` | Metadata prompt state | [UI-ONLY] |
| `closed_tabs` | `HashMap<Tabs, NodeIndex>` | Dock tab re-open memory | [UI-ONLY] |
| `tree` | `Rc<RefCell<DockState<Tabs>>>` | Dock layout | [UI-ONLY] |
| `focused` | `FocusedPane` | Which list owns keyboard arrows | [UI-ONLY] |
| `error` | `Option<Error>` | Currently displayed error | [UI-ONLY] |
| `new_profile` | `Option<String>` | New-profile dialog input | [UI-ONLY] |
| `confirm` | `Option<(Message, String)>` | Confirm dialog action + prompt | [UI-ONLY] |
| `busy` | `Cell<bool>` | One-job guard / busy overlay | [APP] |
| `show_about` | `bool` | About modal | [UI-ONLY] |
| `package_builder` | `RefCell<ModPackerBuilder>` | Packaging form | [APP]/[UI-ONLY] |
| `show_package_deps` | `bool` | Packaging dependency modal | [UI-ONLY] |
| `opt_folders` | `Option<Mutex<FxHashSet<PathBuf>>>` | Available option folders | [APP] |
| `dirty` | `RwLock<HashMap<String, Manifest>>` | **Pending merge manifests per profile** | [APP] |
| `options_mod` | `Option<(Mod, bool)>` | Option picker: mod + update-vs-install mode | [UI-ONLY] |
| `temp_settings` | `Settings` | Settings draft | [APP] |
| `toasts` | `egui_notify::Toasts` | Transient notifications | [EGUI] |
| `theme` | `visuals::Theme` | Active theme | [UI-ONLY] |
| `dock_style` | `uk_ui::egui_dock::Style` | Cached dock style | [EGUI] |
| `changelog` | `Option<String>` | Markdown in changelog/intro modal | [UI-ONLY] |
| `new_version` | `Option<VersionResponse>` | Available update | [APP] |

**Correctness-critical hidden state:** `mods`, `dirty`, `install_queue`, `update_mod`, `error_queue`, `busy`, `temp_settings`, and the loaded profile set. A second frontend that does not replicate these exactly will corrupt merges. This is the core motivation for `uk-app`.

**Derived state:** `displayed_mods` is only recomputed on sort changes and refreshes, **not** when `mods` changes directly. Every mutation path deliberately sends `RefreshModsDisplay`.

---

## 4. Message Catalogue

All GUI actions are `Message` values. This is the full set and its semantics.

| Message | Trigger | Effect |
|---|---|---|
| `AddMod(Mod)` | Install job finished adding | Extend dirty with manifest; reload `mods`; refresh display; clear busy; start next queued install; if queue empty, surface accumulated errors |
| `AddToProfile(name)` | Context menu “Send to profile” | Add each selected mod to named profile; extend that profile's dirty manifest; success toast |
| `AddProfile` | New-profile dialog OK | `core.change_profile(input)`; reset mods |
| `Apply` | Pending window Apply | Clone current `mods`, take current dirty manifest, run apply job |
| `ChangeProfile(name)` | Profile combo | Switch core profile; reload mods; refresh display; reload profile manager |
| `ChangeSort(sort, descending)` | Mod-list header | Re-sort `displayed_mods`; remember choice |
| `CheckMeta` | Packaging source changed | Look for `info.json`, `rules.txt`, `meta.yml` in source root; parse first found |
| `CleanProfile(name)` | After profile delete | Reload profiles; delete stored mod files no longer referenced by any profile |
| `ClearDrag` | Pointer release without a move | Clear drag index |
| `ClearSelect` | Settings save / settings import | Empty selection |
| `CloseAbout`, `CloseConfirm`, `CloseError`, `CloseChangelog`, `ClosePackagingOptions`, `ClosePackagingDependencies`, `CloseProfiles` | Dialog buttons | Close the named dialog |
| `Confirm(msg, prompt)` | Destructive actions | Open confirm modal whose OK dispatches `msg` |
| `DeleteProfile(name)` | Profiles manager Delete | Job: `remove_dir_all(profiles/<name>)`, then `CleanProfile` |
| `Deploy` | Deploy tab button | `deploy_manager().deploy()`, then `ResetMods` |
| `Deselect(i)` | Ctrl-click selected row | Remove mod at index from selection |
| `DevUpdate` | Context menu | For each selected mod, prompt for a folder and repack from it |
| `DoUpdate` | Changelog Update button | Download and install the offered release, then restart |
| `DuplicateProfile(name)` | Profiles manager Duplicate | Copy profile dir to `<name>_copy`; register profile; reload |
| `Error(Error)` | Any job failure | Log; show modal if no install queue, else stash and continue batch |
| `Extract` | Context menu | Prompt for folder; unpack selected mods into `<folder>/<mod name>` |
| `FilePickerBack` | Install toolbar | Pop history path |
| `FilePickerSet(Option<PathBuf>)` | Path edit / double-click / refresh | Navigate if directory; update entries/history |
| `FilePickerUp` | Install toolbar | Go to parent |
| `GetPackagingOptions` | Package “Manage Options” | Read `source/options/*` directories |
| `HandleMod(Mod)` | Open job succeeded | Apply platform/masters checks; request options or install |
| `HandleSettings` | BCML migration success | Reload settings draft; toast; clear caches; reset selection/mods |
| `ImportCemu` | Settings Wii U import | Prompt for Cemu folder; import; reset settings draft |
| `InstallMod(Mod)` | Options accepted or no options | Job: replace old mod if updating, else add+sanitize options+save |
| `MigrateBcml` | Settings button | Job: import BCML settings and mods |
| `ModUpdate` | Context menu Update | Prompt for mod archive; mark first selected mod as replacement target; open it |
| `MoveSelected(dest)` | Drag release / context move | Reorder selected block in `mods`; dirty their manifests |
| `NewProfile` | Profile add button | Open new-profile modal |
| `Noop` | Completion without state change | Clear busy |
| `OfferUpdate(VersionResponse)` | Release check | Show changelog with “update available” + release notes; store update |
| `OpenMod(path)` | File picker / native picker / metadata OK | Job: `uk_app::open_mod(core, path, meta)` |
| `PackageMod` | Package button | Native save dialog; package builder's source/dest/meta |
| `RefreshModsDisplay` | Internal | Re-apply current sort from `mods` |
| `ReloadProfiles` | Internal | Re-read `profile.yml` files; clear busy |
| `Remerge` | Tools menu | `apply_changes(core, [], None)` — full re-merge |
| `RemoveMods(Vec<Mod>)` | Uninstall job finished | Remove from `mods` and selection; dirty manifests; refresh; clear busy |
| `RenameProfile(old, new)` | Profiles manager | `fs::rename` profile directory; reload |
| `RequestMeta(path)` | Open job says metadata missing | Open metadata modal prefilled from filename |
| `RequestOptions(mod, update)` | Platform/masters passed | Enable defaults first (install mode), open option picker |
| `ResetMods(Option<Manifest>)` | Many completions | Clear current dirty; optionally replace with provided manifest; reload mods; retain selection; refresh; reload profiles; clear markdown/manifest caches |
| `ResetPacker` | Packaging finished | Reset package builder for platform; clear busy |
| `ResetPending` | Tools menu / settings save | Reset deployment pending log and save |
| `ResetSettings` | Cemu import / Settings Reset | Re-clone core settings into draft; clear platform draft cache |
| `Restart` | After self-update | Spawn current exe (dropping `.bak`), close viewport |
| `SaveSettings` | Settings Save | Validate drafts; wipe output if needed; save + core reload; reset UI caches |
| `SelectAlso(i)` | Ctrl-click / Shift+Down | Add row to selection |
| `SelectFile` | File menu / Install toolbar | Native multi-file picker |
| `SelectOnly(i)` | Click | Select only row (or reduce multi-selection to it) |
| `SelectProfileManage(name)` | Profile manager list | Select profile in side panel |
| `SelectThrough(i)` | Shift-click | Select visible range from first selected to row |
| `SetChangelog(md)` | Release check | Show markdown in changelog modal |
| `SetDownloading(name)` | One-click start | Request repaint, set busy, log the download |
| `SetFocus(pane)` | Row/entry click | Set keyboard-arrow owner |
| `SetLanguage(lang)` | Settings combo | Apply localization immediately |
| `SetTheme(theme)` | Settings combo | Apply theme immediately; recompute dock style |
| `ShowAbout` | Help menu | Open About |
| `ShowPackagingDependencies` | Package button | Open dependency chooser |
| `ShowPackagingOptions(folders)` | After folder scan | Open options configuration modal |
| `StartDrag(i)` | Drag gesture | Set drag index; select row unless Ctrl is held |
| `Toast(text)` | Bitcoin copy | Info toast |
| `ToggleMods(Option<Vec<Mod>>, enabled)` | Checkbox/context menu | Set enabled flag in working copy; dirty manifests |
| `UninstallMods(Option<Vec<Mod>>)` | Confirm OK | Job: delete each mod from current profile and save |
| `UpdateOptions(Mod)` | Options modal in update mode | Write options to core immediately; dirty manifest; update working copies |
| `UpdatePackageMeta(Meta)` | Metadata parse finished | Store in package builder; clear busy |

---

## 5. Global Shell and Navigation

### 5.1 Top menu bar [UI-ONLY]

A `TopBottomPanel::top("menu_bar")` with button frames disabled. The entire menu is disabled whenever `modal_open()` is true.

- **File**
  - `Install mod…` — opens the native multi-file picker (`SelectFile`).
  - `Exit` — closes the viewport (does not confirm).
- **Tools**
  - `Refresh merge` — full re-merge (`Remerge`).
  - `Reset pending` — clears the deployment pending log (`ResetPending`).
  - `Open Config folder` — opens `Settings::config_dir()` in the OS file manager.
  - `Open Storage folder` — opens `settings.storage_dir`.
  - `Open Deployment folder` — enabled only when a deploy output is configured; opens it.
- **Window**
  - `Reset` — restores the default dock layout.
  - Separator.
  - A checkable list of all seven tabs. Clicking toggles visibility. Hidden tabs remember a parent dock node; re-opening appends to that parent leaf if it still has tabs, otherwise pushes to the focused leaf.
- **Help**
  - `Help` — opens `https://nicenenerd.github.io/UKMM`.
  - `About` — opens About.

Right-aligned in the bar: current platform name uppercased and bold (`WII U` / `SWITCH`).

No keyboard shortcuts are registered for menu actions.

### 5.2 Dockable tab layout [EGUI]/[UI-ONLY]

Seven tabs: `Info`, `Install`, `Deploy`, `Mods`, `Log`, `Settings`, `Package`.

Default `DockState`:

- Root leaf: **Mods, Package, Settings** (in that tab order).
- Split right at 0.85 from root: a right column containing **Info, Install** on top and **Deploy** below (split 0.6).
- Split the main column below at 0.75: **Log** at the bottom.
- Focused node: main (Mods) area.

Characteristics:

- Tabs can be dragged, split, stacked, resized, and closed.
- Closing a tab records it in `closed_tabs`; the Window menu can restore it.
- The whole dock layout is serialized inside `ui.json` as `DockState<Tabs>`.
- Dock style is derived from the active egui style: 2px top corner radius on tab bodies, focused/weak text colors, 0.5px separators, no dock padding.
- The Mods tab paints a decorative blueprint-like grid when the Sheikah theme is active. Purely decorative. [EGUI]
- All tab bodies are wrapped in `add_enabled_ui(!modal_open())`, so any modal or busy state makes the entire dock non-interactive.

### 5.3 Tab titles [UI-ONLY]

Titles are localized each frame via `Display`: `Tab_Info`, `Tab_Install`, `Tab_Deploy`, `Tab_Mods`, `Tab_Log`, `Tab_Settings`, `Tab_Package`.

---

## 6. Mods Tab

This is the primary workflow surface. It contains, top to bottom: the profile bar, the mod table, and the floating Pending Changes window.

### 6.1 Profile bar [UI-ONLY]/[APP]

A horizontal row:

1. **Profile combo** listing every directory under the current platform's `profiles/`. The selected text is the current profile (or `Default` if no platform config). Choosing another profile sends `ChangeProfile`.
2. **Add icon** — opens New Profile modal.
3. **Manage icon** — opens the profile manager window.
4. Right-aligned statistics: `"<enabled> Active / <total> Mods / <filtered> Displayed"`, with localized fragments and bold text.
5. Filter text field, labelled `Filter`. Filtering is live, case-insensitive substring matching against `meta.name` only. It does not affect selection.

No “apply” button lives here; Apply is in the Pending window.

### 6.2 Empty state [UI-ONLY]

If `displayed_mods` is empty (i.e. no mods at all, not merely all filtered out), the table is replaced by a centered message:

- If the current platform has **no config**: `Helper_Modlist_NoConfig`, interpolating the platform name, plus `Helper_Modlist_NoConfig_Hint`.
- Otherwise: `Helper_Modlist_NoMods` plus `Helper_Modlist_NoMods_Hint`.

If a filter hides every mod, the table body is simply empty; no empty-state message appears. [GAP]

### 6.3 Mod table [UI-ONLY]

Columns, left to right:

1. **Enabled** — a checkbox per row; header is a small transparent button toggling sort by enabled state.
2. **Name** — name, clipped to one line with hover tooltip only when truncated.
3. **Category** — localized category name, centered.
4. **Version** — version string, centered. Sorting by version is **lexicographic** over the string, not semver. [GAP]
5. **Priority** — centered row number.

Sorting:

- Sort modes: Enabled, Name, Category, Version, Priority.
- Default: Priority ascending (load order).
- Clicking a header sorts by it; clicking the active header reverses direction.
- Direction arrows: `⏶` ascending, `⏷` descending; inactive headers get trailing spaces to reserve width.
- Rows are rendered from `displayed_mods` filtered by name, so filtering and sorting are view-only and never mutate `mods`.

Column sizing:

- Icon column exact width = egui icon width + button padding.
- Name column takes the remainder once other widths are known.
- Category column is measured once from the rendered header label, then fixed (a process-global `OnceLock`); before measurement it is `100..240px`.
- Numeric columns exact width measured from the string `PriorityWW`.
- Selection highlight uses the table's selected-row style.

Row interactions:

- Clicking any cell selects per modifier rules below.
- The checkbox toggles enabled state.
- Starting a drag begins reordering; if the row was not selected and Ctrl is not held, the drag start also replaces the selection with that row.
- Right-clicking any cell opens the context menu.
- Hovering a non-dragged row records it as the drop target.
- The drag cursor becomes “grabbing” after six frames of dragging. No drag ghost/preview is drawn (implementation is commented out). [GAP]

### 6.4 Selection rules [APP semantics, UI storage]

- Selection is a vector of cloned `Mod` values, so equality is by the mod's metadata hash.
- Initial selection: first mod in load order.
- Plain click: select only that row. If the row is already in a multi-selection, the selection collapses to it (it does not deselect on second click).
- Ctrl+click on an unselected row: add to selection.
- Ctrl+click on a selected row: remove from selection.
- Shift+click: replace selection with the range from the first selected visible row to the clicked row, in the visible (filtered/sorted) order.
- Arrow Down/Up (when keyboard focus is owned by the mod list and no modal is open):
  - Down picks the **last** selected row in load order; Up picks the **first**.
  - Shift adds the adjacent row; otherwise replaces selection.
  - Arrow Up clamps at row 0; Down can select at most `len-1`.
- Any selection mutation clears the drag index.
- Keyboard navigation requires some egui widget to currently hold focus; it does not focus the table itself. [EGUI]/[GAP]

### 6.5 Drag reordering [UI-ONLY gesture, APP result]

- Drag start stores the row; if the row was not selected, selection is cleared unless Ctrl is held, then the row is added.
- Row hover updates the destination index.
- On pointer release:
  - If start ≠ destination, `MoveSelected(dest)`.
  - Else `ClearDrag`.
- Reorder algorithm:
  1. Clamp destination to `0..len-1`.
  2. If every mod is selected, do nothing.
  3. Remove all selected mods from `mods` (preserving their relative order).
  4. Insert them consecutively at the destination, clamping to the shrinking length.
  5. Extend the dirty manifest with each moved mod's manifest under its enabled options.
  6. Refresh display.
- Moves do **not** touch the core until Apply.
- Context menu alternatives: “Move to start” (`dest = 0`) and “Move to end” (`dest = 9999`, clamped).

### 6.6 Row context menu [APP]

Right-click menu entries:

1. **Send to profile** submenu — every profile except the current one; copies the selected mods into that profile via the core, dirties the target profile manifest, and shows a success toast.
2. **Update** — opens a native picker for one archive, marks the first selected mod as the replacement target, and starts open/install.
3. **Dev Update** — for each selected mod, sequentially opens a native **folder** picker and repacks that mod from the folder.
4. **Uninstall** — opens a confirmation with the clicked mod's name; OK uninstalls all currently selected mods.
5. **Enable/Disable** — label depends on the clicked mod's state; toggles only that mod.
6. **View folder** — opens the mod path (or its parent, for files) using `explorer` on Windows or `xdg-open` elsewhere.
7. **Extract** — opens a native folder picker, then unpacks all selected mods to `<chosen>/<mod name>`.
8. **Move to start** / **Move to end**.

Note that Uninstall acts on **all selected mods**, while Update targets only the **first selected mod**; both entries originate from the clicked row. [GAP if unexpected]

### 6.7 Pending Changes window [APP]

Rendered inside the Mods tab whenever the current profile's `dirty` manifest is non-empty.

- Floating window anchored bottom-right with a 32px offset.
- Collapsible, scrollable, max height 200px.
- Shows a collapsible “Files Pending Update” tree of base (`content`) and DLC (`aoc`) files.
- Apply dispatches `Apply`; Cancel dispatches `ResetMods(None)`, discarding all unsaved working state and reloading from the core.
- The manifest tree is built from file paths split on `/`; it is cached globally by a hash of the file sets.
- The window is neither part of `modal_open()` nor blocking.

### 6.8 Profile manager window [APP]

Opened from the Manage icon. A centered, resizable window (default 320×240) with a two-column grid:

- **Left:** every profile name, selectable; selecting updates the right pane. Profile list is read from `profile.yml` files at construction/reload; a profile whose YAML fails to read is omitted silently.
- **Right:** for the selected profile:
  - Scrollable list of mods in that profile's load order, one line each, wrapped anywhere, max one row.
  - If empty: `Profile_NoMods`, centered.
  - Rename flow: Rename button reveals a text field plus a check icon (tooltip Save). Confirming sends `RenameProfile(old, typed)`.
  - Duplicate button copies the profile directory to `<name>_copy` and registers it.
  - Delete button opens a confirm dialog naming the profile; OK deletes the directory, reloads, and tries to delete stored file-based mod archives not referenced by any remaining profile.
- Close button.

Deletion side effect: `CleanProfile` gathers all mod hashes referenced by all profiles and deletes any mod file from the deleted profile that is now unassigned. This can delete shared archives if bookkeeping is wrong. [GAP]

Duplicate/rename/delete are raw filesystem operations, with the rename target a user-typed string joined to `profiles_dir()`. No validation exists for empty names, `.`, `..`, or path separators. [GAP, explicitly targeted by the refactor]

### 6.9 New Profile modal [UI-ONLY]

- Text input labelled “Enter name for new profile”.
- OK is disabled only when the input is exactly empty (whitespace is accepted, separators are accepted).
- OK calls `core.change_profile(name)`; success resets mods, failure shows an error.
- Close cancels.

---

## 7. Info Tab

Shows details for `selected.first()`, or a centered “No mod selected”.

Contents, top to bottom:

1. **Preview image** — if `thumb.jpg`, `thumb.jpeg`, `thumb.png`, or `thumb.svg` exists at the mod archive root, it is decoded and uploaded as a texture. Textures are cached process-wide by mod hash; failures log and show nothing.
   - The image is stretched to `ui.available_size()` with no aspect-ratio preservation. [EGUI]/[GAP]
   - SVG is supported through `egui_extras::image`.
2. **Metadata rows** — Name, Version, Category, Author. Empty values are omitted. Labels bold, values right-aligned and truncated with tooltip.
3. **Description** — rendered as markdown with `egui_commonmark`, using a cache stored in egui temp data under `md_cache`.
4. **Enabled Options** — only if the mod declares option groups:
   - Header “Enabled Options” with a settings icon that opens the option picker in **update** mode.
   - Each currently enabled option name as a disabled checkbox.
   - If none: “No enabled options”.
5. **Manifest** — header “Manifest”, then:
   - Base Files tree (`content_files`) if non-empty.
   - DLC Files tree (`aoc_files`) if non-empty.
   - Trees are nested collapsible directories; leaf labels show the file name, with full path hover tooltip, single line, break-anywhere.
   - Tree structures cached globally by manifest file-set hash.
   - If manifest loading fails: logs the error and shows literal `FAILED TO LOAD MANIFEST` (unlocalized).

The manifest tree cache and the mod-description markdown cache are cleared on `ResetMods`. Decoded preview textures are **not** cleared. [GAP]

---

## 8. Install Tab (built-in file browser)

An in-app file browser, independent of the OS dialog.

### 8.1 Toolbar

- **Open** icon — native multi-select picker (same as File → Install mod).
- **Up** — parent directory, pushes current onto history.
- **Back** — pops history.
- **Refresh** — reloads current directory.
- **Path text field** — typing a path and submitting navigates if it is an existing directory.

### 8.2 Directory listing [UI-ONLY]

Only these entries are shown:

- Directories.
- Files with extension `zip`, `rar`, `7z`, `bnp`.
- Files named `rules.txt` or `info.json`.
- Hidden names starting with `.` are excluded.

Ordering: directories first, then files, each case-insensitively sorted by filename.

Each row is an image-and-text button: folder icon or archive icon; selected row filled with the selection color. Click selects; double-click or Enter on the selected row navigates into directories or opens files as mods.

Arrow Up/Down moves the browser selection when the file-picker pane owns focus and no modal is open.

### 8.3 Native picker filters [HOST]

`SelectFile` and `ModUpdate` use the same filters:

- `Any mod (*.zip, *.rar, *.7z, *.bnp, rules.txt)` → `zip,rar,7z,bnp,txt`
- `UKMM Mod (*.zip)` → `zip`
- `BCML Mod (*.bnp)` → `bnp`
- `Legacy Mod (*.zip, *.rar, *.7z, rules.txt)` → `zip,rar,7z,txt`
- `All files (*.*)` → `*`

Multi-select for Install; single-select for Update. All but the first selected path go into the install queue.

Persisted browser state: current path, history, typed path. `selected` is not persisted; entries are reloaded from the path on deserialize. Default path: user's Downloads folder, else home.

---

## 9. Install / Open Pipeline

This is the most important domain workflow.

### 9.1 Open and classify [APP]

`uk_app::open_mod(core, path, meta)`:

1. Log “Opening mod at …”.
2. If the path sniffs as a BNP (directory with mod roots + `info.json`, `.bnp` archive with mod root and `info.json`, or an `info.json` beside mod roots), convert it with `convert_bnp`.
3. Otherwise try `ModReader::open_peek`. On success, construct `Mod` directly.
4. On failure, inspect the typed `OpenError`:
   - If it is not a UKMM mod, sniff whether the path is a legacy mod and whether it has metadata (`rules.txt`).
   - If it is not a legacy mod, return the original error.
   - If it is a legacy mod without metadata and no `meta` was supplied, return `Opened::NeedsMeta(path)`.
   - Otherwise convert via `convert_gfx` using the supplied metadata, then open the result.
5. Result is either `Opened::Mod(Mod)` or `Opened::NeedsMeta(path)`.

BNP detection roots: `content`, `aoc`, `01007EF00011E000`, `01007EF00011F001`, `logs`. Legacy detection roots omit `logs` and use `rules.txt` as metadata. RAR, 7z, ZIP, directories, and `rules.txt` itself are handled.

### 9.2 Metadata prompt [UI-ONLY]

When `NeedsMeta` is returned:

- Opens a centered, non-collapsible window.
- Message: details could not be determined.
- Fields:
  - Name (prefilled from file stem, underscores replaced with spaces).
  - Category combo (all `ModCategory` values, localized).
  - Description multiline, markdown supported note.
- OK re-opens the same path with the filled `Meta`.
- Close clears the modal and sends `Noop`.
- Platform is set to the current platform; version defaults to `1.0.0`; author empty; category `Other` from `Meta::blank`.

### 9.3 Acceptance checks [APP — currently in reducer]

`HandleMod(mod)` runs before install:

1. Clear busy.
2. **Master check:** for every `meta.masters` entry `(hash, (name, version))`, if no installed mod has that hash, show an error: `Could not find required mod dependency <name> (version <version>)`.
   - Non-blocking: installation continues even after reporting missing masters.
3. **Platform check:** if the mod is platform-specific and does not match the current platform, show an error: `Mod is for <Switch|Wii U|any console>, current mode is <platform>` and stop.
4. If the mod has option groups, request options (after enabling default picks in install mode).
5. Otherwise install immediately.

### 9.4 Option picker [UI-ONLY rendering, APP gating]

A centered, scrollable window.

For each option group (collapsible, open by default, showing description):

- **Exclusive group:**
  - If not required, a “None” radio clears the group's picks.
  - One radio per option; choosing one removes other picks from the same group.
  - Option descriptions shown under each.
- **Multiple group:**
  - One checkbox per option; toggling adds/removes it.
  - Descriptions shown.

At the bottom:

- If any required group has no selection, show red `You must set all required option groups` and disable OK.
- OK either installs (install mode) or updates options on the existing mod (update mode), then closes.
- Cancel closes without action.

Install mode calls `enable_default_options()` before display: exclusive-group `default` and multiple-group `defaults` are preselected.

### 9.5 Add/update mechanics [APP]

**Fresh install** (`install_queue` path):

- Add mod to current profile. The core copies/archives it into the platform mod store under a sanitized `<Name>.zip`, enables it, and appends it to the end of the load order.
- If the user picked options, set them (this computes the manifest).
- Save the profile.
- Return `AddMod(new_mod)`.

**Update** (`update_mod` path, from context menu Update):

- Build dirty from the new mod's manifest.
- Replace the old mod by hash, preserving the old enabled flag and path.
- Dirty also absorbs the old mod's manifest.
- Return `ResetMods(Some(dirty))`.

**Batch behaviour:**

- Queue is processed one file at a time.
- On error: if more installs are queued, stash the error and immediately start the next; otherwise show the error modal.
- After the last success, if errors were stashed, show one aggregate error with the context “One or more errors occured while installing your mods. Please see full details.”

### 9.6 Uninstall [APP]

- Target list is explicit or the current selection.
- For each, delete from the current profile; the core deletes the stored archive/directory only if no other profile references that hash.
- Save once after all deletions.
- Return `RemoveMods(list)`; GUI removes them from working state, selection, and dirties their manifests.

---

## 10. Deploy Tab

Reads only the **current platform's** deploy config; if absent, shows a centered `No deployment config for current platform`.

When present:

- **Deploy Method** — localized method name (Copy / Hard Links / Symlink).
- **Auto Deploy** — localized Yes in green or No in red.
- **Output Folder** — path rendered as a one-line link with full-path tooltip; clicking opens the directory (or its parent if the configured path is a file) in the OS file manager.
- **Open Emulator** — only if an emulator executable is configured:
  - The command string has `\` replaced by `/` and is split with shell-word rules.
  - The first token is the executable, the rest are arguments.
  - It is spawned directly (not via a shell), then the UI thread sleeps one second and calls `try_wait`.
  - Non-zero immediate exit produces “Emulator exited with non-zero exit status …”.
  - Spawn failure produces “Failed to open emulator”.
  - This runs inside render code, blocking the UI for at least a second and ignoring the job system. [GAP]
  - Empty/malformed command panics on `iter.next().unwrap()`. [GAP]
- **Deploy** button — immediately runs deployment of the current merged state.
- If Auto Deploy is on and the deploy manager still reports pending work, show red `Auto deploy incomplete, please deploy manually`.

The tab does not edit settings; those fields are in Settings.

---

## 11. Settings Tab

A vertical scroll area with three collapsing sections and a footer. Every setting label is rendered in bold with an info icon whose hover text is the setting's description.

### 11.1 General section (open by default)

- **Migrate from BCML** — runs the BCML importer (see §17.2). On success: settings draft reload, success toast, dump cache clear, package builder reset, selection clear, mods reset.
- **Register 1-Click Handler** — registers the `bcml:` URL scheme handler (see §17.5). Success only logs; failure opens error modal.
- **Theme** combo — 11 themes, applied immediately (`SetTheme`), including dock style recompute. Persisted only in `ui.json`.
- **UKMM Language** combo — 12 UI languages, applied immediately (global localization). Persisted in core settings only when Save is pressed.
- **Current Mode** — radio Wii U / Switch. Changes which platform config and storage tree are active after save.
- **Storage Folder** — folder picker with manual path editing.
- **Use System 7z** — checkbox controlling extraction backend preference.
- **Show Changelog** — checkbox. Stored but unused by this frontend. [GAP]

### 11.2 Platform config sections

Two collapsing sections: `Switch Config` and `Wii U Config`. Each renders a draft of that platform's `PlatformSettings`, even when the other platform is active. Editing both is supported.

Wii U section additionally has **Import Cemu Settings** at the top.

Per-platform fields:

1. **Game Language** — combo over `Language` values (game text language, separate from UI language).
2. **Game Dump** group:
   - Wii U only: **Dump Type** radio Unpacked / WUA. Clicking either radio replaces the dump with fresh default fields, even when re-clicking the active type. [GAP]
   - Unpacked:
     - Base folder (Wii U) or “Base with Update Folder” (Switch).
     - Update folder (Wii U only).
     - DLC folder.
     - Changing any folder sets the serialized `host_path` to `/`.
   - WUA: single `.wua` file picker.
3. **Deployment** group:
   - Method: Copy / Hard Links / Symlink.
   - Layout: platform-specific labels:
     - Wii U: Without Named Folder / With Named Folder.
     - Switch: Atmosphere Layout / Emulator Mod Layout.
   - Auto Deploy checkbox.
   - Deploy `rules.txt` checkbox (Wii U only).
   - Output folder.
   - Emulator executable — file picker over a string command line.

### 11.3 Draft and validation model [APP]

Two parallel representations exist:

- `PlatformSettingsUI` — editable UI model: language, profile name, `DumpType` (Unpacked / ZArchive), `DeployConfig`.
- `PlatformSettings` — real core model holding an `Arc<ResourceReader>` and optional deploy config.

`CONFIG` is a process-global `RwLock<HashMap<Platform, PlatformSettingsUI>>` that stages edits. It is initialized lazily from the existing settings, persists across tab switches, and is cleared on Reset or after a successful save.

Conversion (`TryFrom<PlatformSettingsUI> for PlatformSettings`) does the following:

- **Unpacked dump:** infers endianness from `content_dir.to_string_lossy().contains("content")`: contains → Big (Wii U), otherwise Little (Switch). This is a heuristic, not a platform check. [GAP, explicitly targeted]
- Builds `ResourceReader::from_unpacked_dirs(content, update, aoc, endian)`, which validates the directories.
- **ZArchive:** builds `ResourceReader::from_zarchive(host_path)`.
- If no content dir exists at all, returns `ROMError::MissingDumpDir("Base", …)`.
- Deploy config is stored as `None` if its output path is empty; otherwise `Some(config)`.
- Profile name comes from the draft string.

`PartialEq<PlatformSettings>` compares game language, whether the persisted deploy config equals the draft deploy config, and the dump host path; this drives the unsaved-changes indicator.

### 11.4 Save and Reset [APP]

The Save button is enabled when:

- `temp_settings` differs from core settings, **or**
- either platform's draft differs from its persisted config (using dump emptiness and `PartialEq` rules above).

Save sequence:

1. For each platform marked changed, convert its `CONFIG` entry to `PlatformSettings`. A conversion failure opens the error modal and aborts the save.
2. On conversion success, remove the entry from `CONFIG` and write the result into `temp_settings`.
3. **Wipe rule:** if the current platform had an old deploy config and the new one changes **layout, method, or output**, call `core.settings().wipe_output(current_platform)`. If that succeeds, schedule a pending reset.
4. Save `temp_settings` to `settings.yml`, then `core.reload()` (rebuilding settings, mod, and deploy managers).
5. On success:
   - Success toast “Settings saved”.
   - Clear the dump cache.
   - Reset the package builder for the (possibly new) platform.
   - Clear selection.
   - Reset mods and reload profiles.
   - If the wipe rule fired, reset pending deployment data.
6. On failure: error modal; `CONFIG` was already partially consumed. [GAP]

Reset sequence:

- Re-apply core UI language.
- Clear all `CONFIG` drafts.
- Reload `temp_settings` from core.
- Clear busy.

Changing UI language previews immediately; Reset restores the saved language immediately.

Changing the storage folder does **not** migrate existing data; it only changes where future storage paths point after save.

---

## 12. Package Tab

A form for building a UKMM mod archive.

### 12.1 State [APP]

`ModPackerBuilder { source: PathBuf, dest: PathBuf, meta: Meta }`.

Initialized with blank metadata for the current platform; reset after settings save/import and after successful packaging.

### 12.2 Toolbar

- **Manage Options** — enabled only when the source path exists. Scans `<source>/options/` for subdirectories and opens the options editor.
- **Dependencies** — opens the dependency chooser.
- **Help** — opens `https://nicenenerd.github.io/UKMM/mod_format.html`.

### 12.3 Options editor modal [UI-ONLY]

Centered, vertically scrollable.

- “Add Option Group” appends a Multiple group with defaults.
- Each group is a collapsible header showing its name or “New Option Group”:
  - Delete group.
  - Group Name (single-line).
  - Group Description (multi-line).
  - Group Type radio Exclusive / Multiple. Converting preserves name, description, current options, and required flag; exclusive gets `default: None`, multiple gets empty defaults.
  - Required checkbox with tooltip.
  - Exclusive only: Default Option combo listing the options. When no default is set, the closed combo displays “None”, but there is no explicit None item to clear an existing default.
  - “Add Option” enabled only when at least one folder is available; appends a blank option.
  - Each option is a collapsible sub-header:
    - Delete option.
    - Option Name, Description.
    - Multiple only: “Enable by default” checkbox, maintaining the group's default path set.
    - Option Folder combo listing `source/options/*` directory names. Selecting a folder consumes it from the available set and stores its file name as the option path; if a previous path existed, a renamed folder is inserted back into the available set. The multiple-defaults set is updated accordingly.
- OK closes.

The available-folder set is held in a `Mutex<FxHashSet<PathBuf>>` inside `App`, so option assignment is destructive within a single editing session.

### 12.4 Dependencies modal [UI-ONLY]

Centered, scrollable list of every installed mod in the current profile, each with a checkbox `<name> (v<version>)`. Checking inserts `hash -> (name, version)` into `meta.masters`; unchecking removes it. OK closes.

### 12.5 Form fields [APP semantics]

- **Source** — folder picker. On change, scan the source root for `info.json`, then `rules.txt`, then `meta.yml`; parse the first found and load it into the form.
- **Mark as cross-platform** — toggles `meta.platform` between `Universal` and `Specific(current platform)`.
- **Name** — single-line.
- **Version** — single-line. The edit buffer is reseeded from `meta.version` every frame; on change, the model updates **only** if the entire text parses as a lenient semantic version, so partial or invalid edits are discarded on the next frame. [GAP]
- **Author** — single-line.
- **Category** — combo over all categories, localized.
- **URL** — single-line; empty stores `None`.
- **Description** — multi-line markdown.
- **Package Mod** — enabled only if source path is non-default and exists and name is non-empty. Opens a native save dialog with default filename `sanitise(meta.name) + ".zip"` and filter `UKMM Mod (*.zip)`. Runs packaging with the source, chosen destination, and metadata; on success resets the builder.

Packaging itself is `uk_app::package_mod`: requires a configured dump for the current platform, creates a `ModPacker`, and packs.

---

## 13. Log Tab

- Renders `egui_logger::logger_ui()` in a frame filled with `extreme_bg_color`.
- Shows log records collected by the custom `Logger`, which:
  - Accepts only records whose target starts with `uk`.
  - Treats `INFO` and messages beginning with `PROGRESS` as the current progress line shown in the busy modal.
  - Treats `CLEARPROGRESS` as clearing the progress line and does not display it.
  - Copies non-progress records into the egui logger and appends timestamped, leveled lines to an in-memory buffer.
  - Flushes the buffer to `config/log.txt` at 1024 lines, and on exit/flush.
  - Truncates the log file at startup.
- No level selector, search, or clear button exists in the UI. (The book mentions a Help-menu verbose toggle, but the current code has none.) [GAP]

---

## 14. Global Dialogs and Overlays

### 14.1 `modal_open()` definition [EGUI]

A modal state is any of:

`error`, `busy`, option picker, confirm, About, new profile, packaging dependencies, packaging options, metadata prompt, changelog.

It does **not** include the profile manager or Pending Changes window. When true:

- Menu bar disabled.
- All dock tabs disabled.
- File drops ignored.

### 14.2 Error window [UI-ONLY]

- Logs the error on receipt.
- Centered, auto-sized window titled “Error”.
- Primary label:
  - Chain length 0–1: generic “An error has occurred. Check the details for more information.”
  - Otherwise: first error and last error joined with `": "`.
- Collapsible **Details**: numbered chain entries in monospace.
- If any chain element is a `uk_content::UKError` with context data, a collapsible **Data Context** shows it with debug formatting.
- Buttons: OK closes; Copy copies the full debug error chain.
  - There are two “Copy” buttons in the code: one only has a hover tooltip (“Copied”), the other performs the copy. This is a UI defect. [GAP]
- Only one error can be displayed at a time.

### 14.3 Confirm window [UI-ONLY]

Centered, auto-sized, titled “Confirm”. Shows the prompt and OK/Cancel. OK dispatches the stored `Message` and then closes.

### 14.4 Busy window [EGUI]/[APP]

Centered window titled “Working”, default 240×80, non-collapsible:

- Large spinner.
- “Processing…” label.
- Optional progress text from `Logger::get_progress()`, truncated to one line.
- No cancel button. No percentage bar.

### 14.5 About window [UI-ONLY]

Fixed 360×240, centered:

- “U-King Mod Manager” heading.
- Copyright (`© 2022-<current year> Caleb Smith, Ginger Chody - GPLv3`).
- Version.
- GitHub link, opens browser.
- “Gui Library:” and an `egui` link.
- OK.

### 14.6 Changelog / update window [UI-ONLY]

Centered, 600×500, scrollable, non-collapsible.

- Markdown body from either the intro, a release description, or “A new update is available!” plus release notes.
- Patreon button (opens `https://www.patreon.com/nicenenerd`).
- Bitcoin button: copies `392YEGQ8WybkRSg4oyeLf7Pj2gQNhPcWoa` to the clipboard and shows an info toast.
- If an update is offered: Update and Cancel buttons. Update starts download/install and closes the window; Cancel closes.
- Otherwise: OK closes.

### 14.7 Metadata prompt, option picker, profile manager, package dialogs

Described in their workflow sections (§9.2, §9.4, §6.8, §12).

---

## 15. Apply, Merge, Deploy, and Pending State

### 15.1 What is staged where [APP]

There are three layers:

1. **Core profile state** — committed `enabled`, `enabled_options`, and load order. Modified immediately by some actions (`UpdateOptions`, AddToProfile, install, uninstall, profile operations).
2. **Frontend working state** — `App.mods` and `App.dirty`. Most enable/disable/reorder actions only change this.
3. **Deployment pending log** — `pending.yml` plus the core's notion of files awaiting deployment.

`Apply` reconciles layer 2 into layer 1, then runs a merge limited to the accumulated dirty manifest.

### 15.2 Apply sequence [APP]

1. Clone `App.mods` (the working load order and flags).
2. `mem::take` the current profile's dirty manifest.
3. Run a job:
   - For every mod in the supplied list that differs in `enabled` or `enabled_options` from the installed copy, call `set_enabled` and `set_enabled_options`.
   - Set the entire load order from the list.
   - Save the profile.
   - `deploy_manager().apply(Some(dirty))` to merge only affected files.
   - If Auto Deploy is on, immediately `deploy()`.
4. Return `ResetMods(None)`: clear the current profile's dirty manifest, reload from core, update display, reload profile manager, clear caches.

If `mods` is empty, the configuration is left untouched and only a merge runs. This is how `Remerge` (dirty `None`) and a full reprocess are expressed.

### 15.3 Deploy vs Apply

- **Apply** commits UI changes and merges into the profile's `merged` directory.
- **Deploy** copies/links the merged directory to the configured output.
- Auto Deploy does both after Apply.
- “Refresh merge” runs merge without changing configuration.
- “Reset pending” recomputes the pending set from the current merged output and increments for a full redeploy.

### 15.4 Toggle/option/reorder dirty tracking [APP]

- Toggle: adds the mod's manifest under its currently enabled options.
- Reorder: adds each moved mod's manifest.
- Install: adds new manifest (and old manifest when updating).
- Uninstall: adds removed manifests.
- Option change: extends with the manifest returned by `set_enabled_options`.
- AddToProfile: extends the **target** profile's dirty manifest, but neither the target profile nor the current profile is saved by this action; the addition is applied only when that profile becomes current and Apply is used. [GAP]

Because manifests are unioned per profile, Apply is incremental and skips unrelated game files.

---

## 16. Profiles: Semantics and Operations

- Profiles are directories under `<storage>/<platform>/profiles/`.
- A profile consists of `profile.yml`, which serializes mod map + load order.
- Current profile name is stored in the platform config (`PlatformSettings.profile`).
- Switching profiles calls the core, which creates the profile directory if missing, reloads the mod working set, and updates profile-manager data. The dirty manifests are **not** cleared or reloaded per profile; all profiles are loaded into one map at startup and fall back to the first value if the current key is absent. [GAP]
- New profile: no name validation. [GAP]
- Duplicate: directory copy to `<name>_copy`; if that exists, the copy fails. [GAP]
- Rename: raw `fs::rename` from one joined path to another. [GAP]
- Delete: removes the directory; then attempts to delete file-based mod archives referenced only by that profile. Confirmation required.
- “Send to profile”: copies selected mod archive paths into another profile using the core; does not switch profile.

Profile store data displayed by the manager is loaded directly from `profile.yml`, not from the core, and omits unreadable profiles silently.

---

## 17. Imports, Updates, and OS Integration

### 17.1 Cemu settings import [APP]

Triggered from Settings → Wii U Config. Prompts for the Cemu directory (folder picker), then:

1. Locate `settings.xml`:
   - `<cemu>/portable/settings.xml`
   - On Windows: `<cemu>/settings.xml`
   - Fallback: config dir `Cemu/settings.xml`
2. Locate `mlc_path` from XML, or `<cemu>/portable/mlc01`, or platform-specific `mlc01`.
3. If `title_list_cache.xml` exists (config or local data Cemu dir), parse every `<title>`:
   - Accept region suffixes `101C9400/101C9500/101C9300` in either case.
   - Parse `<format>` and `<path>`.
   - Resolve relative paths against the chosen Cemu dir, portable dir, local Cemu dir, or config Cemu dir.
   - Map:
     - `00050000`, format 1 → base `<path>/content`.
     - `0005000e`, format 1 → update `<path>/content`.
     - `0005000c`, format 1 → DLC `<path>/content/0010`.
     - `00050000/0e/0c`, format 4 → encrypted dump flag.
     - `00050000`, format 3 → WUA file.
4. Else derive from `<mlc>/usr/title/...` paths, with a fallback that scrapes a `<path>` line containing `U-King.rpx` from `settings.xml`.
5. Require base + update, or a WUA.
6. If encrypted: error “UKMM does not support encrypted dumps.”
7. Find graphics pack folder (`portable/graphicPacks`, or config Cemu `graphicPacks`).
8. Set current mode to Wii U and install a `PlatformSettings`:
   - Validated dump (WUA or unpacked with Big endian).
   - Deploy output = graphics pack folder.
   - Auto deploy on, Cemu rules on, symlink method, “With Named Folder” layout.
   - Emulator command: `Cemu.exe` (Windows) or `Cemu`/last `Cemu-*-*.AppImage` (Unix), quoted if it contains spaces, plus `-g "<base>/../code/U-King.rpx"`.
9. Save settings.

The importer does not call `core.reload`; the GUI just refreshes its settings draft.

### 17.2 BCML migration [APP]

Reads BCML's `settings.json` from platform config dir `bcml/settings.json` (Windows: data-local). Fields: language, Cemu dir, export dirs, game dirs, update/DLC dirs, store dir.

It can import:

- **Wii U:** if game and update dirs exist, create Wii U platform settings with Big-endian dump; output = BCML export dir if present, else `<cemu>/graphicPacks/BreathOfTheWild_UKMM`; Cemu rules if Cemu dir present; set mode Wii U; save; reload; import mods from `<store>/mods`.
- **Switch:** if `game_dir_nx` exists, create Switch platform settings with Little-endian dump; output = `export_dir_nx`; set mode Switch; save; reload; import mods from `<store>/mods_nx`.
- Restore the original current mode, save, reload.
- Imports each non-`9999*` directory as a BNP conversion, adds it to the current profile, and saves.

On success the GUI reloads its settings draft and resets mod/package/selection state.

### 17.3 Update check [APP]

At startup (unless first run/no config):

- GET `https://api.github.com/repos/NiceneNerd/UKMM/releases?per_page=10` with User-Agent `UKMM`, following redirects.
- Parse up to ten releases.
- Include prereleases only if update preference is `Beta` or the running version is below `1.0.0`.
- Take the newest release whose tag parses after stripping a leading `v`.
- Compare with the running crate version:
  - Newer → offer update (`OfferUpdate`) with release body/name/tag.
  - Older → show that release's notes as a changelog.
  - Equal → nothing.

Network and parse failures only log a warning; there is no UI. This runs synchronously at startup and can delay the window. [GAP]

### 17.4 Self-update [HOST]

Asset names by platform:

- Windows: `ukmm-x86_64-pc-windows-msvc.zip`
- macOS ARM: `ukmm-aarch64-apple-darwin.tar.xz`
- macOS x86_64: `ukmm-x86_64-apple-darwin.tar.xz`
- Linux: `ukmm-x86_64-unknown-linux-gnu.tar.xz`
- Other: empty (fails with “No matching platform for update”)

Sequence:

1. Find matching asset; error if absent.
2. Download it.
3. Write to a temp file.
4. Windows: extract ZIP into the temp directory, rename current exe to `.bak`, copy `ukmm.exe` over the current exe.
5. Unix: rename current exe to `.bak`, run `tar xf <temp> -C <exe dir> --overwrite`; non-empty stderr is an error.
6. Dispatch `Restart`: spawn current exe (if its extension contains `bak`, strip it), on Unix put it in its own process group, then close the viewport.

The update dialog closes as soon as the download job starts.

### 17.5 URL-handler registration [HOST]

Linux:

- Write `~/.local/share/applications/ukmm.desktop` (if absent):
  - `Type=Application`, `Name=UKMM`, `Exec=<current exe> %u`, `StartupNotify=false`, `MimeType=x-scheme-handler/bcml;`.
- Run `xdg-mime default <desktop> x-scheme-handler/bcml`.
- Run `update-desktop-database <applications dir>`.

Windows:

- Create/open `HKCU\Software\Classes\bcml`.
- Set `URL Protocol` empty.
- Create `shell\open\command` and set default value to `"<current exe>" "%1"`.
- If an existing command already contains the current exe path, do nothing.

Only the `bcml:` scheme is registered (GameBanana one-click links).

### 17.6 GameBanana one-click and IPC [APP]/[HOST]

- IPC uses `ssilide` on port 6666. Message JSON variants: `OpenMod(path)`, `Error(string)`, `Starting(mod_name)`.
- If an existing UKMM instance claims the socket:
  - Process the URL; send the resulting message to the existing instance; exit.
- If no instance exists:
  - Start processing on a background thread, wait for the UI's `ONECLICK_SENDER` to be published, then send the result into the UI.
- URL format: `url,category,id` (the `bcml:` prefix is stripped by `main`).
- Processing:
  1. Query `https://api.gamebanana.com/Core/Item/Data?itemtype=<cat>&itemid=<id>&fields=name`; sanitize the name, fallback `oneclick_mod`.
  2. If an IPC client connects, send `Starting(name)` (UI sets busy and logs).
  3. GET the download URL with User-Agent `UKMM`, redirect policy limit 0.
  4. Read the `Location` header, derive the filename from its path (fallback `<name>.bnp`), and fetch the redirect target.
  5. Save under a temp file and send `OpenMod(temp_path)`.
  6. Any error becomes `Error`.
- The listener thread claims the socket and loops receiving up to 1024-byte JSON messages, forwarding them to the UI. Failure to claim reports “One-Click listener failed to initialize. Is UKMM already open?” via the platform error display.

### 17.7 File manager and external links [HOST]

- “View folder” uses `explorer` on Windows and `xdg-open` elsewhere, ignoring failures.
- Tools menu, deploy output link, Help menu, About links, Patreon, and package Help use the `open` crate and ignore failures.
- Emulator launch is the only long-lived child process monitored for exit status; the updater's `tar` command is separately checked for stderr output.

---

## 18. Presentation System

### 18.1 Themes [EGUI]/[UI-ONLY]

Eleven themes, persisted in `ui.json`:

1. Sheikah Slate (default)
2. egui Dark
3. egui Light
4. Frappe
5. Latte
6. Macchiato
7. Mocha
8. Adwaita Dark
9. Adwaita Light
10. Carl
11. Sweet Dark

Sheikah is fully hand-specified: dark blue-gray palette, zero-radius non-interactive widgets, 2px-radius interactive widgets, blue selection, 4px window corners, 5px blur/spread shadows, 0.2s animation, compact button padding (4×2), menu margin 4, no collapsing-header frame, heading font family “Bold”.

Sweet Dark is also fully hand-specified. Frappe/Latte/Macchiato/Mocha use catppuccin-egui; Adwaita themes use egui-aesthetix; egui themes use stock visuals.

Named colors:

| Name | Hex |
|---|---|
| GREEN | `#528f24` |
| BLUE | `#38b6f1` |
| RED | `#F52331` |
| YELLOW | `#ffbc28` |
| ORANGE | `#ff953f` |

`error_bg` derives from RED with its HSV value replaced by the current window fill's value.

Theme changes apply immediately on selection. They are saved only at exit, not in core settings.

### 18.2 Fonts [EGUI]/[HOST]

At startup:

- Pick a system UI font family: `Segoe UI` (Windows), `Lucida Grande` (macOS), or the GNOME `font-name` setting, fallback `Ubuntu` (Linux).
- Load regular and bold variants from the OS font loader when available.
- Always embed compressed Noto Sans Regular/Bold, Noto Sans JP, KR, SC regular/bold.
- Proportional family order: System, Noto, NotoJP, NotoKR, NotoSC, then egui defaults.
- Register a custom `Bold` family: system bold, NotoBold, JP/KR/SC bold.
- Heading text style uses the Bold family.

This gives CJK coverage regardless of installed fonts.

### 18.3 Icons [EGUI]

22 SVG icons are loaded once as textures: Add, ArrowBack, ArrowUp, Bitcoin, Blank, Cancel, Check, Delete, Help, Import, Info, FolderOpen, FolderZip, Folder, List, Menu, Patreon, Refresh, Reset, Save, Settings, Tune.

- `icon_button` draws an image button tinted with the inactive text stroke and half padding.
- `icon_text_button` draws an image-and-text button.
- Blank is an empty SVG used for unchecked Window-menu items.

### 18.4 Localization [APP]/[UI-ONLY]

- UI string keys are looked up in a global `LOCALIZATION` map, falling back to English, then to the key itself.
- 12 UI languages: English, Dutch, French, German, Italian, Japanese, Korean, Russian, Simple Chinese, Spanish, Arabic, Vietnamese.
- Language is initialized from core settings and previewed immediately in Settings.
- English is canonical (`crates/uk-localization/localization/en.json`).
- Localized strings cover menu items, tabs, settings labels/descriptions, dialogs, categories, profile/package/mod operations, and hints.
- Not localized (hard-coded English): many error strings from `anyhow`, log messages, `Browse…`, `FAILED TO LOAD MANIFEST`, the settings-unsaved warning, About product name/labels, `PriorityWW` measurement sentinel, and `Default` profile fallback.
- Format-string interpolation uses `strfmt` with named variables (`{platform}`, `{mod_name}`, `{profile_name}`).
- Composite sentences such as the profile counts are assembled from localized fragments plus fixed punctuation/order, so translators cannot reorder the whole sentence.
- Arabic has a translation but there is no right-to-left layout handling. [GAP]

### 18.5 Toasts [EGUI]

- Anchored bottom-right.
- Success (green): settings saved, mods added to a profile.
- Info: BTC address copied.
- Most durations are 2 seconds.
- No persistent notification center or history.
- Toast text is localized; generic `Toast` accepts arbitrary text.

### 18.6 Markdown [EGUI]

- Mod descriptions, changelog, intro, and update notes render through `egui_commonmark`.
- SVG images are enabled for markdown.
- Separate caches for mod description (`md_cache`) and changelog (`md_cache_changelog`).
- No raw HTML or external link handling beyond commonmark defaults.

### 18.7 Decorative elements [EGUI]/[UI-ONLY]

- Sheikah Mods-tab grid: 48px spacing, offset 16px, faint blue lines, a subtle tessellated border.
- Bold headings, colored Yes/No, red warnings.
- No icons in the mod table itself.
- No custom animations beyond egui defaults and the spinner.

### 18.8 Accessibility and keyboard [EGUI]/[GAP]

- No explicit accessibility labels, roles, or screen-reader annotations.
- Tooltips are the only descriptions for icon buttons.
- Keyboard support is limited to list navigation (arrows), file-picker arrows/Enter, and standard egui text editing/tab focus.
- No focus outlines beyond egui defaults.
- No keyboard shortcut for Install, Apply, Deploy, Save Settings, search/filter, or closing dialogs (Escape behavior is whatever egui supplies).

---

## 19. Domain Policies Currently Enforced in the GUI

These must move below the frontend for parity and correctness:

| Policy | Current location | Rule |
|---|---|---|
| BNP/legacy detection and conversion routing | `uk_app::open_mod` (recently extracted) | `.bnp`/sniffed BNP → convert; non-UKMM archives → legacy conversion with metadata; typed `OpenError` distinction |
| Platform compatibility | `HandleMod` in `update.rs` | Universal accepts all; Specific must equal current mode; message renders Switch/Wii U/any console |
| Master dependencies | `HandleMod` | Every `masters` hash must exist among installed mods; reports but does not block |
| Required option groups | `options.rs` render code | OK disabled until every required group has at least one pick |
| Default options | `RequestOptions` reducer | Enabled before showing picker in install mode |
| Load order | `App.mods` | Vector order is priority; only committed by Apply |
| Enabled/option staging | `App.mods` + dirty | Working changes not saved until Apply |
| Pending manifest | `App.dirty` per profile | Union of affected files, drives incremental merge |
| One mutating job | `App.busy` | UI disabled while a job runs |
| Profile names | New-profile modal and profile ops | **No validation** (target of refactor) |
| Profile storage | `profiles.rs` + core | Raw `fs` under profiles dir |
| Settings save/wipe | `SaveSettings` | Wipe output only when layout/method/output changed; reset pending on success |
| Endian inference | `PlatformSettingsUI` conversion | Substring `content` in content path → Big, else Little |
| Mode/dump/config association | Settings draft + core | Platform configs independent; only current mode used at runtime |
| Metadata-less mod adoption | Metadata modal | Require user-supplied name/category/description before conversion |
| Option folder assignment | Package options editor | Folders are consumed from a set; paths are file names |

---

## 20. Persistence and On-Disk Artifacts

| Artifact | Location | Written by | Contents |
|---|---|---|---|
| `settings.yml` | `<config>/settings.yml` | core Settings | Mode, storage dir, 7z preference, update preference, changelog flag, last version, both platform configs, UI language |
| `ui.json` | `<config>/ui.json` | GUI only | Theme, sort/direction, picker path/history/input, dock layout |
| `log.txt` | `<config>/log.txt` | Logger | Append/flush of UKMM log lines; truncated on launch |
| `profile.yml` | `<storage>/<platform>/profiles/<name>/profile.yml` | core | Mod map and load order |
| `pending.yml` | `<storage>/<platform>/pending.yml` | deploy manager | Files pending deployment; legacy format auto-migrated |
| Mod archives | `<storage>/<platform>/mods/<sanitized name>.zip` | core | Installed mods |
| `merged/` | `<storage>/<platform>/profiles/<profile>/merged/` | deploy manager | Intermediate merged files |
| Deploy output | User-configured | deploy manager | Copy/hardlink/symlink tree |
| Temp files | Manager temp dir / OS temp | update/one-click | Downloads and staging |

`Settings::state_file()` is shared by any frontend and currently points to `ui.json`. A second frontend would overwrite the first's layout; the refactor should make this frontend-specific.

Persisted UI state is limited to theme, sort/direction, picker path/history/input, and dock layout. Filter text, selection, closed-tab memory, scroll positions, and profile-manager selection reset each launch.

Portable mode (`--portable`) changes config dir to `<exe dir>/config` and storage default to `<exe dir>/data`.

---

## 21. Egui-Specific vs Portable

### 21.1 Portable requirements (any GUI must have these to reach parity)

**Shell and navigation**

- A main window with a top menu bar containing File, Tools, Window, Help.
- File → install mod, exit.
- Tools → refresh merge, reset pending, open config/storage/deployment folders.
- Window → reset layout and toggle each of seven functional areas.
- Help → external docs, About.
- Seven areas: Info, Install, Deploy, Mods, Log, Settings, Package. They may be tabs, panes, pages, or windows; docking is not essential to the feature set, but all must be reachable simultaneously or via navigation.
- Current platform displayed persistently.

**Mods**

- Profile selector, new-profile action, profile manager, active/total/filtered counts, text filter.
- A mod list with enabled toggle, name, category, version, priority.
- Sorting by all five fields and direction toggling.
- Single, additive (Ctrl), and range (Shift) selection.
- Drag reorder, move-to-start/end.
- Context actions: send to profile, update, dev update, uninstall, enable/disable, open folder, extract.
- Pending-changes surface with Apply and Cancel.
- Empty-state guidance for no mods and no platform config.

**Info**

- Preview image from archive `thumb.*`.
- Name/version/category/author.
- Markdown description.
- Enabled options list and an edit action.
- Manifest trees for base and DLC files.

**Install**

- A way to select one or many mod files.
- Native file picker and/or built-in browser.
- Batch queueing with sequential processing and aggregated errors.

**Deploy**

- Show method, auto-deploy, output folder.
- Open output folder.
- Launch configured emulator.
- Manual Deploy.
- Warn when auto-deploy is incomplete.

**Settings**

- General: BCML migration, one-click registration, theme, UI language, mode, storage folder, system-7z, show-changelog.
- Per-platform: game language, dump selection (unpacked folders or WUA), Cemu import (Wii U), deploy method/layout/auto/rules/output/emulator.
- Draft semantics with unsaved indicator, Save, Reset.
- Output wipe and pending reset when deployment identity changes.

**Package**

- Source folder and metadata auto-detection.
- Cross-platform toggle.
- All metadata fields.
- Dependency multi-select from installed mods.
- Option-group editor: add/delete groups and options, exclusive/multiple, required, defaults, folder assignment.
- Save-to-archive action with default sanitized filename.

**Log**

- Scrollable log view with timestamps/levels; progress line integrated with busy state.

**Global workflows**

- First-run introduction.
- Update check, changelog, download-and-restart.
- Metadata prompt for legacy mods.
- Option selection with required gating.
- Error modal with chain details and context.
- Confirmation dialogs for destructive actions.
- Busy/progress overlay.
- Toasts for success/info.
- Profiles manager with rename/duplicate/delete.
- One-click downloads and single-instance IPC.
- File drag-and-drop install.

### 21.2 Egui-specific implementation choices (replaceable)

| Area | Current technique | Portable equivalent |
|---|---|---|
| Rendering | Immediate mode; rebuild UI every frame | Any retained or immediate UI |
| State | Plain fields on `App`, mutated by a message reducer | MVU store, view models, Redux, signals |
| Events | Unbounded channel, one message per frame | Command queue/dispatcher |
| Threads | `thread::spawn` + message result | Async tasks, worker pool, job queue |
| “Modals” | `egui::Window` plus global `modal_open` disabling | True modal dialogs or overlay stack |
| Docking | `egui_dock::DockState<Tabs>` serialized to JSON | Fixed panes, splitter layout, tab framework |
| Lists | `egui_extras::TableBuilder` | Table/grid/list view |
| Context menu | `Response::context_menu` per cell | Native or custom context menu |
| Tooltips | `on_hover_text` | Native tooltips/help text |
| Drag reorder | Manual index tracking + pointer release | Framework drag-and-drop model |
| Long lists | Table row virtualization | Virtualized list |
| Manifest tree | Recursive `CollapsingHeader` | Tree view |
| Markdown | `egui_commonmark` | Markdown renderer |
| Log view | `egui_logger::logger_ui` | Log widget/table |
| Icons | SVG → egui texture handles | Icon font, SVGs, native images |
| Fonts | egui `FontDefinitions` + system font loader | Platform font stack + CJK fallback |
| Theme | `egui::Style`/`Visuals`, catppuccin/aesthetix | Stylesheet/theme tokens |
| Toasts | `egui_notify` | Notification system |
| File dialogs | `rfd` (blocking, in render code in places) | Native dialogs or in-app picker |
| Window icon/DPI | eframe `ViewportBuilder`, `set_pixels_per_point` | Native window API |
| Persistence | `serde_json` of UI structs | Any frontend-local state store |
| Playback of progress | `Logger` scraping `PROGRESS` log lines | Structured progress events |
| Keyboard | Manual `Key::Arrow*` reads | Command/keybinding system |
| Text editing of smart strings | `TextBuffer` wrapper | Native text binding |
| Format strings | `strfmt` | ICU/printf/`format!` |

### 21.3 Egui-specific quirks that leak into behaviour

These are not requirements; they are causes of current oddities:

- `OnceLock` column widths persist for the process and are shared across all table instances.
- Selection and keyboard handling depend on egui's internal focused-widget memory.
- `Id`/`id_salt` values determine whether collapsing headers, scrolling, and temp caches collide.
- Temp-string caches are used to edit `smartstring::String` and parsed values without committing invalid text.
- Caches live in egui context data (`md_cache`, `md_cache_changelog`), so they die with the context.
- Texture caches are process-global and never evicted; a rebuilt mod with the same hash keeps its old preview.
- `egui::Window` is not truly modal; the entire UI is disabled manually, and profile manager/pending intentionally escape that rule.
- Dock tab close records the root node, while menu-based hide records the actual parent, so re-open placement can be wrong. [GAP]
- One-message-per-frame processing and per-action follow-up messages make multi-step updates visibly incremental.
- Drawing order (dialogs before dock, busy/toasts after) determines stacking.

---

### 21.4 Capabilities that do not exist

These are not parity requirements, but are easy to assume from the UI:

- No online mod catalog/browser, ratings, screenshots, or in-app search of GameBanana; the only network install is a one-click URL or a downloaded file.
- No conflict/overwrite viewer, no dependency auto-install, no dependency-aware sorting. Missing masters are only warned about.
- No per-mod file selection, patch editing, or metadata editing after installation.
- No undo/redo beyond Cancel Pending (which reloads committed state).
- No job cancellation, install-queue management, progress percentage, or ETA.
- No “update all mods” check; Update is one selected mod at a time from a local archive.
- No profile backup/export/import, no settings export/reset-to-defaults (Reset only discards the draft).
- No simultaneous multi-profile view; only the current profile is loaded into the central model.
- No category/author/URL search or category filter; filter is name substring only.
- No notification history, dock badges, or persistent error list.
- No theme editor; themes are fixed presets.
- No update-preference control, despite the core setting existing.
- No accessibility layer, high-contrast mode, scalable density, or keyboard command layer.
- No plugin/extension mechanism and no embedded CLI console.

---

## 22. Defects, Inconsistencies, and Traps to Decide On

A new frontend should either fix these or deliberately reproduce them. Items marked **danger** affect correctness.

1. **Danger: index-space mismatch.** The table is rendered from `displayed_mods` (sorted/filtered), but selection, deselect, shift-range, and drag messages carry indices that the reducer applies to `mods` (load order). With a non-Priority sort or an active filter, clicking row *n* can select a different mod, and the Priority column displays display order rather than load order. [GAP]
2. **Danger: profile rename has no validation.** A typed name containing separators or `..` is joined onto `profiles_dir()`. Empty names, `.`, and `..` are accepted by the new-profile flow too. [GAP]
3. **Danger: profile deletion deletes unreferenced archives.** `CleanProfile` removes files from the deleted profile that no remaining profile references, assuming the file was unique to it. It uses `remove_file`, so directory-based mods are not removed and the failure is swallowed. [GAP]
4. **Danger: `dirty()` falls back to the first profile's manifest** if the current profile key is missing, and panics if the map is empty. New profiles are not added to the map.
5. **Danger: settings Save consumes `CONFIG` entries before all conversions succeed**; a later failure leaves drafts half-applied.
6. **Danger: `UpdateOptions` writes options to the core immediately but does not save the profile.** Cancel/Apply semantics differ from enable/reorder, which are purely in-memory.
7. **Danger: endianness is inferred from the substring `content`** in a path.
8. **Danger: only one queued message is processed per frame**, allowing backlogs and visible lag during batch installs.
9. **Danger: all installed mods are loaded and hashed at startup**; a bad archive prevents the GUI from opening (`Manager::init().unwrap()` and profile validation).
10. **`ModUpdate`, `DevUpdate`, and `Extract` operate on the selection, not necessarily the clicked mod**; Update replaces the first selected mod.
11. **`SelectOnly` reduces a multi-selection rather than toggling a selected row off**, so plain click cannot deselect the last row.
12. **Category column width is a global `OnceLock`**, so the first measurement in the process fixes it.
13. **Version sort is lexical.** A semver-aware sort would change order.
14. **`show_changelog` is unused; `check_updates` has no control; no UI for update preference.**
15. **Update check blocks first paint** and is skipped entirely on first run/config-less launch.
16. **Emulator launch blocks the UI for one second**, runs in a render closure, and can panic on malformed command strings.
17. **Error modal has two Copy buttons**; one is a tooltip-only prop.
18. **`render_about` and several labels are unlocalized.**
19. **Closed-tab position is usually remembered as the root node**, not the actual former parent.
20. **Changing storage folder does not migrate existing data.**
21. **`ResetMods` clears the current profile's dirty manifest and globally clears the manifest-tree and markdown caches.** It does not touch other profiles' dirty manifests (they keep any partial AddToProfile entries).
22. **`PackageMod` default destination name is sanitized from the name but the user can save anywhere; the `.zip` extension is only suggested.**
23. **Package version editing is fragile:** the edit buffer is rebuilt from the model each frame, and only a fully valid semver commits, so intermediate keystrokes can be lost.
24. **Malformed/deleted profile YAML files are silently omitted from the profile manager** but still appear in the combo (combo uses directory listing).
25. **Profile combo values come from the filesystem each frame**; there is no cached/validated profile model.
26. **`SetDownloading` sets busy without a corresponding task**, relying on later IPC messages to clear it.
27. **One-click starts with no visible progress until `Starting` arrives**, and errors during a pre-existing instance exit are only sent over IPC.
28. **Busy has no cancellation**, except that some dialogs close and send `Noop` on cancellation (dev update/extract).
29. **No thumbnail cache invalidation on mod update.**
30. **Mod list has no multi-column sorting, grouping, or search beyond name substring.**
31. **“Send to profile” never saves the target profile**; the core in-memory addition survives only until the next reload, unless the user later switches to that profile and Applies. [GAP]
32. **Dev Update replaces the mod in the core but never saves the profile**, so the repack is lost on reload unless a later Apply saves it. [GAP]
33. **Deleting or renaming the active profile does not update the core's current profile or the settings' stored profile name.** The UI can keep using an in-memory profile whose directory no longer exists. [GAP]
34. **Update on an empty selection panics** (`selected.first().unwrap()`); right-clicking does not first select the clicked row, and settings actions can deliberately clear the selection. [GAP]
35. **Cancel/Close semantics are not uniform:** Apply/Cancel in Pending, the file-picker dialogs, and the option picker behave differently from settings Reset and from the profile manager's immediate filesystem operations.
36. **The install queue is invisible.** There is no list of queued files, no per-file status, and no way to remove a queued item; the only feedback is the busy overlay and the final aggregated error.
37. **A failed output wipe is silently ignored.** Settings still save without resetting pending state, potentially leaving a stale merged output/deployment pointing at the old paths.
38. **The mod-update install path does not save the profile.** It replaces in memory and returns a manifest; persistence happens only when a later Apply saves, so quitting or reloading can lose the replacement.
39. **Switching profiles does not reconcile `selected`.** The mod list reloads, but selected entries from the previous profile remain; the Info tab can show a mod that is not installed in the active profile, and context actions can target it.

---

## 23. Suggested Application-Layer Contract Derived From This GUI

This is not existing code; it is the minimum shape implied by the behaviours above, aligned with the project plan.

### 23.1 State belonging in `uk-app` (not the frontend)

- Current profile and profile list.
- Working mod entries in load order: id, metadata, enabled, enabled options, path.
- Per-profile pending manifest, and staged edits.
- Settings draft for both platforms, plus validation and the wipe/reset rule.
- Install batch queue and replacement target.
- One mutating job at a time, with progress.
- Profile file operations with a validated `ProfileName`.
- Cemu/BCML import, self-update, one-click/IPC, URL-handler registration.
- Error/event queue for batch operations.

### 23.2 Data the frontend may own

- Selection, drag, filter text, sort mode/direction, scroll position.
- Dock layout, theme, theme-specific decoration.
- Modal stack, confirmations, toasts.
- Preview textures, markdown caches, collapsed-tree state.
- UI language preview and frontend-local persistence.
- Keyboard/focus state.

### 23.3 Commands implied

```text
Open(paths)
InstallPrepared(candidate)            // only constructible after options resolve
SetEnabled(ids, bool)
SetOptions(id, picks)
MoveTo(ids, index)
Apply
CancelPending
Deploy
ResetPending
Remerge
SwitchProfile(ProfileName)
CreateProfile(ProfileName)
DuplicateProfile(ProfileName)
RenameProfile(ProfileName, ProfileName)
DeleteProfile(ProfileName)
AddToProfile(ids, ProfileName)
Uninstall(ids)
UpdateModFromPath(id, path)
DevUpdateFromPath(ids, folders)
ExtractTo(ids, directory)
PackageTo(source, dest, meta)
SaveSettings(SettingsDraft)
ImportCemu(directory)
MigrateBcml
RegisterUrlHandler
CheckForUpdates
DownloadUpdate(release)
LaunchEmulator
OpenFolder(kind or path)
```

### 23.4 Inspection/event results implied

```text
NeedsMetadata { path, suggested_name }
NeedsOptions(OptionsPrompt)
WrongPlatform { mod_is, current }
MissingMasters(Vec<(name, version)>)
Ready(InstallCandidate)
Installed / Updated / Removed
DirtyChanged(manifest / count)
JobStarted / Progress / JobFinished
SettingsSaved { output_wiped, pending_reset }
UpdateAvailable / Changelog
Error(AppError)
```

### 23.5 Frontend parity smoke test

A replacement frontend is at parity when it can:

1. Start with no config, show the intro, configure both platforms, and restart into the normal UI.
2. Install a `.zip`, a legacy `.zip`/`.rar`/`.7z` with `rules.txt`, a `.bnp`, a directory mod, and a mod with missing metadata (metadata prompt).
3. Reject a wrong-platform mod, warn about a missing master, and gate required options.
4. Enable/disable, change options, reorder, filter, sort, multi-select, move to start/end, and Apply; verify the resulting `profile.yml` and incremental merge.
5. Cancel pending changes and verify the working state reverts.
6. Uninstall, extract, dev-update, and update a mod, verifying archive/store effects.
7. Create, rename, duplicate, delete, and switch profiles, including cleanup of unused archives.
8. Save settings that change output/method/layout and verify wipe + pending reset; reset drafts without saving.
9. Import Cemu and BCML configurations on fixtures.
10. Package a mod with dependencies and option groups, then reinstall the package.
11. Trigger a fake update and one-click URL; verify download, restart, single-instance forwarding, and busy/progress.
12. Show every log, error, confirm, busy, toast, and changelog state described above.
13. Under a Name sort with an active filter, drag a row onto another row and verify the *clicked mod* moves to that mod's load-order position (not a display index), and that the Priority column shows load order.
14. Switch profiles and verify the stale selection is cleared; update a selected mod and verify the selection follows the new mod id.

> Status on `gui2` (2026-11-19): the replacement frontend implements items
> 1–14, but the checklist has not been run manually yet. The `UKMM_TEST_DUMP`
> end-to-end scenario is likewise written but pending a real game dump
> (setup per `README.md`).

### 23.6 Deliberate deviations in the session frontend (`gui2` branch)

The `uk_app::session` frontend intentionally changes three behaviours of the
old reducer; they are fixes, not regressions:

- **Deploy keeps the pending manifest.** The old `Deploy` ran
  `ResetMods(None)`, clearing the current profile's dirty manifest — but
  `deploy()` only copies the merged output, it does not merge, so the staged
  files were silently lost from tracking. The session's `Deploy` leaves
  pending untouched; `CancelPending` is the only way to drop staged files.
- **Descending sorts are stable.** Ties now keep load order instead of
  shuffling (the old `sort_by` was unstable). Priority-descending therefore
  still reads as reverse load order.
- **Renaming the active profile heals the core's profile map.** The old code
  left the core's `current_profile` pointing at the old name, so the next
  `profile()` call hit the "Invalid profile" panic path. The session teaches
  the core the new name (`add_profile`) and switches the core over.
- **The startup update check runs as a background session job.** It answers
  with the usual events but never sets `Snapshot::busy`, so the UI stays
  interactive while GitHub answers (a partial step toward moving startup
  work into a job; `Manager::init` at startup is still a phase-4 item).
  HTTP calls now carry an overall timeout (10 s for API calls, 5 min for
  downloads).

### 23.7 Progress notes on `gui2` (2026-11-19)

- The phase-3 review outcome (findings and their resolutions, accepted
  trade-offs) is recorded in `GUI_REFACTOR_PHASE3_PLAN.md` §10.
- Known accepted trade-off: `render_mod_row` clones the full `ModEntry` per
  row per frame plus two O(n) scans per row (load-index lookup, selection
  check). Accepted as fine at current mod counts; revisit only if large
  profiles become slow.

---

## Appendix A. User-Visible String Inventory (English keys)

The canonical values are in `crates/uk-localization/localization/en.json`. Keys by surface:

- **Shell/tabs:** `Menu_File`, `Menu_File_Open`, `Menu_Tools`, `Menu_Tools_RefreshMerge`, `Menu_Tools_ResetPending`, `Menu_Tools_ConfigFolder`, `Menu_Tools_StorageFolder`, `Menu_Tools_DeployFolder`, `Menu_Window`, `Menu_Window_Reset`, `Menu_Help`, `Menu_Help_About`, `Menu_Help_About_GUI`, `Tab_Info`, `Tab_Install`, `Tab_Deploy`, `Tab_Mods`, `Tab_Log`, `Tab_Settings`, `Tab_Package`.
- **Generic:** `Generic_OK`, `Generic_Close`, `Generic_Cancel`, `Generic_Confirm`, `Generic_Save`, `Generic_Reset`, `Generic_Delete`, `Generic_Apply`, `Generic_Update`, `Generic_Exit`, `Generic_Copy`, `Generic_Copied`, `Generic_Yes`, `Generic_No`, `Generic_MarkdownSupported`.
- **Mods/profiles:** `Mod_Filter`, `Mod_Select_Title`, `Mod_Selected_None`, `Mod_Send`, `Mod_Update_Folder`, `Mod_Dev_Update`, `Mod_Uninstall`, `Mod_Uninstall_Confirmation`, `Mod_Enable`, `Mod_Disable`, `Mod_View`, `Mod_Extract`, `Mod_Move_Start`, `Mod_Move_End`, `Mod_Unpack_Folder`, `Profile_Select`, `Profile_New`, `Profile_New_Label`, `Profile_Manage`, `Profile_Label`, `Profile_ActiveMods_1/2/3`, `Profile_Added`, `Profile_Delete_Confirmation`, `Profile_Duplicate`, `Profile_Rename`, `Profile_NoMods`, `Pending_Changes`, `Pending_Files`, `Helper_Modlist_NoMods`, `Helper_Modlist_NoMods_Hint`, `Helper_Modlist_NoConfig`, `Helper_Modlist_NoConfig_Hint`.
- **Info:** `Info_Name`, `Info_Version`, `Info_Category`, `Info_Author`, `Info_Description`, `Info_Options`, `Info_Options_None`, `Info_Manifest`, `Info_Manifest_BaseFiles`, `Info_Manifest_DLCFiles`, `Info_Provide_Label`, `Info_Provide_Message`, `Info_URL`, `Info_Priority`.
- **Install:** `FilePicker_Up`, `FilePicker_Back`, `FilePicker_Refresh`.
- **Deploy:** `Deploy_OpenEmu`, `Deploy_NoConfig`, `Deploy_Auto_Failed`.
- **Settings:** `Settings_General`, `Settings_Migrate`, `Settings_OneClick`, `Settings_OneClick_Desc`, `Settings_Theme`, `Settings_Theme_Desc`, `Settings_Language`, `Settings_Language_Desc`, `Settings_Mode`, `Settings_Mode_Desc`, `Settings_Mode_WiiU`, `Settings_Mode_Switch`, `Settings_Storage`, `Settings_Storage_Desc`, `Settings_Sys7z`, `Settings_Sys7z_Desc`, `Settings_Changelog`, `Settings_Changelog_Desc`, `Settings_Config_WiiU`, `Settings_Config_NX`, `Settings_Config_WiiU_ImportCemu`, `Settings_SelectFolder_Cemu`, `Settings_Saved`, `Settings_Platform_Language`, `Settings_Platform_Language_Desc`, `Settings_Platform_Dump`, `Settings_Platform_Dump_Type`, `Settings_Platform_Dump_Type_Desc`, `Settings_Platform_Dump_Type_Unpacked`, `Settings_Platform_Dump_Type_WUA`, `Settings_Platform_Dump_WiiU_Base`, `Settings_Platform_Dump_WiiU_Base_Desc`, `Settings_Platform_Dump_Update`, `Settings_Platform_Dump_Update_Desc`, `Settings_Platform_Dump_DLC`, `Settings_Platform_Dump_DLC_WiiU_Desc`, `Settings_Platform_Dump_DLC_NX_Desc`, `Settings_Platform_Dump_WUA`, `Settings_Platform_Dump_WUA_Desc`, `Settings_Platform_Dump_NX_Base`, `Settings_Platform_Dump_NX_Base_Desc`, `Settings_Platform_Deploy`, `Settings_Platform_Deploy_Method`, `Settings_Platform_Deploy_Method_Desc`, `Settings_Platform_Deploy_Method_Copy`, `Settings_Platform_Deploy_Method_HardLink`, `Settings_Platform_Deploy_Method_Symlink`, `Settings_Platform_Deploy_Layout`, `Settings_Platform_Deploy_Layout_WiiU_Desc`, `Settings_Platform_Deploy_Layout_NX_Desc`, `Settings_Platform_Deploy_Layout_WiiU_WithoutName`, `Settings_Platform_Deploy_Layout_WiiU_WithName`, `Settings_Platform_Deploy_Layout_NX_WithoutName`, `Settings_Platform_Deploy_Layout_NX_WithName`, `Settings_Platform_Deploy_Auto`, `Settings_Platform_Deploy_Auto_Desc`, `Settings_Platform_Deploy_Rules`, `Settings_Platform_Deploy_Rules_Desc`, `Settings_Platform_Deploy_Output`, `Settings_Platform_Deploy_Output_Desc`, `Settings_Platform_Deploy_Emu`, `Settings_Platform_Deploy_Emu_Desc`.
- **Package:** `Package_RootFolder`, `Package_CrossPlatform`, `Package_CrossPlatform_Desc`, `Package_Dependencies`, `Package_Finish`, `Package_ManageOptions`, `Package_Save_Title`, `Package_Version_Desc`, `Options_Configure`, `Options_Select`, `Options_Group_Add`, `Options_Group_Name`, `Options_Group_Desc`, `Options_Group_Type`, `Options_Group_Exclusive`, `Options_Group_Multiple`, `Options_Group_Required`, `Options_Group_Required_Desc`, `Options_Group_New`, `Options_Add`, `Options_Name`, `Options_Desc`, `Options_Folder`, `Options_Default`, `Options_Default_Enable`, `Options_None`, `Options_New`, `Options_Required`.
- **Dialogs/updates:** `Error_Label`, `Error_Details`, `Error_Context`, `Busy_Working`, `Busy_Processing`, `Changelog_New`, `Changelog_Subscribe`, `Changelog_Bitcoin`, `Changelog_Bitcoin_Copy`, `Changelog_Bitcoin_Copied`, `Update_Available`, `Intro_Message`, `Generic_Update`.
- **Categories:** `Mod_Category_None`, `Mod_Category_Anim`, `Mod_Category_Balance`, `Mod_Category_Crafting`, `Mod_Category_Customization`, `Mod_Category_Difficulty`, `Mod_Category_Enemies`, `Mod_Category_Expansion`, `Mod_Category_Items`, `Mod_Category_Meme`, `Mod_Category_Other`, `Mod_Category_Overhaul`, `Mod_Category_Overworld`, `Mod_Category_Player`, `Mod_Category_QoL`, `Mod_Category_Quest`, `Mod_Category_Shrine`, `Mod_Category_Texture`.

Unlocalized literals worth carrying over deliberately:

- Product name `U-King Mod Manager`, copyright, `GitHub:`, `egui`.
- `Browse…` in all folder/file pickers.
- `FAILED TO LOAD MANIFEST`.
- `Changes to settings have not been saved`.
- `"An error has occurred. Check the details for more information."`
- Deployment method/layout display names in core (`DeployMethod::name`, `DeployLayout::name`; note `DeployLayout::name` is not used by this GUI).
- All error contexts produced by `anyhow` in tasks/import/update.
- Log messages.

---

## Appendix B. File/Package Format Touchpoints

The GUI surfaces these formats; a frontend must not reinterpret them:

- **UKMM mod archive:** ZIP with `meta.yml`, content under `content/`, DLC under `aoc/`, optional `thumb.{jpg,jpeg,png,svg}`, optional `options/` tree referenced by option paths.
- **BCML `.bnp`:** 7z-based legacy package with `info.json`; converted through the core.
- **Legacy archives/directories:** `rules.txt` metadata; converted through `gfx` conversion.
- **Metadata sources:** `info.json` (BCML), `rules.txt` (legacy), `meta.yml` (UKMM). The package tab prefers them in that order when auto-detecting from the source root.
- **`profile.yml`:** profile mod map + load order.
- **`pending.yml`:** deployment pending log; supports an older `{files, delete}` schema.
- **`settings.yml`, `ui.json`:** as described in §20.
- **`meta.yml`:** `api`, name, version, author, category, description, platform (`Specific(Big/Little)` or `Universal`), optional URL, `option_groups`, `masters` mapping hash → (name, version). Option groups are exclusive (single pick, optional default, required flag) or multiple (set of defaults, required flag); options have name, description, path, requires.
- **Manifest:** sets of `content` and `aoc` relative file paths, used for pending UI and incremental merge.
- **dump sources:** unpacked content/update/aoc directories (endian inferred by the GUI) or a `.wua` archive.

---

## Appendix C. One-Screen Behaviour Summary

A new frontend must present a single window (or equivalent workspace) containing:

- A menu bar: File/Install+Exit, Tools/merge+pending+folders, Window/layout+tabs, Help/docs+About.
- Seven areas: Mods, Info, Install, Deploy, Settings, Package, Log.
- A Mods area with profile switching/management, counts, filter, sortable mod table, multi-select, drag reorder, context actions, and a pending Apply/Cancel window.
- An Info area with thumbnail, metadata, markdown description, option editing, and file manifest.
- An Install area with file selection/browser, batch queue, metadata prompt, and required-option gating.
- A Deploy area showing method/auto/output, opening folders, launching an emulator, manual deploy, and pending warning.
- A Settings area with global options and two independent platform drafts, save validation, output wipe, and reset.
- A Package area with metadata form, dependency chooser, option-group editor, and archive export.
- A Log area with the UKMM log.
- Global error details, confirmations, busy/progress, toasts, About, intro/changelog, update download/restart, Cemu/BCML import, one-click IPC, and URL-handler registration.
- Persistent local UI state (theme, sort, picker location, dock layout) and persistent core state (settings, profiles, mods, pending deployment).
