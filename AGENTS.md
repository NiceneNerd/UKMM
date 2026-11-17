# AGENTS.md — U-King Mod Manager (UKMM)

UKMM is a mod manager and merger for *The Legend of Zelda: Breath of the Wild*
(Wii U and Switch), successor to BCML. Licensed **GPL-3.0-or-later**; source on
GitHub at `NiceneNerd/UKMM`. User documentation lives in `book/` (mdBook, published
to <https://nicenenerd.github.io/UKMM>) — update it when user-facing behavior
changes, and `CHANGELOG.md` on releases.

## Repository layout

Cargo workspace. Root package `ukmm` is the binary (edition 2024, `rust-version`
1.85 in `Cargo.toml` — authoritative over the README's older MSRV note).

```
src/            main.rs (entry: CLI vs GUI dispatch, allocator, IPC), cli.rs (xflags CLI),
                gui/ (the egui frontend), logger.rs
crates/
  uk-content    Pure game-file domain (actors, events, messages, RSTB…). Standalone tests.
  uk-content-derive
  uk-mod        Mod format: Meta, Manifest, pack/unpack, zsdic dictionary.
  uk-reader     ResourceReader: unpacked dumps and .wua (zarchive).
  uk-manager    core::Manager facade; mods, deploy/merge, settings, BNP/legacy conversion.
  uk-util       Shared helpers.
  uk-localization  LocLang + string tables; en.json is canonical.
  uk-ui         egui helpers: icons, themes/visuals, ext, PathNode, syntect. Re-exports
                egui/egui_dock/egui_extras so everyone uses one egui version.
  uk-app        Frontend-agnostic operations on top of uk-manager.
```

**Layering rule:** the dependency direction is
`uk-content → uk-mod / uk-reader → uk-manager → uk-app`, with `uk-util`,
`uk-localization` as side crates. Only the root binary and `uk-ui` may depend on
egui/eframe/rfd/winit. Keep it that way: no GUI toolkit or dialog library may
appear in `cargo tree -p uk-app` or any lower crate.

## Building

Requirements: recent Rust toolchain, a C++17 compiler, CMake 4+ (C dependencies
such as roead/rstb build via cmake). On Linux install `libgtk-3-dev` (eframe);
on Windows, NASM. Then:

```sh
cargo build --release      # from repo root
```

CI builds with `cargo +nightly build` and runs `cargo +nightly test -p uk-content`;
PRs also get clippy via clippy-check. Releases are built with cargo-dist (see
`[workspace.metadata.dist]`), Windows MSI via WiX (`wix/`, `[package.metadata.wix]`).

## Formatting — nightly rustfmt only

`rustfmt.toml` uses **unstable features**, so always format with nightly:

```sh
cargo +nightly fmt --all
```

Key rules from `rustfmt.toml` (max width 100 by default):

- `group_imports = "StdExternalCrate"` + `imports_granularity = "Crate"` +
  `reorder_imports`: one `use` per crate, grouped std / external / crate.
- `struct_field_align_threshold = 4`: align struct field names when a struct has
  enough fields (see existing structs, e.g. in `uk-manager`).
- `use_field_init_shorthand`, `use_try_shorthand`: use `Foo { x }` and `?`.
- `wrap_comments` + `comment_width = 100`, `format_code_in_doc_comments`,
  `format_strings`, `normalize_doc_attributes`.
- `force_multiline_blocks`, `overflow_delimited_expr`, `condense_wildcard_suffixes`,
  `reorder_impl_items`, `version = "Two"` (edition 2024 idioms).

Do not reformat files you didn't touch; keep diffs minimal.

## Lints and code conventions

- Library crates start with `#![deny(clippy::unwrap_used)]` (uk-content, uk-manager).
  **No `unwrap`/`expect`/`panic!` in library code** — use `anyhow`/`anyhow_ext`
  with `.context(...)`, or `thiserror` for typed errors (see `uk-content`). If a
  spot truly can't avoid it, scope an `#[allow]` to the item, don't remove the
  crate-level deny. The GUI (`src/`) is more relaxed, but prefer `Result` there too.
- Errors: `anyhow_ext::{Context, Result}` in most crates; attach context at every
  layer. Logging via `log` macros — the in-app log only shows targets starting
  with `uk`, so use them.
- Locks: `parking_lot` (not std). Maps: `rustc_hash` `FxHashMap`, `dashmap` where
  concurrent. Strings: `smartstring` in hot paths. Filesystem: `fs_err as fs`.
- Serde everywhere for persisted formats; keep existing on-disk formats
  (`settings.yml`, `profile.yml`, `pending.yml`, `meta.yml`, `ui.json`) compatible.
- `Settings::config_dir()` is a process-global (dirs2 config dir, or
  `<exe>/config` with `--portable`); tests must never touch the real config —
  use tempdirs.
- Localization: add new UI strings to `crates/uk-localization/localization/en.json`
  and use `.localize()` (`LocString`); other languages fall back to English.

## Testing

- `cargo test -p uk-content` must pass with no setup (CI runs exactly this).
- Some integration tests need a real game dump and/or a Wii U mod and are set up
  manually (see README). Anything new you add should be self-contained: generate
  fixtures (zips, temp dirs) in the test, don't depend on the developer's machine.

## Git and PRs

- Commit messages are short, informal, imperative ("Fix RSTB stuff", "Update
  deps"). No conventional-commit enforcement. `Cargo.lock` is committed — update
  it when dependencies change.
- The codebase has accreted inconsistencies over the years: when you work in an
  area, feel free to improve consistency there, but keep unrelated churn out of
  the PR.

## Platform notes

- `main.rs` sets `mimalloc` as global allocator and, on Windows,
  `windows_subsystem = "windows"` plus `AttachConsole`/`MessageBoxW` shims.
- The `bcml:` URL scheme registration (`winreg` on Windows, `.desktop` file +
  `xdg-mime` on Linux) and the GameBanana one-click IPC (ssilide, port 6666)
  live in the binary crate.
- Windows builds embed `assets/ukmm.ico` via winres (build.rs).
