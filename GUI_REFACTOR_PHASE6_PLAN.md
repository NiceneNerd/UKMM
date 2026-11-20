# UKMM refactor — detailed plan for phase 6 (prove interoperability)

> Working plan for phase 6, kept in the repo on branch `gui2`. Phase 6 proves
> the application layer is a real interoperability seam: the CLI becomes a
> second, toolkit-free frontend over `uk-app`'s `Session`, with `--json` wire
> output and a test that forces the CLI to cover **every** `Command` variant.

Companion to `GUI_REFACTOR.md` (§4 phase 6, §6 testing/CI, §7 risks).
Base: HEAD after phase 5 (`d17c875`).

Phase 6 scope from `GUI_REFACTOR.md` §4:

- Rewrite the CLI on `Session`, add `--json` (serde `Event`/`Snapshot` output), and require the CLI to cover **every** `Command` variant — that coverage is the architecture test.
- The other half of the test: a frontend crate whose `Cargo.toml` lists only `uk-app` (+ toolkit + `uk-localization`). For the CLI that means *no* toolkit at all.
- **Shippable when:** `ukmm --json install …` etc. work end-to-end against a fixture config in CI.

## 1. Design decisions (locked before starting)

- **The CLI moves to its own crate `crates/uk-cli`** (deps: `uk-app`, `xflags`, `log`, `env_logger`, `anyhow_ext`, `fs-err`, `serde_json`, `serde_yaml`). This is what makes the "frontend with only `uk-app`" claim CI-checkable: `cargo tree -p uk-cli` must not contain `egui|eframe|rfd|winit|uk-ui`. The root binary keeps `uk-app`, `uk-cli`, `uk-ui`, `uk-localization` and only dispatches (`uk_cli::run(Ukmm::from_env()?)`).
- **`--json` is the machine interface.** Every `Event` is one JSON line (`{"event":…}`), a `{"snapshot":…}` line follows each `Revision`, human-readable progress goes to stderr in JSON mode, and job errors surface as `{"event":{"JobDone":…,"result":{"Err":…}}}}` plus exit 1. `Event`/`Snapshot`/`AppError` already derive serde (§7 decision: stringify for the wire), so no new wire work was needed.
- **The CLI is non-interactive for prompts:** `Inspection` events are auto-resolved from flags/defaults (`Ready`→`Install`, `NeedsOptions`→`resolve(defaults)`, `NeedsMeta`→re-`Open` with `--meta` or a synthesized `Meta`, `WrongPlatform`/`MissingMasters`→exit 1). Interactive picking stays a GUI thing (except the pre-existing interactive `uninstall` list).
- **Coverage mechanism:** `Command::variant_name()` is an exhaustive `match` (a new variant is a compile error until named), and `Command::variant_names()` feeds a unit test in uk-app that constructs one instance of every variant and serde-round-trips it. In uk-cli, `COMMAND_COVERAGE` maps each variant to a subcommand; `crates/uk-cli/tests/coverage.rs` asserts the table covers `variant_names()` exactly. `InstallCandidate`'s fields are `pub(crate)`, so coverage is enforced via the name table rather than by constructing commands outside uk-app.
- **Fixture config in CI:** `Settings::config_dir()` honors `UKMM_CONFIG_DIR` (precedence: env > `--portable` > dirs2) so a spawned `ukmm` never touches the real config. The smoke test (`tests/cli.rs` in the root package, `env!("CARGO_BIN_EXE_ukmm")`) builds a tempdir fixture: settings written through the `Settings` API pointing at a synthetic unpacked dump, plus generated mod zips from the shared `uk_app::testkit` (`fake_dump`, `mod_zip_named` — extracted from the e2e test so unit, e2e, and CLI tests share them).
- **Merge/deploy against a synthetic dump is not asserted in CI** (needs real game files); that stays gated on `UKMM_TEST_DUMP` in `crates/uk-app/tests/e2e.rs`. The CLI smoke test asserts session-level behavior: install (incl. into a second profile), list, enable/disable, move, profile lifecycle, uninstall, mode, package, parse-meta, open, and a no-dump error path — checking JSONL shape, exit codes, and the resulting `profile.yml`/`settings.yml`.

## 2. Increments

- **6a — `uk-cli` crate + Session rewrite of the existing subcommands** (`ad129ea`): `mode`, `install`, `package`, `uninstall`, `remerge`, `deploy` re-expressed as `Command` dispatches through a `run_job`-style helper; install now enables via `SetEnabled` and merges via `Apply` (matching the old direct-core behavior). Fixes the pre-existing interactive-`uninstall` manifest shadowing (the session's dirty tracking replaces manual manifest collection). CI guard for `cargo tree -p uk-cli` in `pr.yml` + `rust.yml`.
- **6b — `--json` + fixture smoke test** (`91a2057`): global `-j/--json`; `variant_name`/`variant_names` + round-trip test in uk-app; `UKMM_CONFIG_DIR`; `uk_app::testkit`; `tests/cli.rs` (3 tests) wired into `rust.yml`.
- **6c — full `Command` coverage** (`ea119a8`): subcommands for every remaining variant (`list`, `open`, `one-click`, `install-batch`, `enable`/`disable`, `options`, `move`, `apply`, `cancel-pending`, `reset-pending`, `add-to-profile`, `profiles`, `profile-switch/-create/-duplicate/-rename/-delete`, `update-from`, `dev-update`, `extract`, `reload`, `import-cemu`, `migrate-bcml`, `parse-meta`, `check-updates --yes`, `launch-emulator`, `open-folder`, `open-mod-folder`, `register-url`, `save-settings`); `COMMAND_COVERAGE` + `crates/uk-cli/tests/coverage.rs`; `profile-switch` persists via `SaveSettings` (the command alone is in-memory); `tests/cli.rs` grows to 5 tests.
- **6d — docs + changelog + final verification** (this commit): this file, `GUI_REFACTOR.md` status/phase-6 heading, `GUI_REF.md` §23.7 note, `CHANGELOG.md` `[Unreleased]`.

## 3. Notes & deviations

- Mods are addressed by 1-based load-order index (as printed by `list`), matching the old interactive uninstall; `--profile` on `install`/`uninstall` is a positional (the `-p` short flag belongs to `--portable`).
- One-click IPC listener stays in the root `main.rs` GUI path; the `one-click` subcommand calls `uk_app::host::oneclick::oneclick` directly.
- Pre-existing quirk observed but not changed: an xflags parse error falls into the GUI branch in `main.rs` (silent exit 0 on a headless machine). Out of scope for phase 6; worth a follow-up.

## 4. Outcome (2026-11-19)

- `uk-cli` depends only on `uk-app` + non-GUI crates; both `cargo tree` guards (uk-app, uk-cli) are green in CI (`pr.yml`, `rust.yml`).
- `ukmm --json install/list/enable/move/profile-*/uninstall/mode/package/parse-meta/open` run end-to-end against a fixture config in `tests/cli.rs` (5 tests); `crates/uk-cli/tests/coverage.rs` enforces full `Command` coverage; uk-app's round-trip test proves every variant survives serde.
- Verification at push: `cargo check --workspace --all-targets` clean, `cargo test -p uk-app -p uk-content -p uk-cli` + `cargo test --test cli` green (100 + 193 + 1 + 5), `cargo +nightly fmt --check` clean.
- Phase 6 complete: the application layer is now demonstrably frontend-agnostic — the egui GUI and the CLI are two thin frontends over the same `Session`/`Command`/`Event`/`Snapshot` contract.
