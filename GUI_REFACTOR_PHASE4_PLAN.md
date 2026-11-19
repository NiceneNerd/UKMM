# UKMM refactor — detailed plan for phase 4 (move the services)

> Working plan for phase 4, kept in the repo on branch `gui2`. Phase 4 moves
> the remaining services out of the egui frontend into `uk-app`: profiles,
> settings drafts (wipe rule, endian fix, per-frontend state file, `LocLang`
> out of core), host commands (emulator launch, open-folder, URL-handler
> registration, restart), `ModReader::thumbnail()`, and resilience at
> `Manager::init`.

Companion to `GUI_REFACTOR.md` (repo root) and `GUI_REF.md` (parity spec).
Scope: branch `gui2`, HEAD after phase 3 (`b6be2b7`) — phases 1–3 landed.

Phase 3 left these `GUI_REF.md` §22 defects **deliberately reproduced** and
owned by phase 4: **2, 3, 5, 7, 9, 16, 20, 29, 37**.

---

## 0. Where phase 3 left things

Already `Command`s on the `Session` (no phase-4 work needed): `ImportCemu`,
`MigrateBcml` (import/migration ✅), `CheckForUpdates` / `InstallUpdate`
(updater ✅), `OneClick` + `host::oneclick` IPC (one-click ✅),
`Package` / `ParseMeta`, and all profile ops (`SwitchProfile`,
`CreateProfile`, `DuplicateProfile`, `RenameProfile`, `DeleteProfile`,
`AddToProfile`) with `ProfileName::parse` validation at the boundary
(§22.2 ✅ at the GUI entry points). `Meta::blank(platform)` already exists in
`uk-mod` and both default-Meta sites use it ✅.

Still in the frontend (phase 4's work):

- `src/gui/settings.rs`: `temp_settings: Settings` + global `CONFIG:
  RwLock<FxHashMap<Platform, PlatformSettingsUI>>` drafts, `DumpType`, the
  endian substring hack (§22.7), and the save/wipe logic split between
  `settings.rs` (CONFIG consumption, §22.5) and `update.rs::SaveSettings`
  (wipe `.is_ok()` swallow, §22.37; save + `Reload` + `CancelPending` +
  `ResetPending`).
- `src/gui/profiles.rs`: `ProfileManagerState` does raw `fs` reads of
  `profile.yml` (§22.24/25); the profile combo reads the fs every frame via
  `settings.profiles()` (§22.25).
- `src/gui/deploy.rs`: emulator spawn + `sleep(1s)` + `try_wait` inside a
  render closure (§22.16).
- `src/gui/menus.rs` / `deploy.rs` / `mods.rs`: `open::that` / `explorer`
  calls in render code; `settings.rs` calls `host::handlers::register_url_handler()`
  directly.
- `src/gui/info.rs`: reads `thumb.*` out of the mod zip in a render helper
  (§22.29 cache is hash-keyed already — the move just cleans up).
- `src/gui.rs`: `Manager::init().unwrap()` (§22.9); `Settings::state_file()`
  shared `ui.json`; `Settings.lang: LocLang` in core settings.

## 1. Increments (each shippable, lowest risk first)

### 4a. Host commands + `ModReader::thumbnail()` (§22.16, §22.29)

- `uk-mod`: `ModReader::thumbnail() -> Result<Option<Vec<u8>>>` — reads
  `thumb.{jpg,jpeg,png,svg}` from the (already open) archive, or from a loose
  mod dir. Zip-level entries are `Stored` (`pack.rs`), so `ParallelZipReader::get_file`
  returns the raw image bytes. Add `Mod::thumbnail()` in `uk-manager` as a
  thin delegate (`ModReader::open_peek` + `thumbnail()`), since the GUI holds
  `Mod`s, not readers.
- `uk-app::host`: add `open` + `shlex` deps (move to `[workspace.dependencies]`).
  New fns: `open_path(&Path)`, `open_url(&str)` (via `open`, ignore failures —
  parity with §17.7), `launch_emulator(&str) -> Result<()>` (shlex split,
  spawn, 1s wait, `try_wait`; errors get context), `restart()` (spawn current
  exe, strip `.bak`, process group on unix).
- New commands: `LaunchEmulator`, `OpenFolder(FolderKind)`,
  `RegisterUrlHandler`. `FolderKind { Config, Storage, DeployOutput, Mod(ModId) }`;
  the handler resolves paths from the core (so a CLI can use them too). URL
  links stay plain `host::open_url` calls in the frontend (no state change, no
  job needed — decision, see §8).
- GUI: `deploy.rs` button dispatches `Command::LaunchEmulator`; menus/deploy/
  `mods.rs` `Mod_View` dispatch `Command::OpenFolder(..)`; settings one-click
  button dispatches `Command::RegisterUrlHandler`; `update.rs::restart` calls
  `host::restart()` then closes the viewport; `info.rs` uses `Mod::thumbnail()`
  (zip crate leaves the GUI render path; the hash-keyed `PREVIEW` cache stays —
  §22.29 fixed by keying on hash, update changes the hash).

### 4b. `ProfileService` + profiles in the `Snapshot` (§22.2, §22.3, §22.24/25)

- `uk-app::profile`: add `ProfileService` — `new(&Settings)`,
  `list() -> Vec<ProfileName>` (dir entries, `from_existing` — on-disk names
  are read as-is per `GUI_REFACTOR.md` §5), `read(&ProfileName) ->
  Result<Option<Profile>>` (parses `profile.yml`; unreadable → `None` +
  `log::warn`, §22.24 keeps the "silently omitted from the manager" behaviour
  but now through one validated API).
- `Snapshot` gains `profiles: Vec<ProfileName>`, rebuilt on `publish()` from
  the core — the GUI combo reads the snapshot instead of hitting the fs every
  frame (§22.25).
- GUI `ProfileManagerState::reload` goes through `ProfileService`.
- §22.3: `delete_profile`'s orphan-archive cleanup stops swallowing failures
  silently — `log::warn!` per failed removal (the profile delete itself still
  succeeds).

### 4c. Settings drafts in `uk-app::settings` (§22.5, §22.7, §22.20, §22.37)

- New module `uk-app::settings` (serde types, no toolkit):
  - `DumpDraft` (was `gui::settings::DumpType`), `PlatformDraft` (was
    `PlatformSettingsUI`), `SettingsDraft { current_mode, system_7z,
    storage_dir, check_updates, show_changelog, wiiu: Option<PlatformDraft>,
    switch: Option<PlatformDraft> }`.
  - `SettingsDraft::from_settings(&Settings) -> Self`;
    `PlatformDraft::differs_from(&self, saved: Option<&PlatformSettings>) -> bool`
    preserves the old `(None, Some(_)) => !dump.is_empty() || !output.is_empty()`
    rule.
  - `SettingsDraft::validate(self, old: &Settings) -> Result<Settings>` —
    validates **both** platform drafts before anything is applied (§22.5 fix),
    keeps `file_path` from `old`.
  - **Endian fix (§22.7):** `PlatformDraft::validate(platform)` uses
    `Endian::from(platform)` — never the `content` substring. ZArchive keeps
    `ResourceReader::from_zarchive` (self-detecting).
  - `wipe_needed(old, new) -> bool`: current platform's deploy
    layout/method/output changed.
- New `Command::SaveSettings(SettingsDraft)`; handler: validate → if
  `wipe_needed`, `wipe_output` and **propagate failure** (§22.37 fix: abort the
  save with an error instead of silently saving without the reset) → save →
  `core.reload()` → `CancelPending` → `ResetPending` if wiped → emit
  `Event::SettingsSaved { output_wiped, pending_reset }` + `SettingsChanged`.
  If `storage_dir` changed, `log::warn!` that data is not migrated (§22.20
  decision: no migration, explicit warning; noted in CHANGELOG).
- `ResetSettings` stays frontend-local (a draft reset is view state:
  `SettingsDraft::from_settings(core.settings())`) — deviation from the §2.2
  sketch, recorded in §8.
- GUI `settings.rs` rewritten to edit a `SettingsDraft` (no global `CONFIG`);
  Save dispatches `Command::SaveSettings(draft.clone())`; `Event::SettingsSaved`
  → toast + reset draft + clear selection (current behaviour).

### 4d. Per-frontend state file + `LocLang` out of core settings

- `uk-manager::Settings`: add `format_version: u32` (`#[serde(default =
  "default_format_version")]` → 1 for old files; `save()` writes 2) and make
  `lang` `#[serde(default, skip_serializing)]` — v1 files (lang inside) still
  read, new writes omit it. `state_file()` → replaced by
  `Settings::frontend_state_file(frontend: &str)` → `ui-<frontend>.json`.
- GUI `UiState` gains `lang: LocLang`; reads `ui-egui.json`, migrating once
  from legacy `ui.json` (lang seeded from the legacy `settings.lang`).
  `SetLanguage` becomes pure frontend state persisted on exit (like theme).
- `uk-manager` keeps its `uk-localization` dep for now (the `LocLang` type in
  the legacy field); dropping `uk-content → uk-localization` is phase 5.

### 4e. Resilient startup (§22.9)

- `uk-manager::mods::Manager::init`: a mod archive that fails to open is
  skipped with `log::warn!` instead of failing the whole init; a profile dir
  with a missing/malformed `profile.yml` gets a default `Profile` + warning
  instead of failing init. The GUI then opens normally (§22.9 fix; aligns with
  §22.24).

## 2. Locked decisions (this phase)

1. §22.37: a failed output wipe **aborts the save** with a typed error —
   never save settings while leaving a stale merged output un-wiped.
   User-visible; CHANGELOG.
2. §22.20: storage-folder changes do **not** migrate data; the save logs a
   warning. CHANGELOG.
3. §22.7: endian comes from `Platform` (`Endian::from(platform)`), never from
   path substrings. For unpacked dumps this matches the old hack for all real
   dumps (Wii U `…/content` → Big, Switch `…/romfs` → Little).
4. `ResetSettings` is not a `Command` — the draft lives in the frontend.
5. URL links (`open::that("https://…")`) are plain `host::open_url` calls, not
   commands — they mutate no session state.
6. `format_version` in `settings.yml`: v1 = lang inside (read compat),
   v2 = lang omitted (frontend state file owns it).
7. `Snapshot.profiles` is rebuilt on publish (per job), not per frame.

## 3. Phase 4 is shippable when

1. Every behaviour in `GUI_REF.md` §16/§17/§20 is reproducible from `uk-app`
   alone; the egui frontend holds only view state (selection, filter, sort,
   dock, drafts-as-view, pickers, toasts).
2. §22 items 2, 3, 5, 7, 9, 16, 20, 29, 37 are fixed or consciously decided
   with regression tests where testable (endian derivation, wipe-abort,
   draft validate-all-first, init resilience, profile service).
3. `cargo test -p uk-app` + `cargo test -p uk-content` green;
   `cargo check --workspace --all-targets` clean;
   `cargo tree -p uk-app` still free of egui/eframe/rfd/winit.
4. `GUI_REF.md` §23.5 smoke checklist still passes.

---

## 10. Phase 4 outcome (2026-11-19)

Landed as four commits on `gui2` (`7d32ccc`, `cbe68a3`, `4fdece6`, plus the
plan doc itself):

- **4a** `7d32ccc` — `uk-mod::unpack::read_thumbnail` / `ModReader::thumbnail` /
  `Mod::thumbnail()`; `uk-app::host::shell` (`open_path`, `open_url`,
  `launch_emulator`, `restart`); commands `LaunchEmulator`, `OpenFolder(FolderKind)`,
  `RegisterUrlHandler`. The GUI's deploy tab, menus, `Mod_View`, one-click
  button, restart, help links, and mod preview all go through `uk-app` now.
  Defects fixed: **16** (emulator spawn out of render code), **29** (thumbnail
  via uk-mod; cache stays hash-keyed).
- **4b** `cbe68a3` — `uk-app::profile::ProfileService` (list + validated read);
  `Snapshot.profiles: Vec<ProfileName>` rebuilt per publish; the profile combo
  and Send-to menu read the snapshot. `delete_profile` orphan cleanup now
  `log::warn!`s on failure. Defects fixed: **2, 3, 25**.
- **4c/d/e** `4fdece6` — `uk-app::settings::{SettingsDraft, PlatformDraft,
  DumpDraft}`; `Command::SaveSettings` + `Event::SettingsSaved { output_wiped,
  pending_reset }` (validate-all-first, wipe-failure-aborts-save, reload,
  pending reset). `settings.yml` gains `format_version` (v1 read compat, v2
  written; legacy `lang` read-only); per-frontend `ui-egui.json` state file
  with one-time migration from `ui.json`. `mods::Manager::init` skips
  unreadable mod zips / malformed `profile.yml` with warnings. Defects fixed:
  **5, 7, 9, 20, 37** (20 decided as warn-only, no storage migration).

Verification: `cargo test -p uk-app` 98 passed (incl. new tests:
`save_settings_validates_before_saving`, `save_settings_roundtrip_and_event`,
`save_settings_wipe_failure_aborts_save`, settings-module endian/wipe/differs
tests, `ProfileService` test); `cargo test -p uk-content` 193 passed;
`cargo check --workspace --all-targets` clean; `cargo tree -p uk-app` has no
egui/eframe/rfd/winit. (The 5 `uk-mod` fixture tests needing `test/wiiu.zip`
and the `uk-manager` `bnp::test_convert` needing a real BNP remain
developer-machine-only, as before — CI runs `uk-content` only.)

Deviations from the §1 sketch, all recorded in §2: `ResetSettings` stayed
frontend-local (view state); URL links are plain `host::open_url` calls, not
commands; dump "changed" detection compares full dump dirs, not just
`host_path` (the old comparison missed content-dir changes with equal host
root).
