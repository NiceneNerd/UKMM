# UKMM Application-Layer Refactor — Full Plan

Companion to `GUI_REF.md` (the behavioural parity spec of the current egui frontend). This document is the plan for extracting an application layer (`uk-app`) so that frontends become thin and any new frontend can be built with complete feature parity.

- **Parity spec:** `GUI_REF.md` — every GUI feature tagged `[UI-ONLY] / [APP] / [CORE] / [EGUI] / [HOST] / [GAP]`, a 39-item defect list (§22), and a suggested app-layer contract (§23). Every phase below is validated against it.
- **Scope:** the working tree as of 2026-11-16, where phase 2 is already in progress (§1).
- **Status (2026-11-19):** phases 1–4 have landed on branch `gui2` (@ `4fdece6`, in sync with `origin/gui2`); the egui frontend is now a thin view over `uk-app` (`Session`/`Command`/`Event`/`Snapshot`) plus host commands. The detailed per-phase working plans are held in the repo: `GUI_REFACTOR_PHASES12_PLAN.md`, `GUI_REFACTOR_PHASE3_PLAN.md` (includes the phase-3 review outcome, §10), and `GUI_REFACTOR_PHASE4_PLAN.md` (includes the phase-4 outcome).

## 1. Where we are now — phase 2 status

> Update 2026-11-19: phases 1–3 have since landed on `gui2` (see the Status
> note in the header). This section is kept as the historical record of the
> 2026-11-16 state.

Already landed in the working tree:

- **`crates/uk-app` exists** (workspace member, ~990 lines) with `sniff`, `open`, `apply`, `package`, `import` modules:
  - `open_mod(core, path, meta) -> Result<Opened>` with typed `OpenError`-based routing: BNP sniff → `convert_bnp`; non-UKMM archive → legacy convert with optional `Meta`; `Opened::NeedsMeta(path)` when metadata is required.
  - `apply_changes(core, mods, dirty)`, `package_mod`, `dev_update_mod`, `extract_mods`, `parse_meta`, `import_cemu_settings`, `migrate_bcml`.
- **The CLI no longer depends on the GUI.** `src/cli.rs` calls `uk_app::open_mod` / `package_mod` / `apply_changes` directly. This already fixes the BNP drift (CLI `install` now converts `.bnp`) and kills the error-string matching (`contains("meta file")` etc.) — replaced by `uk_app::Opened` / `OpenError`.
- `src/gui/tasks.rs` is now a thin adapter layer (355 lines): wrappers that call `uk_app::*` and map results to `gui::Message`. **Still contains `rfd::FileDialog`** in `dev_update_mods` and `extract_mods`, plus the host subsystems (self-update, GameBanana one-click + `ssilide` IPC, URL-handler registration, `response()` HTTP helper).

Not yet done (the remaining bulk of phases 2–4):

- `src/gui/update.rs` (696-line reducer) still owns: `HandleMod` platform/masters checks, `SaveSettings` wipe rule, profile fs ops, `rfd` calls in `SelectFile`/`ModUpdate`/`PackageMod`/Cemu-import, the `ChangeProfile` stale-selection bug, etc.
- Correctness-critical state still in `App`: `mods`, `dirty`, `install_queue`, `error_queue`, `update_mod`, `busy`, `temp_settings`, `CONFIG` drafts.
- `Manager` still hands out raw `parking_lot` guards; `Settings::state_file()` still shared; `LocLang` still in core `Settings`; `uk-content` still depends on `uk-localization`.

### What's wrong (diagnosis, from the original brief)

- **The CLI depended on the GUI** — now fixed; the drift (GUI-only BNP conversion) is gone.
- **Control flow depended on error text** — now fixed in `uk-app::open` via typed `OpenError`.
- **Correctness-critical state lives in `App`:** `dirty: RwLock<HashMap<String, Manifest>>` drives the incremental `deploy.apply(Some(manifest))` path; `mods: Vec<Mod>` shadows the load order and only reaches the core inside `apply_changes` via `set_enabled`/`set_enabled_options`/`set_order`/`save`; install and error queues and the `busy: Cell<bool>` guard also live there. A second frontend that doesn't replicate this bookkeeping produces wrong merges.
- **Domain rules sit in the reducer and views:** platform/masters checks in `HandleMod`; required-option gating computed inside the options modal's render code; profile delete/duplicate/rename do raw `fs` ops with an unvalidated user-typed rename target joined onto `profiles_dir()`; `SaveSettings` wipes output + resets pending when layout/method/output changed; endian inferred from `content_dir.to_string_lossy().contains("content")`.
- **Whole subsystems live in the GUI:** Cemu import, BCML migration (now in `uk-app::import`), self-update, GameBanana one-click with `ssilide` IPC, URL-handler registration, `response()`.
- **Toolkit calls sit inside "tasks":** `rfd` in dev-update/extract; emulator launch in `deploy.rs` runs `spawn` + `sleep(1s)` + `try_wait` inside a render closure; `info.rs` extracts `thumb.*` in a render helper.
- **The core leaks in return:** `Manager` hands out raw guards; `do_task` calls `dump().clear_cache()` after every task; `Settings::state_file()` serializes egui's `DockState`; `Settings.lang: LocLang` puts UI language in core settings; `uk-content` depends on `uk-localization`.
- **Test coverage is thin:** three `cfg(test)` modules in `uk-manager`/root; the `tasks.rs` one runs a full remerge against whatever config `Manager::init()` finds.

## 2. Target architecture

### 2.1 Crate layout

```
uk-content        (pure game-file domain; drop its uk-localization dep)
uk-util
uk-mod            (Meta, Manifest, pack/unpack; add ModReader::thumbnail(), one Meta constructor)
uk-reader
uk-manager        (core::Manager facade; sealed in phase 5)
uk-localization   (standalone; frontend-facing only)
uk-app            (application layer — the only crate frontends import for logic)
  ├── open/inspect service  (sniff, open_mod, Inspection)
  ├── session/              (Session, Command, Event, Snapshot, PendingChanges, job queue)
  ├── profiles.rs           (ProfileService on ProfileName)
  ├── settings.rs           (SettingsDraft::validate, wipe rule, Cemu/BCML import)
  ├── package.rs            (package/dev-update/extract/parse_meta)
  ├── host.rs               (self-update, one-click+IPC, URL handler, response(), emulator launch, open-folder)
  └── model.rs              (re-exports: Meta, ModCategory, option groups, ModEntry, ModId, …)
ukmm (root bin)   (main.rs: choose GUI vs CLI, a few dozen lines; cli.rs on Session)
frontends/egui/   (thin: egui rendering, dock, theme, modals, toasts, rfd, keybinds)
```

**Dependency rule:** `cargo tree -p uk-app` must contain no `egui`, `eframe`, `rfd`, `winit` (CI check, phase 5). Frontends depend only on `uk-app` + their toolkit + `uk-localization`.

### 2.2 The contract — Command in, immutable revisioned `Snapshot` + `Event` stream out

The core never asks a question mid-job. Anything needing input comes back as data (`Inspection`), and the frontend answers with a follow-up command. That replaces `rfd` calls inside tasks and the `RequestMeta`/`RequestOptions` round-trips.

```rust
// --- identity ---
pub struct ModId(pub usize);                 // stable hash; selection/drag hold these, not Mod clones
pub struct ProfileName(String);              // ProfileName::parse rejects "", ".", "..", separators
pub struct JobId(pub u64);

// --- input the frontend may need to collect, returned as data ---
pub enum Inspection {
    Ready(InstallCandidate),
    NeedsOptions(OptionsPrompt),
    NeedsMeta { path: PathBuf, suggested_name: String },
    WrongPlatform { mod_is: Platform, current: Platform },
    MissingMasters(Vec<MasterRef>),         // (name, version) pairs; non-blocking warning today
}
pub struct OptionsPrompt { /* groups, picks, required flags; */
    pub fn resolve(self, picks: Picks) -> Option<InstallCandidate> /* None if a required group is unmet */ }
pub struct InstallCandidate { /* private ctor; only via resolve() or the no-options path */ }

// --- commands ---
pub enum Command {
    Open(PathBuf),                            // classify + inspect; never blocks on input
    Install(InstallCandidate),                // unrepresentable without resolved options
    InstallBatch(Vec<PathBuf>),               // replaces install_queue
    SetEnabled(Vec<ModId>, bool),
    SetOptions(ModId, Picks),
    MoveTo { ids: Vec<ModId>, index: usize },
    Apply,
    CancelPending,
    Deploy, Remerge, ResetPending,
    SwitchProfile(ProfileName),
    CreateProfile(ProfileName), DuplicateProfile(ProfileName),
    RenameProfile { from: ProfileName, to: ProfileName }, DeleteProfile(ProfileName),
    AddToProfile { ids: Vec<ModId>, profile: ProfileName },
    Uninstall(Vec<ModId>),
    UpdateFromPath { id: ModId, path: PathBuf },
    DevUpdate { ids: Vec<ModId>, folders: HashMap<ModId, PathBuf> },  // no rfd inside
    Extract { ids: Vec<ModId>, dest: PathBuf },
    Package { source: PathBuf, dest: PathBuf, meta: Meta },
    SaveSettings(SettingsDraft), ResetSettings,
    ImportCemu(PathBuf), MigrateBcml,
    RegisterUrlHandler, CheckForUpdates, InstallUpdate, Restart,
    LaunchEmulator, OpenFolder(FolderKind),
    SetLanguage(LocLang),                     // frontend-state command; persisted per frontend
}

// --- output ---
pub struct Snapshot {           // immutable, revisioned; Arc-shared
    pub revision: u64,
    pub profile: ProfileName,
    pub mods: Vec<ModEntry>,    // vector order IS load order; ModEntry { id, meta, enabled, options, path }
    pub pending: PendingSummary, // staged edits owned by the core, not the UI
    pub settings: SettingsView, // effective settings (draft vs saved exposed)
    pub busy: Option<JobId>,
    pub updates: Option<VersionInfo>,
}
pub enum Event {
    Revision(u64),
    Inspection(Inspection),
    Installed { id: ModId }, Updated { id: ModId }, Removed { ids: Vec<ModId> },
    PendingChanged { files: usize },
    JobStarted(JobId), Progress { job: JobId, message: String }, JobDone { job: JobId, result: Result<(), AppError> },
    SettingsSaved { output_wiped: bool, pending_reset: bool },
    Error(AppError),            // typed; frontends localize
    UpdateAvailable(VersionInfo), Changelog(String),
}
pub enum AppError {
    WrongPlatform { .. }, MissingMasters(Vec<String>), NeedsMeta { .. },
    DumpMissing { .. }, Io { .. }, Other(anyhow::Error),
}

pub struct Session { /* … */ }
impl Session {
    pub fn open(core: Manager) -> Result<Session>;
    pub fn snapshot(&self) -> Arc<Snapshot>;
    pub fn subscribe(&self) -> broadcast::Receiver<Event>;   // or mpsc per subscriber
    pub fn dispatch(&self, cmd: Command) -> JobId;
}
```

Key properties:

- **Type-level guarantees:** `InstallCandidate` is only constructible through `OptionsPrompt::resolve`, so installing with an unmet required option group is unrepresentable. `ProfileName::parse` rejects empty names, `.`, `..` and path separators, so profile names are validated once at the boundary (fixes the unvalidated-rename defect).
- **One mutating job at a time**, enforced inside `Session` (replaces `busy: Cell<bool>`); read-only queries never block.
- **Serde derives on `Command`, `Event`, `Snapshot`** from day one → enables `ukmm serve` (web/TUI) later and CLI `--json`.

### 2.3 Job/threading model

`Session` owns one worker thread (or a small pool behind a single-mutation mutex). `dispatch` enqueues; the worker runs the command, mutates state, bumps `revision`, publishes `Event`s. Long operations (merge, deploy, download, import) emit `Progress`. A panic in a job becomes `Event::JobDone(Err)` + backtrace, never a dead UI. Events stream, and the frontend drains all of them per frame — fixing the current one-message-per-frame backlog.

### 2.4 State split

| Moves into `uk-app` (Session/state) | Stays in the frontend |
|---|---|
| load order, enabled flags, option picks (the `mods: Vec<Mod>` shadow) | selection (as `Vec<ModId>`), drag index |
| per-profile pending manifest (`dirty`) + `PendingChanges` | filter text, sort mode/direction (pure helpers `sort_mods(entries, Sort, desc)`, `filter_mods(entries, &str)` exported for frontends) |
| profiles list + fs ops (`ProfileService`) | dock layout, theme, closed tabs |
| settings drafts (`temp_settings`, `CONFIG`) + validation/wipe rule | modal stack, confirm dialogs, toasts |
| install batch queue + replacement target + error aggregation | file-picker browser state, scroll positions |
| one-mutating-job guard + progress | keyboard focus ownership |
| UI language preference (moved out of core `Settings` → per-frontend state file) | localization lookup itself |

## 3. Module-by-module move map

| Current location | Destination | Phase | Notes |
|---|---|---|---|
| `uk-app::{sniff,open}` | done | 2 ✅ | typed `OpenError`; CLI uses it |
| `uk-app::{apply,package,import}` | done | 2 ✅ | still `Result<()>`; become `Command` handlers in phase 3 |
| `gui/tasks.rs` wrappers + `rfd` in dev-update/extract | delete; GUI sends `DevUpdate{folders}` / `Extract{dest}` after its own `rfd` | 2 | |
| `gui/tasks.rs` + `handlers.rs`: self-update, one-click/`ssilide`, URL registration, `response()` | `uk-app::host` | 2/4 | `response()` → `host::http_get` |
| `gui/update.rs` `HandleMod` checks | `Inspection::{WrongPlatform, MissingMasters}` in open/inspect | 3 | |
| options gating in `gui/options.rs` render code | `OptionsPrompt::resolve` | 3 | unrepresentable-bad-install |
| `App.mods`, `App.dirty`, `install_queue`, `error_queue`, `update_mod`, `busy` | `Session` state + job queue | 3 | fixes wrong-merge risk; `unwrap_unchecked` lookups disappear (index by `ModId`) |
| `gui::Message` | split into `UiMsg` (frontend-internal) + `Command` (→ Session) | 3 | `Error`/`Noop`/`ResetMods` become `Event`s |
| profile fs ops + `profiles.rs` reads | `uk-app::profiles::ProfileService` on `ProfileName` | 4 | fixes unvalidated rename; delete-cleanup becomes transactional |
| `gui/settings.rs` drafts, `DumpType`, endian inference, wipe rule | `uk-app::settings::SettingsDraft::validate() -> Result<Settings>` | 4 | endian: replace substring hack with explicit platform field (§4) |
| Cemu/BCML import | already in `uk-app::import`; wrap as `Command`s | 2 ✅ / 4 | |
| `gui/deploy.rs` emulator spawn+sleep+`try_wait` | `Command::LaunchEmulator` via `host` | 4 | no render-code process spawning |
| "open mod folder" | `Command::OpenFolder` | 4 | |
| `gui/info.rs` thumb extraction | `ModReader::thumbnail()` in `uk-mod` | 4 | |
| `ModPackerBuilder::new` / `MetaInputModal::open` default Meta | single `Meta::blank(platform)` in `uk-mod` | 4 | |
| `Settings::state_file()` shared `ui.json` | per-frontend file, e.g. `ui-egui.json` | 4 | |
| `Settings.lang: LocLang` | frontend state; settings.yml gains a format version | 4 | see §4 |
| `Manager` guard-returning accessors | methods; internals `pub(crate)` | 5 | |
| data types frontends need | re-export from `uk-app::model` | 5 | |
| `uk-content` → `uk-localization` dep | remove; content stays pure | 5 | |
| CLI | rewrite on `Session`, add `--json`, must cover every `Command` | 6 | architecture test |

## 4. Phases — each shippable

### Phase 1 — Safety net

- Unit tests for the pure pieces: Cemu XML parsing against fixture `settings.xml` / `title_list_cache.xml`; archive sniffing on generated zips/7z/dirs (BNP vs legacy vs UKMM); reorder logic (`MoveTo` semantics incl. all-selected no-op and clamping); option resolve/required gating; `ProfileName::parse` rejects `""`, `.`, `..`, `/`, `\`; `Meta::blank`.
- Replace the `tasks.rs` remerge test with a temp-config-dir test; add an opt-in end-to-end scenario gated on `UKMM_TEST_DUMP` (install → enable → move → apply against a fixture dump).
- Fix, or consciously defer with a regression test, the `GUI_REF.md` §22 "danger" items as tests land: display-index vs load-order mismatch, stale selection on profile switch, missing saves on AddToProfile/dev-update/mod-update, `dirty()` fallback/panic.
- **Shippable when:** `cargo test` green on the pure pieces and the e2e scenario runs against a fixture, not the developer's real config.

### Phase 2 — Extract `uk-app` mechanically ✅ landed on `gui2` (`58568de`, `80e39bb`, `ad73c75`)

- ✅ `sniff`/`open`/`apply`/`package`/`import` moved; CLI decoupled from the GUI; BNP drift and error-string matching fixed.
- Remaining:
  1. Strip `rfd` from `gui/tasks.rs` — the GUI collects paths itself and sends data-only commands (`DevUpdate { folders }`, `Extract { dest }`).
  2. Move self-update (GitHub API, asset names, exe swap + restart), GameBanana one-click + `ssilide` IPC, URL-handler registration, and `response()` into `uk-app::host`.
  3. Shrink `gui/tasks.rs` to zero — the GUI calls `uk-app`/`Session` directly.
  4. Verify against `GUI_REF.md` that no behaviour changed.
- **Shippable when:** CLI and GUI both build on `uk-app` only; `src/gui/tasks.rs` is deleted; the `GUI_REF.md` checklist is still green.

### Phase 3 — Move the staged state (the heart of the refactor) ✅ landed on `gui2` (`fafc18e`, `0c9ed3a`, `b58a9c6`, `8d68071`; review outcome in `GUI_REFACTOR_PHASE3_PLAN.md` §10)

- Add `Session`, `Snapshot`, `Event`, `PendingChanges`, and the job queue (one mutating job at a time, progress events).
- Split `gui::Message` into `UiMsg` + `Command`; `Error`/`Noop`/`ResetMods` become `Event`s; the frontend drains all events per frame.
- Selection and drag hold `ModId`s; `displayed_mods` is derived from the `Snapshot` via the pure `sort`/`filter` helpers — fixes the display-index vs load-order mismatch.
- `HandleMod` checks become `Inspection` variants; option gating becomes `OptionsPrompt::resolve`.
- **Shippable when:** the GUI drives everything through `dispatch`; `App.mods`/`dirty`/`busy` are gone; the phase-1 e2e scenario passes headless (enable/move/apply with no UI at all).

### Phase 4 — Move the services, lowest risk first

Order: profiles → settings drafts (+ wipe rule, endian fix, per-frontend state file, `LocLang` out of core) → import/migration → updater → one-click. Plus `ModReader::thumbnail()` in `uk-mod` and the single `Meta` constructor.

Deferred from phase 3 (per `GUI_REFACTOR_PHASE3_PLAN.md` §0.5): GUI_REF §22 items **2, 3, 5, 7, 9, 16, 20, 29, 37** are deliberately reproduced for now and belong to phase 4; items **11–14, 17–19, 22–25, 27, 30, 33, 35** were out of scope for phase 3.

- **Shippable when:** every behaviour in `GUI_REF.md` §16/§17/§20 is reproducible from `uk-app` alone; the egui frontend holds only view state.

### Phase 5 — Seal it

- Replace `Manager`'s guard-returning accessors with methods; make `uk-manager` internals `pub(crate)`; re-export the data types frontends need (`Meta`, `ModCategory`, option groups) from `uk-app::model`; remove the `uk-content → uk-localization` dependency.
- CI check: `cargo tree -p uk-app` contains no `egui`, `eframe`, `rfd`, `winit`.
- Settings format version bump for the `LocLang` move (read old `settings.yml`, write new).
- **Shippable when:** CI green; no frontend types leak through `uk-manager`'s public API.

### Phase 6 — Prove interoperability

- Rewrite the CLI on `Session`, add `--json` (serde `Event`/`Snapshot` output), and require the CLI to cover **every** `Command` variant — that coverage is the architecture test.
- A frontend whose `Cargo.toml` lists only `uk-app` (+ toolkit + `uk-localization`) is the other half of the test.
- **Shippable when:** `ukmm --json install …` etc. work end-to-end against a fixture config in CI.

## 5. Data & format migrations

- `settings.yml`: add `format_version`; when moving `lang` out, read v1 (lang inside settings) → write v2 (lang in the frontend state file). Portable-mode (`--portable`) paths unchanged.
- `ui.json` → `ui-egui.json` per frontend; read the old file once for migration.
- Profile names: existing on-disk names are read as-is; only new create/rename go through `ProfileName::parse`.
- `pending.yml`: the legacy `{files, delete}` schema is already auto-migrated by the deploy manager — keep that.
- Endianness: stop inferring from the `content` substring; store explicit platform/endian in `PlatformSettings` (derived from `Platform`, with a one-time migration for existing configs).

## 6. Testing & CI

- Per-crate unit tests (phase 1) plus a `UKMM_TEST_DUMP` e2e in `uk-app` (temp config + fixture dump): full cycle install → options → reorder → apply → deploy, asserting the merged output manifest.
- Headless `Session` property test driving enable/move/apply with no UI.
- CI: `cargo tree -p uk-app` grep guard; `cargo test -p uk-app` with fixtures; CLI `--json` smoke test.
- Parity: the `GUI_REF.md` §23.5 smoke test as a manual checklist per phase; §22 defects get regression tests as they are fixed.

## 7. Risks & open questions

- **Incremental merge correctness** is the highest-stakes behaviour (`dirty` manifests drive `deploy.apply(Some(..))`). Phase 3 must carry the per-profile dirty map over exactly; add a test comparing old vs new apply results on a fixture profile.
- **`busy` semantics:** some current paths set busy without a task (`SetDownloading`). Model one-click as an explicit job with `JobStarted`, not a stray flag.
- **Blocking calls at startup** (update check, `Manager::init`) — move into a startup job emitting events; the first frame should not block.
- **Serde on `Command` with `anyhow::Error`:** `AppError::Other` won't round-trip. Decision: stringify for the wire, keep typed in-process.
- **`Manager::reload()`** re-creates everything; `Session` should own reload as a command so events stay consistent.

## 8. Decisions (locked)

- Phase-specific locked decisions live with each phase's working plan (`GUI_REFACTOR_PHASES12_PLAN.md`, `GUI_REFACTOR_PHASE3_PLAN.md` §8). Notably, phase 3's save-semantics decision (mod actions save `profile.yml` immediately) is a user-visible change and is called out in `CHANGELOG.md` under `[Unreleased]`.
- Serializable contract: derive serde on `Command`, `Event`, `Snapshot` now — keeps an out-of-process `ukmm serve` (web UI, TUI) possible later.
- Typed errors: small `AppError` enum + `Other(anyhow::Error)` fallback; frontends localize. (Already landed for open via `OpenError`.)
- `LocLang` moves out of core `Settings` into frontend state, behind a settings-format version bump.
- `InstallCandidate` is only constructible through `OptionsPrompt::resolve`; `ProfileName::parse` validates once at the boundary.
- Sort and filter are pure helpers in `uk-app`, called by frontends.
- One mutating job at a time, owned by `Session`; progress via events, not log scraping.
