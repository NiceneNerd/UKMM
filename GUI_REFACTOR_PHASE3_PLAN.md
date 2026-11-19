# UKMM refactor — detailed plan for phase 3 (move the staged state)

> Working plan for phase 3, kept in the repo on branch `gui2` (copied from the
> planning directory 2026-11-19). Phase 3 has since **landed** on `gui2`
> (`fafc18e`, `0c9ed3a`, `b58a9c6`, `8d68071`) and was reviewed — see §10 for
> the review outcome. This file is the historical spec; the sections below
> describe the plan as approved, with `ad73c75` as its ground truth.

Companion to `GUI_REFACTOR.md` (repo root) and `GUI_REF.md` (parity spec).
Scope: branch `gui2`, HEAD `ad73c75` — phases 1 and 2 are landed, the working tree is clean.
Phase 3 is "the heart of the refactor": `App.mods`, `App.dirty`, `install_queue`,
`error_queue`, `update_mod`, and `busy` leave the egui frontend and become
`uk_app::session` state; the frontend keeps only view state and talks to the
Session through `Command` → `Snapshot` + `Event`.

---

## 0. Ground truth at `ad73c75`

### 0.1 State that must move

`App` (`src/gui.rs:254-289`), fields the Session takes over:

| Field | Type | Notes |
|---|---|---|
| `mods` | `Vec<Mod>` | working copy / load order (shadow of `profile.yml`) |
| `dirty` | `RwLock<HashMap<String, Manifest>>` | per-profile pending manifests |
| `install_queue` | `VecDeque<PathBuf>` | batch install queue |
| `error_queue` | `VecDeque<anyhow_ext::Error>` | per-file install errors, aggregated at the end |
| `update_mod` | `Option<Mod>` | replacement target for the *next* install |
| `busy` | `Cell<bool>` | one-mutating-job flag |

Derived today, becomes a pure helper over the snapshot: `displayed_mods`
(`gui.rs:258`), `filtered_mods` (`gui.rs:510-521`), `Sort::orderer`
(`gui.rs:126-153`).

Audit result: every direct mutation of these six fields happens inside
`src/gui/update.rs` — the other GUI modules only read them or dispatch
messages. The move is therefore well-contained (the risk is in the reducer's
semantics, not in scattered writes).

Stays in the frontend (per `GUI_REFACTOR.md` §2.4): selection, drag state,
filter text, `list_state` (sort), `picker_state`, `profiles_state`, modal stack
(`error`, `confirm`, `new_profile`, `show_about`, `changelog`, `options_mod`,
`meta_input`, `opt_folders`, `show_package_deps`), `toasts`, `theme`, `tree`,
`closed_tabs`, `focused`, `package_builder`, `temp_settings` (phase 4),
`new_version`.

### 0.2 `Message` today

74 variants (`src/gui.rs:156-231`). Grouped by destination:

- **Frontend-only (renamed `UiMsg`)**: sort/filter (`ChangeSort`), selection
  (`SelectOnly`, `SelectAlso`, `SelectThrough`, `Deselect`, `ClearSelect`),
  drag (`StartDrag`, `ClearDrag`), picker (`FilePickerUp/Back/Set`,
  `SelectFile`, `SetFocus`), modals (`CloseError`, `CloseConfirm`,
  `ShowAbout`, `CloseAbout`, `CloseProfiles`, `Confirm`, `NewProfile`,
  `ShowPackagingOptions`, `ShowPackagingDependencies`,
  `ClosePackagingOptions`, `ClosePackagingDependencies`, `CloseChangelog`,
  `SelectProfileManage`, `SetTheme`, `SetLanguage`, `Toast`), and the
  `RequestMeta`/`RequestOptions` modal openers (their *causes* become events).
- **Operations (become `Command`)**: `OpenMod`, `InstallMod`, `AddMod`,
  `UninstallMods`, `ModUpdate`, `DevUpdate`, `Extract`, `ToggleMods`,
  `MoveSelected`, `UpdateOptions`, `AddToProfile`, `Apply`, `Deploy`,
  `Remerge`, `ResetPending`, profile CRUD (`ChangeProfile`, `AddProfile`,
  `DeleteProfile`, `DuplicateProfile`, `RenameProfile`, `CleanProfile`),
  `SaveSettings`, `ResetSettings`, `ImportCemu`, `MigrateBcml`, `PackageMod`,
  `CheckMeta`, `DoUpdate`, `Restart`.
- **Results (become `Event`)**: `Noop`, `ResetMods`, `Error`, `HandleMod`,
  `RemoveMods`, `ResetPacker`, `HandleSettings`, `OfferUpdate`,
  `SetChangelog`, `SetDownloading`, `UpdatePackageMeta`, `ReloadProfiles`,
  `RefreshModsDisplay` (derived), `CheckMeta`'s result.
- **`Confirm(Box<Message>, String)`** becomes `UiMsg::Confirm(Box<Command>, String)`
  followed by `Command` dispatch on OK.

### 0.3 Reducer facts (`src/gui/update.rs`)

- **One message per frame**: `if let Ok(msg) = self.channel.1.try_recv()` at
  line 12, `else { self.handle_drops(ctx) }` at 752-754. Batch installs lag
  (`GUI_REF.md` §22.8).
- `dirty()`/`dirty_mut()` (`gui.rs:425-444`) fall back to *the first*
  profile's manifest and `.unwrap()` — panics when the map is empty; new
  profiles never get an entry (§22.4). `dirty_mut` uses a raw-pointer
  `unsafe` Polonius hack (line 438-441).
- `SelectOnly`/`SelectAlso`/`Deselect`/`StartDrag` index `self.mods` (load
  order) but the GUI passes `displayed_mods` indices: `render_mod_row(index)`
  receives `filtered[row.index()]` where `filtered` are display indices
  (`gui/mods.rs:232-239`), then emits `SelectOnly(index)` etc. (455-465) and
  `MoveSelected(dest_index)` from `hover_index` (282-288, 467-468). The
  Priority column also prints the display index (`mods.rs:408`). This is
  §22.1.
- `HandleMod` does platform/masters checks in the reducer and *then* installs
  or opens the options modal (278-312). Checks are advisory: missing masters
  produce a modal error but the install continues into the install branch
  (the `if/else if` at 292-311 only gates platform-vs-options).
- `InstallMod` (313-336) is the update-or-add worker: `replace(tmp, old_hash)`
  or `add()` + `set_enabled_options` + `save()`; the `replace` path returns
  `ResetMods(Some(dirty))` and **does not save** (§22.38). Add path uses
  `unsafe { get_mod(hash).unwrap_unchecked() }` (332).
- `AddToProfile` (458-487) mutates the *target* profile's dirty map directly
  and never saves the target profile (§22.31).
- `DevUpdate` (365-401) picks folders in the reducer (rfd) and the worker
  calls `uk_app::dev_update_mods` — which *does* save (phase 1), but the
  in-memory mod list is only refreshed through `ResetMods` (§22.32).
- `UpdateOptions` (595-618) calls `set_enabled_options` on the core
  immediately but does not save (§22.6); `ToggleMods` (402-422) is purely
  in-memory; `MoveSelected` (127-142) extends dirty from the moved mods'
  manifests.
- `Apply` (499-506) `mem::take`s the current profile's dirty manifest and
  calls `uk_app::apply_changes(core, mods, Some(dirty))`.
- `SetDownloading` (240-244) sets `busy` with no task (§22.26); only IPC
  clears it.
- `ModUpdate` (350-364) uses `self.selected.first().unwrap()` (§22.34) and
  sets `update_mod`, then re-enters the open pipeline.
- `SaveSettings` (533-574) wipes output, saves, `core.reload()`s (which
  re-creates mod manager + deploy manager, `core.rs:39-46`), clears
  selection, and resets pending (§22.5 is the half-applied draft case).
- `do_task` (464-497) sets `busy`, spawns a thread, `catch_unwind`s,
  emits `Message`, clears the dump cache, sends on the flume channel.
- `handle_drops` (499-508) enqueues dropped paths and starts the queue.
- `modal_open()` (447-458) gates drops and keyboard nav.

### 0.4 Identity and manager semantics

- `Mod: PartialEq + Hash` compare **only the `hash` field** (`mods.rs:51-61`);
  `state_eq` compares `enabled` + `enabled_options` (96-98); `hash()` and
  `as_map_id()` both return that `usize` (101-103, 127-136). So a `ModId`
  newtype over `usize` is safe for selection/drag.
- `mods::Manager::profile()` returns `MappedRef<'_, String, Profile>` and
  **panics** ("Invalid profile") if `current_profile` is missing
  (`mods.rs:293-299`); `get_profile` has the same `expect`.
- `core.change_profile(p)` sets `current_profile`, calls `create_profile_if`
  (creates the dir, inserts a default `Profile`, saves — so switching to a
  nonexistent name silently creates it, `mods.rs:329-344`) and updates
  `settings.platform_config_mut().profile`. It does **not** re-create the
  manager.
- `core.reload()` re-reads settings from disk and re-inits mods + deploy
  managers (`core.rs:39-46`) — must be owned by the Session so events and
  revisions stay consistent (`GUI_REFACTOR.md` §7).
- API the Session will call: `all_mods() -> ModIterator`, `get_mod(hash) ->
  Option<Mod>`, `set_enabled(LookupMod, bool, Option<&String>) ->
  Result<Arc<Manifest>>`, `set_enabled_options(LookupMod, Vec<ModOption>) ->
  Result<Arc<Manifest>>`, `set_order(Vec<usize>)`, `save() -> Result<()>`,
  `add(&Path, Option<&String>) -> Result<Mod>`, `del(LookupMod,
  Option<&String>) -> Result<Arc<Manifest>>`, `replace(Mod, old_hash) ->
  Result<Mod>`, `add_profile(String)`.
- `Mod::manifest_with_options` has a process-wide `MANIFEST_CACHE` keyed by
  `(hash, option paths)` (`mods.rs:81-94`) — a session must invalidate by
  hash change, not by name.

### 0.5 §22 defects phase 3 owns

Fix (with regression tests): **1** (index-space), **4** (`dirty()` fallback /
panic), **6** (options save), **8** (one message per frame), **10**
(update/dev-update act on selection, not clicked mod), **26** (`busy`
without a job), **31** (AddToProfile never saves), **32** (DevUpdate never
saves), **34** (empty-selection `unwrap` panic), **36** (invisible install
queue), **38** (mod-update never saves), **39** (stale selection on profile
switch). Partially: **21** (ResetMods clears global caches — events can carry
the right scope), **28** (no job cancellation; keep out of scope but model
`CancelPending` correctly).

Phase 4 keeps: 2, 3, 5, 7, 9, 16, 20, 29, 37. Deliberate reproductions /
out of scope for 3: 11, 12, 13, 14, 17, 18, 19, 22, 23, 24, 25, 27, 30, 33,
35.

---

## 1. Target types (`crates/uk-app/src/session/`)

```rust
// identity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModId(pub usize);        // = Mod::hash();
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

// working-state view
pub struct ModEntry {
    pub id: ModId,
    pub meta: Meta,
    pub enabled: bool,
    pub options: Vec<ModOption>,
    pub path: PathBuf,
}
pub struct PendingSummary { pub files: usize }          // current profile
pub struct Snapshot {
    pub revision: u64,
    pub profile: ProfileName,
    pub entries: Vec<ModEntry>,     // vec order IS load order
    pub pending: PendingSummary,
    pub busy: Option<JobId>,
    pub progress: Option<String>,   // last Progress text for the busy label
    pub installing: usize,          // queue depth for the busy label (#36)
}

// input that needs the user, returned as data
pub enum Inspection {
    Ready(InstallCandidate),
    NeedsOptions(OptionsPrompt),
    NeedsMeta { path: PathBuf, suggested_name: String },
    WrongPlatform { mod_is: String, current: String },
    MissingMasters(Vec<MasterRef>),
}
pub struct Picks(pub Vec<ModOption>);
pub struct OptionsPrompt { /* private groups + defaults */ }
impl OptionsPrompt {
    pub fn groups(&self) -> &[OptionGroup];
    pub fn defaults(&self) -> &Picks;
    pub fn resolve(self, picks: Picks) -> Option<InstallCandidate>; // required_satisfied
}
pub struct InstallCandidate { /* private: staged Mod + resolved options */ }

// commands
pub enum Command {
    Open { path: PathBuf, meta: Option<Meta> },          // classification + inspection
    Install(InstallCandidate),
    InstallBatch { paths: Vec<PathBuf>, replace: Option<ModId> },
    SetEnabled { ids: Vec<ModId>, enabled: bool },
    SetOptions { id: ModId, picks: Picks },
    MoveTo { ids: Vec<ModId>, index: usize },             // load-order index
    Apply, CancelPending, Deploy, Remerge, ResetPending,
    Uninstall(Vec<ModId>),
    AddToProfile { ids: Vec<ModId>, profile: ProfileName },
    SwitchProfile(ProfileName), CreateProfile(ProfileName),
    DuplicateProfile(ProfileName),
    RenameProfile { from: ProfileName, to: ProfileName },
    DeleteProfile(ProfileName),
    UpdateFromPath { id: ModId, path: PathBuf },
    DevUpdate { ids: Vec<ModId>, folders: Vec<PathBuf> }, // same order
    Extract { ids: Vec<ModId>, dest: PathBuf },
    CheckForUpdates, InstallUpdate(VersionInfo), RegisterUrlHandler,
    ImportCemu(PathBuf), MigrateBcml,
    Package { source: PathBuf, dest: PathBuf, meta: Meta },
    // grows per phase; phase 4 adds: SaveSettings(SettingsDraft),
    // ResetSettings, LaunchEmulator, OpenFolder(FolderKind), Restart,
    // ParseMeta(PathBuf)
}

// output
pub enum Event {
    Revision(u64),                       // snapshot changed; re-read it
    Inspection(Inspection),              // answer with a follow-up Command
    Installed { id: ModId },
    Updated { id: ModId },
    Removed { ids: Vec<ModId> },
    PendingChanged { profile: ProfileName, files: usize },
    ProfilesChanged,
    JobStarted(JobId),
    Progress { job: JobId, message: String },
    JobDone { job: JobId, result: Result<(), AppError> },
    Error(AppError),
}
```

Serde on all of `Command`, `Event`, `Snapshot`, `ModEntry` from day one
(locked decision; `AppError::Other` stringifies for the wire, stays typed
in-process).

`AppError` gains `Unsupported(String)` (only if phase-4 commands are declared
early) — see §8.3 (locked: enum grows per phase, so no `Unsupported`
variant is needed).

---

## 2. Session mechanics

```rust
pub struct Session { /* Arc<Manager>, command Sender, event Receiver, snapshot */ }
impl Session {
    pub fn open(core: Arc<Manager>) -> Result<Session>;
    pub fn snapshot(&self) -> Arc<Snapshot>;          // lock-free read (arc-swap or RwLock<Arc<_>>)
    pub fn subscribe(&self) -> Receiver<Event>;       // flume is multi-consumer
    pub fn dispatch(&self, cmd: Command) -> JobId;
}
```

- **One worker thread, one mutating job at a time.** `dispatch` allocates a
  `JobId` (`AtomicU64`) and enqueues `(JobId, Command)`. The worker owns the
  mutable `State` (`entries: Vec<ModEntry>` in load order, `dirty:
  HashMap<ProfileName, Manifest>`, install queue, replacement target) and
  the `Arc<Manager>`.
- **Every job**: `JobStarted` → handler → snapshot rebuild + revision bump →
  `JobDone { result }` (+ `Error` for user-facing failures). Handler panics
  are `catch_unwind`ed into `JobDone(Err(AppError::Other(..)))` with a
  backtrace (never a dead UI).
- **Snapshot publication**: worker writes a new `Arc<Snapshot>` into a
  `RwLock<Arc<Snapshot>>`/`ArcSwap` after each mutation and emits
  `Event::Revision(n)`. Read-only queries (CLI `--json`, frontends) never
  dispatch and never block.
- **Dump cache**: keep `do_task`'s post-job `dump.clear_cache()` inside the
  worker's job wrapper.
- **Event draining**: the frontend loops `while let Ok(ev) = rx.try_recv()`
  per frame — fixes §22.8. `ctx.request_repaint()` only while
  `snapshot.busy.is_some()` or events remain.
- **Busy/progress (locked)**: `busy` lives in the snapshot as
  `Option<JobId>` and the worker records the latest `Progress` message into
  `snapshot.progress`; the frontend renders the busy overlay straight from
  the snapshot. No frontend-side busy flag.
- **Pending-dirty invariants** (fixes §22.4):
  - `Session::open` seeds `dirty` with an entry for **every** profile from
    `settings().profiles()` and for the current profile.
  - `profiles.get(profile)` failures are impossible by construction; no
    fallback-to-first, no `unwrap`.
  - New/duplicated/renamed profiles insert/remove map entries in the same
    job.
- **Working-state refresh**: after install/uninstall/profile/reload jobs,
  re-read `all_mods()` into `entries` (today's `ResetMods` path). For
  enable/options/order, mutate `entries` and call the corresponding manager
  setter, so the shadow and the core cannot diverge.
- **Save semantics (locked)**: `SetEnabled`, `MoveTo`, `SetOptions`,
  `AddToProfile`, `DevUpdate`, `UpdateFromPath`, `Uninstall` all call
  `mod_manager.save()` (on the affected profile) after mutating, so none of
  the §22.6/31/32/38 "lost until Apply" cases can happen. `Apply`/`Deploy`
  remain the only merge operations; pending manifests still track the
  files for the incremental merge.
- **Profile switch**: `SwitchProfile` calls `core.change_profile`, refreshes
  entries, ensures that profile's dirty entry exists, emits
  `ProfilesChanged` + `Revision`. Frontend **clears** selection on profile
  switch (locked decision; fixes §22.39).
- **CancelPending** = discard the current profile's manifest and re-read
  `all_mods()` (today's `ResetMods(None)` behaviour).
- **Apply** = `mem::take` the current manifest → `apply_changes(core,
  entries_as_mods, Some(dirty))` → on success `PendingChanged { files: 0 }`.
  On failure the manifest is restored (today's `mem::take` loses it on
  error — keep the take, but re-stage on `Err`).
- **One-click / IPC**: stays a frontend bridge (`IpcMessage` → `Command::
  Open` or a `JobStarted`/`Progress` pair for download state), but
  `SetDownloading` becomes an actual job so `busy` always has a `JobId`
  (§22.26).

---

## 3. Pure helpers in `uk-app` (frontend derives its view)

Extract from the GUI, all headless-tested:

| Helper | Replaces | Fixes |
|---|---|---|
| `sort_entries(entries: &[ModEntry], sort: Sort, desc: bool) -> Vec<ModId>` | `Sort::orderer` + `ChangeSort` | §22.1, 13 (keep lexical version sort in 3) |
| `filter_entries(ids: &[ModId], entries: &[ModEntry], filter: &str) -> Vec<ModId>` | `filtered_mods` | §22.30 (name substring preserved) |
| `load_index_of(entries, id) -> Option<usize>` | `mods.iter().position` | §22.1 |
| `move_ids(entries: &mut Vec<ModEntry>, ids: &[ModId], dest) -> Result<()>` | `move_selected(&mut self.mods, &self.selected, dest)` | §22.1 |
| `required_satisfied` (exists) | option gating | §22.6 basis |

`Sort` (the enum) moves to `uk-app` (data-only, serde) and is re-exported;
the frontend still persists it in `ui.json` via `ModListState` (locked
decision).

**Index rule (the core fix):** display indices never leave the frontend.
Rows map `row.index()` → `ModId` via the visible list. Selection, drag,
context menus, and `MoveTo` all carry `ModId`s. `MoveTo.index` is always a
**load-order** index, computed as `load_index_of(hovered_id)` (or 0 /
`entries.len()` for Move-to-Start/End; `move_ids` clamps).

**Context-menu rule (locked):** a context-menu action targets the clicked
mod — unless the clicked mod is part of a multi-selection, in which case it
targets the whole selection (`ids = selected if clicked ∈ selected else
[clicked]`). This fixes §22.10 while preserving multi-mod update/extract/
uninstall. The empty-selection panic (§22.34) disappears because every
command carries explicit ids and the frontend checks for empty before
dispatching.

---

## 4. Message split (mapping table)

| Current `Message` | Becomes |
|---|---|
| `SelectOnly/Also/Through/Deselect/ClearSelect`, `StartDrag`, `ClearDrag`, `ChangeSort`, `SetFocus`, picker/modal/toast/theme/language variants, `Confirm` | `UiMsg` (frontend) |
| `Confirm(msg, text)` | `UiMsg::Confirm(Box<Command>, String)` |
| `OpenMod`, `InstallMod`, `AddMod`, `UninstallMods`, `ModUpdate`, `DevUpdate`, `Extract`, `ToggleMods`, `MoveSelected`, `UpdateOptions`, `AddToProfile`, `Apply`, `Deploy`, `Remerge`, `ResetPending`, profile CRUD, `CleanProfile`, `SaveSettings`, `ResetSettings`, `ImportCemu`, `MigrateBcml`, `PackageMod`, `DoUpdate`, `Restart` | `Command` |
| `Noop`, `ResetMods`, `HandleMod`, `RemoveMods`, `AddMod` result, `Error`, `ReloadProfiles`, `RefreshModsDisplay`, `SetChangelog`, `OfferUpdate`, `SetDownloading`, `ResetPacker`, `HandleSettings`, `UpdatePackageMeta`, `RequestMeta`, `RequestOptions` | `Event` / `Inspection` (frontend reacts: refresh view, open modal, toast, update `package_builder`) |

`HandleMod` → the Session emits `Inspection::{WrongPlatform, MissingMasters,
NeedsOptions, Ready}` from the Open job (fixes §22.10 ordering: checks become
data, not reducer control flow). Required-option gating is
`OptionsPrompt::resolve` returning `None`, so an unrepresentable install
cannot be constructed.

---

## 5. Step ordering (each step independently shippable)

### 3a. Session core + pure helpers (GUI untouched)

- `crates/uk-app/src/session/{mod.rs,command.rs,event.rs,snapshot.rs,inspection.rs,job.rs}`;
  `sort.rs`/`filter` helpers next to `order.rs`.
- Implement the mods-tab job subset: `Open`, `Install`, `InstallBatch`,
  `SetEnabled`, `SetOptions`, `MoveTo`, `Apply`, `CancelPending`, `Deploy`,
  `Remerge`, `ResetPending`, `Uninstall`, `SwitchProfile`,
  `CreateProfile`, `AddToProfile`, `DevUpdate`, `Extract`, `UpdateFromPath`,
  `DuplicateProfile`, `RenameProfile`, `DeleteProfile`.
- Headless tests (temp config via `Settings::load_from` + `Manager::init_with`).
- **Shippable:** `cargo test -p uk-app` green, `cargo check --workspace
  --all-targets` clean, GUI unchanged.

### 3b. GUI drives the mods tab through the Session

- `App` gains `session: Session`, `snapshot: Arc<Snapshot>`,
  `events: Receiver<Event>`, `visible: Vec<ModId>`, `selected: Vec<ModId>`,
  `hover: Option<ModId>`, `drag: Option<ModId>`, `progress: Option<String>`.
- Delete `mods`, `displayed_mods`, `dirty`, `dirty()`/`dirty_mut()`,
  `filtered_mods`, `Sort::orderer` (frontend side), and the selection
  reducer variants' `self.mods[i]` indexing.
- `render_modlist` derives `visible` from the snapshot + `list_state` +
  filter; rows emit `UiMsg` with `ModId`s; drag computes `MoveTo` dest via
  `load_index_of`.
- `handle_update` → `drain_events` (all events per frame), `Revision` →
  re-read snapshot + derive `visible` + reconcile selection.
- **Shippable:** enable/disable, options, reorder, multi-select, sort,
  filter, uninstall, Apply, Cancel all work; `App.mods`/`dirty` gone.

### 3c. Install pipeline

- `Open` emits `Inspection`; frontend renders meta/options modals and
  dispatches `Install(candidate)` / `Open { meta: Some(..) }`.
- `InstallBatch` replaces `install_queue` + `update_mod`: per-file
  `Progress`, `Installed`, aggregated `Error` at the end; `installing`
  count in the snapshot.
- Delete `install_queue`, `error_queue`, `update_mod`, `open_mod_task`, the
  reducer's platform/masters checks.
- **Shippable:** zip/bnp/legacy/directory install, batch, metadata prompt,
  options gating, update-in-place.

### 3d. Profiles through the Session

- Profile commands + `ProfilesChanged`/`Revision`; selection reconciled on
  profile switch (§22.39); `AddToProfile`/`DevUpdate`/`UpdateFromPath` use
  the locked save-immediately semantics (§8.1).
- **Shippable:** profile create/duplicate/rename/delete/switch/add-to, no
  dirty-map panic, correct selection.

### 3e. Cleanup + parity

- Delete dead `Message` variants, `do_task`, `RefreshModsDisplay` plumbing,
  the busy `Cell`; busy overlay reads `snapshot.busy` + `progress`.
- One-click bridge maps `IpcMessage` → `Command`/`Event`s.
- Run `GUI_REF.md` §23.5 smoke checklist; extend it with "drag under a
  Name sort moves the clicked mod" and "switch profile clears stale
  selection".
- **Shippable:** `App` holds only view state; phase-3 e2e (below) passes.

---

## 6. Test plan

Headless `uk-app` tests (temp config, no display):

1. **Round trip**: `SetEnabled` → revision +1, entry flag flips; `MoveTo` →
   load order; `CancelPending` → core order restored.
2. **§22.1 regression**: sort by Name desc + filter; `visible` order, then
   `load_index_of` mapping; `move_ids` with a multi-selection under a
   non-Priority sort moves exactly the clicked ids.
3. **§22.4 regression**: two profiles; pending entries independent; a newly
   created profile has an empty manifest (no panic, no first-profile
   fallback).
4. **§22.8**: dispatch N commands, drain once → N `JobDone` events.
5. **Options gating**: `OptionsPrompt::resolve` with an unmet required group
   → `None`; with a pick → `InstallCandidate` (and install succeeds).
6. **Inspections**: wrong-platform fixture → `WrongPlatform`; masters
   fixture → `MissingMasters`; missing meta → `NeedsMeta`; junk → `Error`.
7. **§22.36**: `InstallBatch` of two paths (one junk) → one `Installed`,
   one `Error`, `installing` goes 2 → 0.
8. **§22.26**: one-click download job → `JobStarted`/`JobDone`, `busy`
   cleared by the `JobDone` (no stray flag).
9. **Panic containment**: a job that panics yields `JobDone(Err)`.
10. **§22.6/31/32/38 regression**: `SetOptions`/`AddToProfile`/`DevUpdate`/
    `UpdateFromPath` persist — assert `profile.yml` on disk after each.

E2E (`crates/uk-app/tests/e2e.rs`, `UKMM_TEST_DUMP`-gated): extend the
current install→enable→move→apply scenario to drive `Session` directly with
no UI at all; assert `profile.yml` load order + enabled flags and the merged
manifest.

---

## 7. Risks and landmines

- **Incremental merge correctness** is the highest-stakes behaviour:
  carry the dirty map over exactly, and add a test comparing old-path vs
  new-path `apply_changes` results on a fixture profile (`GUI_REFACTOR.md`
  §7).
- **`Manager::reload()`** re-creates mods + deploy managers. Only the
  Session may call it (phase 4's `SaveSettings`), inside a job, followed by
  a full entries/dirty refresh.
- **`profile()`/`get_profile()` panic** on an unknown current profile; the
  Session must guarantee the current profile exists (seed from settings,
  validate `ProfileName` at the command boundary — `ProfileName::parse`
  already exists).
- **`switch_profile` auto-creates** a missing profile via
  `create_profile_if`; that is current behaviour — keep it, but make the
  dirty-map entry appear with it (§22.4).
- **`Mod` `PartialEq`/`Hash` is the hash only**; never put a `Mod` clone in
  an event that outlives the job — events carry `ModId`. Selection compares
  ids, `state_eq` compares enabled/options.
- **`MANIFEST_CACHE` is process-global** keyed by `(hash, option paths)`;
  replacing a mod changes its hash, but in-place option edits reuse the
  key — invalidate deliberately in `SetOptions` if paths collide.
- **Unsafe sites to delete**: `dirty_mut`'s Polonius raw pointer (`gui.rs:440`)
  and `get_mod(hash).unwrap_unchecked()` (`update.rs:332`); replace with
  checked lookups by `ModId`.
- **Statics stay frontend** (`LOCALIZATION`, `info::ROOTS`, `settings::CONFIG`,
  the md cache). Events must carry enough data for the frontend to clear
  them (e.g. `Revision` + `ProfilesChanged`); the Session must not import
  frontend statics.
- **Startup blocking**: `App::new` does `Manager::init().unwrap()` and
  spawns the update check (`gui.rs:307, 372-383`). Phase 3 keeps this but
  moves `Manager::init` behind `Session::open`; the "startup job" move is a
  phase-4/server concern (`GUI_REFACTOR.md` §7) — note it, don't do it.
- **`Vec<Mod>` vs `Vec<ModEntry>` boundary**: `apply_changes` takes `Vec<Mod>`;
  the Session keeps `Mod` internally and builds the `ModEntry` snapshot, or
  keeps `ModEntry` + resolves `Mod` on apply. Choose `Mod` internally
  (matches manager APIs) and convert at the snapshot edge.
- **Drag destination semantics**: `move_selected` clamps `dest` and is a
  no-op when everything is selected; `MoveTo` must keep those semantics
  (tests exist in `order.rs`).
- **Serde**: `Command`/`Event` must stay data-only (no `anyhow::Error`,
  no `Mod` with readers); `AppError` gets a manual wire form.

---

## 8. Decisions (locked 2026-11-17, with the user)

1. **Save semantics**: save. `SetEnabled`, `MoveTo`, `SetOptions`,
   `AddToProfile`, `DevUpdate`, `UpdateFromPath`, `Uninstall` call
   `mod_manager.save()` on the affected profile after mutating. Fixes
   §22.6/31/32/38; user-visible change (those actions now survive a
   restart) — call it out in the changelog.
2. **Context-menu targets**: the clicked mod; if the clicked mod is in a
   multi-selection, the whole selection. Fixes §22.10 and removes the
   §22.34 empty-selection panic (commands carry explicit ids).
3. **Command enum scope**: grows per phase. Settings-draft, emulator,
   open-folder, restart, and parse-meta commands land in phase 4.
4. **Selection on profile switch**: cleared (fixes §22.39).
5. **`Sort` location**: the enum moves to `uk-app` (serde, re-exported);
   `ui.json`/`ModListState` persistence is unchanged.
6. **Busy/progress shape**: `Snapshot.busy: Option<JobId>` +
   `Snapshot.progress: Option<String>`; the frontend renders the overlay
   from the snapshot and never sets its own busy flag. `SetDownloading`
   becomes a real job (§22.26).

---

## 9. Phase 3 is shippable when

1. `App` no longer has `mods`, `dirty`, `install_queue`, `error_queue`,
   `update_mod`, or `busy`; `dirty()`/`dirty_mut()` are deleted.
2. The GUI drives enable/options/order/apply/cancel/install/uninstall and
   profile switching through `Session::dispatch`; events are drained in a
   loop per frame.
3. Selection, drag, context menus, and `MoveTo` carry `ModId`s; the
   display-vs-load-order mismatch is gone (regression tests for §22.1/4/8/
   31/32/36/39).
4. `cargo test -p uk-app` green (headless Session + helpers), `cargo
   check --workspace --all-targets` clean, `cargo tree -p uk-app` still
   contains no egui/eframe/rfd/winit.
5. The `UKMM_TEST_DUMP` e2e drives install → enable → move → apply with no
   UI at all.
6. `GUI_REF.md` §23.5 manual checklist passes (user runs it).

---

## 10. Review outcome (2026-11-19)

Phase 3 was reviewed against this plan, `GUI_REF.md`, the original reducer at
`ad73c75`, and the `uk-manager` semantics. **Verdict: plan-conformant** — all
automated §9 criteria are green (88 `uk-app` lib tests, clean
`cargo check --workspace --all-targets`, empty dep guard). The review found
six items; resolutions landed in `8d68071` ("Address phase 3 review findings"):

1. **Single-file installs lost the options prompt.** Drops / the file picker
   always dispatched `InstallBatch` (auto-resolved defaults). Fixed: a single
   path now routes through `Command::Open`, so `Inspection::NeedsOptions` /
   `NeedsMeta` prompt again (GUI_REF §9.3 parity); true batches keep defaults.
2. **No §6.9 panic-containment test.** Added tests: a job that panics (corrupt
   `profile.yml` → panic inside the core) yields `JobDone(Err)` +
   `Event::Error` and the session stays responsive; deleting a profile dir the
   core never loaded no longer panics in `get_profile` (the handler teaches
   the core via `ensure_profile_known` first); background jobs never set
   `Snapshot::busy`.
3. **Startup update check froze the UI** (busy → `modal_open()` greys
   everything; `http_get` had no real timeout). Fixed with
   `Session::dispatch_background` (the startup `CheckForUpdates` runs as a
   background job — a deliberate partial step toward the §7 "startup job"
   item, which stays a phase-4 concern for `Manager::init`) plus overall HTTP
   timeouts (10 s API calls, 5 min downloads).
4. **Cosmetics:** combined `MissingMasters` error rendered "(version )" — now
   one clean `AppError::MissingMasters` per master; the meta modal now prefills
   from `Inspection::NeedsMeta.suggested_name`; three clippy nits fixed
   (let-chain in `handlers.rs`, `trace!` borrow and `.last()` → `.next_back()`
   in `oneclick.rs`).
5. **Accepted as-is (recorded here so it is not lost):** `render_mod_row`
   clones the full `ModEntry` per row per frame plus two O(n) scans per row
   (load-index lookup, selection check). Fine at current mod counts; revisit
   only if large profiles become slow.
6. **Deliberate deviations are improvements** and are documented in
   `GUI_REF.md` §23.6: `Deploy` no longer clears pending (the old flow lost
   staged-file tracking — deploy only copies the merged output); descending
   sorts are now stable; profile rename heals the core's profile map.

**§9 status:** criteria 1–4 green. Criterion 5 (`UKMM_TEST_DUMP` e2e) is
written (`crates/uk-app/tests/e2e.rs`) but needs a real game dump — pending,
setup per `README.md`. Criterion 6 (GUI_REF §23.5 checklist, items 1–14) is
implemented but not yet run manually — pending user run.
