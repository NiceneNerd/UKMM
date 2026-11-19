# UKMM refactor — detailed sketch of phases 1–2

> Working plan for phases 1–2, kept in the repo on branch `gui2` (copied from
> the planning directory 2026-11-19). Both phases have since **landed** on
> `gui2` (`cfc6207` phase 1; `58568de`, `80e39bb`, `ad73c75` phase 2). This
> file is the historical spec.

Companion to `GUI_REFACTOR.md` (repo root) and `GUI_REF.md` (parity spec).
Scope: working tree as of 2026-11-16, where phase 2 is already in progress.

---

## Phase 1 — Safety net (finish before the big moves)

### 1a. Test inventory today

| Location | Tests | Problem |
|---|---|---|
| `crates/uk-app/src/sniff.rs` | 6 (1 `#[ignore]`d known gap: zip detection requires explicit directory entries) | good, needs 7z/bnp-archive cases |
| `crates/uk-app/src/open.rs` | 4 (`OpenError::is_not_ukmm_mod` classification on generated zips) | good; needs full `open_mod` routing with a core |
| `crates/uk-app/src/package.rs` | 2 (`parse_meta` round-trip, bad extension) | ok |
| `crates/uk-manager/src/{bnp,mods}.rs` | 2 | fine |
| `src/gui/tasks.rs:348` | 1 `remerge` test | **bad: runs a full remerge against whatever real config `Manager::init()` finds** |

### 1b. The temp-config blocker (do this first)

`Settings::config_dir()` and `Settings::path()` are `LazyLock`s over `dirs2::config_dir()` / `--portable`, so no test can redirect config. Add to `uk-manager`:

- `Settings::read(path)` already exists — add `Settings::load_from(path: &Path) -> Arc<RwLock<Settings>>` and `Manager::init_with(settings: Arc<RwLock<Settings>>) -> Result<Manager>` (refactor `init()` to call it).
- Then **delete the `src/gui/tasks.rs` remerge test** and re-create it in `uk-app` as a tempdir-config test.

### 1c. Test work items (all in `uk-app`, fixtures under `crates/uk-app/tests/fixtures/`)

1. **Sniffing** (`sniff.rs`): generated-zip cases exist; add 7z `.bnp` archive case (sevenz-rust can write) and directory-BNP case (exists); for `.rar` use a checked-in binary fixture (unrar can't write). Resolve or formally defer the ignored "no directory entries" gap.
2. **Open routing** (`open.rs`): with `Manager::init_with` on a tempdir config, test `open_mod` end-to-end: UKMM zip → `Opened::Mod`; zip with `content/` + `rules.txt` → `Opened::Mod` after legacy convert with `Some(meta)`; same without meta → `Opened::NeedsMeta`; `.bnp` → converted mod; junk → typed error.
3. **Cemu import parsing** (`import.rs`): split IO from XML logic — extract pure `cemu::parse_settings_xml(&str, &CemuDirs) -> Result<CemuPaths>` and `cemu::parse_title_list(&str) -> Vec<TitleEntry>`; test against fixture `settings.xml` (portable vs installed, `mlc_path`, the `U-King.rpx` scrape fallback, encrypted-dump rejection) and `title_list_cache.xml` (region suffixes `101C9400/101C9500/101C9300` both cases, formats 1/3/4, relative-path resolution order).
4. **BCML settings parse** (`import.rs`): extract pure `bcml::parse_settings_json(&str) -> BcmlSettings`; fixture `settings.json`; the mod-import half stays core-dependent.
5. **Reorder logic**: extract the `MoveSelected` algorithm from `gui/update.rs` into a pure `pub fn move_selected<T>(items: &mut Vec<T>, selected: &HashSet<usize>, dest: usize)` in `uk-app` (generic so it's testable without `Mod`); tests: all-selected no-op, dest clamping, relative order preserved, empty selection.
6. **Option gating**: pure `pub fn required_satisfied(groups: &[ModOptionGroup], picks: &Picks) -> bool` + tests for exclusive/multiple/required/defaults.
7. **`ProfileName`** (new type in `uk-app`, lands early because it's pure): `parse` rejects `""`, `.`, `..`, `/`, `\`, control chars; tests. (Full profile service is phase 4.)
8. **`Meta::blank(platform)`** in `uk-mod` replacing the two divergent default-Meta sites (`ModPackerBuilder::new`, `MetaInputModal::open`); test defaults (`version = "1.0.0"`, category `Other`).
9. **`UKMM_TEST_DUMP` e2e**: `crates/uk-app/tests/e2e.rs`, `#[ignore]` unless `UKMM_TEST_DUMP=/path/to/dump` set; temp config via `init_with`; drives install → options → reorder → apply → deploy against the fixture dump and asserts the merged manifest.

### 1d. Phase 1 shippable when

`cargo test -p uk-app` green (incl. temp-config tests), the root-crate remerge test is gone, e2e runs under `UKMM_TEST_DUMP`, and each `GUI_REF.md` §22 "danger" item fixed here has a regression test.

---

## Phase 2 — Extract `uk-app` mechanically (in progress)

### 2a. Already done ✅

- `crates/uk-app` exists with `sniff`, `open`, `apply`, `package`, `import` (~990 lines; deps already clean of egui/rfd).
- CLI decoupled: `src/cli.rs` calls `uk_app::open_mod` / `package_mod` / `apply_changes` — BNP drift and error-string matching are gone.

### 2b. Remaining steps, in order

**Step 1 — Create `uk-app::host` and move the host subsystems out of `gui/tasks.rs` + `gui/tasks/handlers.rs`:**

| From | To | Notes |
|---|---|---|
| `gui/util.rs::response()` | `host::http_get(url) -> Result<Vec<u8>>` | keep redirect-following + UA `UKMM` |
| `tasks.rs::{VersionResponse, VersionAsset, get_releases}` | `host::updates::{VersionResponse, ReleaseCheck}` | return data (`OfferUpdate(v)` / `Changelog(md)` / `None`), no flume sender, no `Message` |
| `tasks.rs::do_update` | `host::updates::install_update(v) -> Result<UpdatePlan>` | download + exe swap + `.bak`, returns "restart" plan instead of `Message::Restart` |
| `tasks.rs::{IpcMessage, oneclick, wait_ipc, ONECLICK_SENDER}` + `main.rs::INTERFACE` (port 6666) | `host::oneclick::{IpcMessage, start_listener() -> Receiver<IpcMessage>, process_url(url) -> IpcMessage}` | flume channel inside uk-app is fine (not a GUI dep); the GUI maps `IpcMessage` → its own messages. `main.rs` asks host to classify argv: `ArgAction::{OneClick(url), ModPath(path), None}` |
| `tasks/handlers.rs::register_handlers` | `host::register_url_handler()` | `winreg` moves to uk-app under `[target.'cfg(windows)'.dependencies]`; desktop-file logic unchanged |
| `tasks.rs::handle_mod_arg` | host returns `ArgAction::ModPath` | GUI/CLI dispatches it |

**Step 2 — Strip `rfd` out of the task wrappers.** `dev_update_mods`/`extract_mods` in `gui/tasks.rs` currently open pickers inside the worker. Change the flow: the context-menu handler in `gui/update.rs` runs `rfd` **first** (render code, where dialogs belong), then sends a message carrying the paths; the worker becomes pure: `dev_update(core, mods, folders: Vec<PathBuf>)`. The other `rfd` sites in `update.rs` (SelectFile, ModUpdate, PackageMod dest, Cemu-import folder) already live in render code — leave them, they just stuff paths into messages.

**Step 3 — Delete `src/gui/tasks.rs`.** Inline its ~10 thin wrappers into the `Message` handlers in `gui/update.rs` (each becomes `uk_app::foo(...)?` + map to the same `Message`). `INTERFACE` and IPC move per step 1.

**Step 4 — Dependency migration in Cargo.tomls:**
- Move to `uk-app`: `http_req`, `ssilide`, `lenient_semver`, `winreg` (windows-only), `open` (only if host needs it — otherwise stays in frontend), `shlex` (emulator launch is phase 4 — defer).
- Root keeps: eframe/egui stack, `rfd`, `flume`, `mimalloc`, `xflags`, `uk-ui`, `uk-localization`, `uk-app`.
- Add the CI guard now (it should already pass): `cargo tree -p uk-app | grep -E 'egui|eframe|rfd|winit'` must be empty.

**Step 5 — Introduce `uk_app::error::AppError`** for host/open operations (`WrongPlatform`, `MissingMasters`, `NeedsMeta`, `DumpMissing`, `UpdateUnavailable`, `Io`, `Other(anyhow::Error)`) so phase 3 has the typed-error surface ready. GUI's English-string matching is already gone for open; host errors still use anyhow strings — wrap at the `uk-app` boundary.

**Step 6 — CLI alignment:** `cli.rs` `package` uses `Meta::parse(&pkg.meta)` — switch to `uk_app::parse_meta` so CLI and GUI accept both `meta.yml` and `info.json`. `install` on `Opened::NeedsMeta` currently bails; add a `--meta <path>` flag that supplies the `Meta` (no prompt in CLI).

### 2c. Phase 2 shippable when

1. `src/gui/tasks.rs` and `src/gui/tasks/handlers.rs` are deleted; `main.rs` shrinks to arg classification + GUI/CLI dispatch.
2. `cargo tree -p uk-app` contains no egui/eframe/rfd/winit (CI check added).
3. `cargo test -p uk-app` green, incl. phase-1 tests.
4. Manual smoke against `GUI_REF.md` §23.5: install `.zip` / `.bnp` / legacy `rules.txt` mod / directory mod; update check + changelog modal; one-click URL forwards to a running instance; `bcml:` handler registration; packaging; GUI and CLI produce identical results for the same inputs.

### 2d. Deliberately out of scope for phase 2 (phase 3+)

`Session`/`Snapshot`/`Event`, the `gui::Message` split, `App.mods`/`dirty`/`busy` moving into core state, profile service, settings drafts, emulator launch as a command, `ModReader::thumbnail()`. Phase 2 only makes `uk-app` a complete toolkit-free library; the egui frontend still owns its state, but all *operations* and all *host* interactions go through `uk-app`.

---

## Open calls

- (a) The `Manager::init_with` / `Settings::load_from` seam in phase 1a is a small `uk-manager` API addition — okay to land in phase 1, or prefer a config-dir env-var override?
- (b) `host::oneclick` uses a flume receiver inside `uk-app` (matches existing code); alternative is `std::sync::mpsc`. Recommendation: keep flume.
