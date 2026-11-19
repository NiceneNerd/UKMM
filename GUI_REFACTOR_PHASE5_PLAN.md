# UKMM refactor — detailed plan for phase 5 (seal it)

> Working plan for phase 5, kept in the repo on branch `gui2`. Phase 5 seals
> the layering: frontends depend only on `uk-app` (+ toolkit + `uk-localization`),
> `uk-manager`'s internals stop leaking, and `uk-content` no longer depends on
> `uk-localization`.

Companion to `GUI_REFACTOR.md` (§4 phase 5, §3 move map rows marked phase 5).
Base: HEAD after phase 4 (`eef7734`).

Phase 5 scope from `GUI_REFACTOR.md` §4:

- Replace `Manager`'s guard-returning accessors with methods; make `uk-manager`
  internals `pub(crate)`; re-export the data types frontends need (`Meta`,
  `ModCategory`, option groups) from `uk-app::model`; remove the
  `uk-content → uk-localization` dependency.
- CI check: `cargo tree -p uk-app` contains no egui/eframe/rfd/winit (already
  enforced and green).
- Settings format bump for the `LocLang` move — **already landed in phase 4**
  (`format_version` v2 written, legacy `lang` read-only; `ui-egui.json`).
- Shippable when: CI green; no frontend types leak through `uk-manager`'s
  public API.

## 1. Current state (measured at base)

- Root binary (`src/`) imports lower crates directly in 10 files:
  `uk_manager::{core::Manager, settings::{Platform, Settings, DeployConfig,
  DeployMethod, DeployLayout}, mods::{Mod, Profile, LookupMod}, util::clear_temp}`,
  `uk_mod::{Meta, ModCategory, ModPlatform, ModOption, ModOptionGroup,
  OptionGroup, ExclusiveOptionGroup, MultipleOptionGroup, Manifest,
  pack::sanitise}`, `uk_content::{util::HashMap, constants::Language, UKError}`,
  `uk_util::OptionResultExt`.
- `core::Manager` exposes guard accessors: `settings()`, `settings_mut()`,
  `mod_manager()`, `mod_manager_mut()`, `deploy_manager()` (~95 call sites total,
  ~12 on the sub-managers).
- `mods::Manager` / `deploy::Manager` fields are already private; `Mod`'s fields
  are `pub` (data type — fine, re-exported via model).
- `uk-content → uk-localization` exists solely for `impl From<Language> for
  LocLang` in `uk-content/src/constants.rs` — **dead code** (no `LocLang::from`
  / `.into()`-to-LocLang call sites anywhere outside uk-localization itself).

## 2. Increments (each shippable)

### 5a. `uk-app::model` + root-crate diet

- New `crates/uk-app/src/model.rs` re-exporting the data types frontends need:
  - from `uk_mod`: `Meta`, `ModCategory`, `ModPlatform`, `ModOption`,
    `ModOptionGroup`, `OptionGroup`, `ExclusiveOptionGroup`, `MultipleOptionGroup`,
    `Manifest`, `pack::sanitise`
  - from `uk_manager`: `core::Manager`, `settings::{Settings, Platform,
    PlatformSettings, DeployConfig, DeployMethod, DeployLayout, UpdatePreference}`,
    `mods::{Mod, Profile, LookupMod}`, `util::clear_temp`
  - from `uk_content`: `constants::Language`, `UKError`, `util::HashMap`
  - from `uk_util`: `OptionResultExt`
  - `pub use` in `lib.rs` (`pub mod model;`).
- Switch all `src/**` imports from `uk_manager::`/`uk_mod::`/`uk_content::`/
  `uk_util::` to `uk_app::model::` (mechanical).
- Root `Cargo.toml`: drop `uk-content`, `uk-manager`, `uk-mod`, `uk-reader`,
  `uk-util`; keep `uk-app`, `uk-localization`, `uk-ui`. The binary then depends
  on lower crates only transitively — the phase-6 frontend test.
- uk-app internally keeps its direct deps (it sits above uk-manager), but
  `uk-app::model` is the canonical path for frontends.

### 5b. Remove `uk-content → uk-localization`

- Delete the dead `impl From<Language> for LocLang` in
  `uk-content/src/constants.rs` and the `uk-localization` dep from
  `uk-content/Cargo.toml` (+ `Cargo.lock` update). If a `Language → LocLang`
  mapping is ever needed, it belongs in the frontend, not in pure content
  code.
- Note: `uk-manager` still depends on `uk-localization` for the legacy
  read-only `Settings.lang: LocLang` field (v1 read compat) — that is a
  side-crate dep and allowed; removing it is a later cleanup once v1 files
  are gone.

### 5c. Seal `uk-manager`'s API

- `core::Manager`: replace the sub-manager guard accessors `mod_manager()`,
  `mod_manager_mut()`, `deploy_manager()` with closure-based
  `with_mod_manager<R>(f)`, `with_mod_manager_mut<R>(f)`, `with_deploy_manager<R>(f)`;
  migrate the ~12 call sites in `uk-app` + `src/`.
- `pub(crate)` pass over any remaining `pub` non-data fields in
  `mods::Manager`/`deploy::Manager` (currently already private — keep it that
  way; `Mod`/`Profile` stay public data types via model).
- Locked decision: `settings()`/`settings_mut()` **stay** as guard accessors.
  `Settings` is core data re-exported via `uk-app::model`, and in the session
  architecture all *mutation* already goes through `Command`s/drafts — the
  guards are read-mostly. Converting ~80 call sites to closures would be pure
  churn with no sealing benefit; revisit in phase 6 if the CLI rewrite wants
  otherwise.

## 3. Shippable when

1. `cargo check --workspace --all-targets` clean;
   `cargo test -p uk-app` + `cargo test -p uk-content` green.
2. `cargo tree -p uk-app` has no egui/eframe/rfd/winit (unchanged guard).
3. `cargo tree -p ukmm` (root binary) shows only `uk-app`, `uk-ui`,
   `uk-localization` as direct uk-* deps.
4. `cargo tree -p uk-content` has no `uk-localization`.
5. `uk-manager` exposes no guard-returning sub-manager accessors and no
   frontend types; its data types are consumed via `uk-app::model`.

## 4. Locked decisions

1. `Settings` guard accessors stay (see 5c); the seal targets sub-manager
   guards and internal fields.
2. The dead `Language → LocLang` conversion is deleted, not relocated —
   nothing uses it.
3. `uk-manager → uk-localization` (legacy `lang` field) stays until v1
   settings files are dropped; not part of this phase.
