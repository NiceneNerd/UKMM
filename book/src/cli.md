# Command Line

UKMM ships with a full command-line interface that runs on the same engine as
the graphical interface. Running `ukmm` with no arguments (or with a mod file
path or a `bcml:` one-click URL) opens the GUI; everything else is a CLI
subcommand.

```sh
ukmm <SUBCOMMAND> [ARGS] [FLAGS]
```

Global flags:

- `-j`, `--json` — emit every session event as one JSON line on stdout, plus a
  `{"snapshot": …}` line after each change (progress messages go to stderr, so
  stdout stays machine-readable).
- `-d`, `--debug` — verbose logging.
- `-p`, `--portable` — use a `config` folder next to the executable instead of
  the system config folder.
- `-D`, `--deploy` — automatically deploy after running the command.

The config folder can also be overridden with the `UKMM_CONFIG_DIR`
environment variable (useful for scripts and tests).

All mod indices are **1-based**, matching the numbering printed by `list`.

## Inspecting and installing mods

- `ukmm list` — list the mods in the current profile with their 1-based index,
  version, author, and enabled state, plus the pending file count.
- `ukmm open <path>` — inspect a mod without installing it (reports the
  platform, options, or metadata it would need). Exits non-zero if the mod
  cannot be used with the current configuration.
- `ukmm one-click <bcml://…>` — handle a GameBanana one-click URL.
- `ukmm install <path> [profile] [-m <meta file>]` — install a mod, enable it,
  and merge immediately. With a profile argument, the mod is installed directly
  into that profile (created if needed); the current profile is left untouched.
  `-m`/`--meta` supplies metadata for legacy mods.
- `ukmm install-batch <paths…> [-r <index>]` — install several mods at once,
  optionally replacing the installed mod at the given 1-based index.

## Managing mods

Each of these stages the change and merges it in the same invocation, so the
change is saved even if you never run `apply` yourself:

- `ukmm enable <indices…>` / `ukmm disable <indices…>`
- `ukmm options <index> [paths…]` — with no paths, lists the mod's option
  groups, their option paths, and the current selection; with paths, sets the
  selection (one path per option group; unknown paths are rejected with a list
  of the valid ones).
- `ukmm move <index> <to>` — move a mod to a new 1-based load-order position.
- `ukmm uninstall [index] [profile]` — uninstall a mod (omit the index to pick
  interactively).
- `ukmm add-to-profile <profile> <indices…>` — copy mods into another profile.
- `ukmm update-from <index> <path> [-m <meta file>]` — replace an installed mod
  with a newer archive, merging in place.

## Merging and deploying

- `ukmm apply` — merge the pending changes.
- `ukmm cancel-pending` — discard the pending changes.
- `ukmm remerge` — rebuild the merged output from scratch.
- `ukmm reset-pending` — reset the pending merge to the last deployed state.
- `ukmm deploy` — copy the merged output to the configured deploy folder.

## Profiles

- `ukmm profiles` — list profiles and mark the current one.
- `ukmm profile-switch <name>`, `profile-create <name>`,
  `profile-duplicate <from> <to>`, `profile-rename <from> <to>`,
  `profile-delete <name>`.

## Settings and configuration

- `ukmm reload` — re-read settings from disk.
- `ukmm import-cemu` — import settings from a Cemu `settings.xml`.
- `ukmm migrate-bcml` — migrate a BCML installation.
- `ukmm save-settings` / `ukmm mode <wiiu|switch>` — change the current mode;
  platform settings are edited in the GUI and saved via its Save button.

## Packaging and development

- `ukmm package <source dir> <output archive> [-m <meta file>]` — package a mod.
- `ukmm parse-meta <meta file>` — parse and print a meta file.
- `ukmm dev-update <indices…> <folder>` — repack installed mods in place from a
  loose folder.
- `ukmm extract <dest> [indices…]` — extract mods to a folder.

## Updates and launching

- `ukmm check-updates [--install]` — check for UKMM updates.
- `ukmm launch-emulator` — launch the configured emulator.
- `ukmm open-folder <kind>` / `ukmm open-mod-folder <index>` — open a folder in
  the OS file manager.
- `ukmm register-url` — register the `bcml:` URL handler.

## Exit codes

- `0` — success (also for the GUI/one-click launch paths).
- `1` — the command ran but failed (e.g. a mod cannot be installed with the
  current configuration).
- `2` — malformed invocation (unknown subcommand or bad flags); the parse
  error is printed and the GUI is *not* started.

## JSON output

With `--json`, stdout is newline-delimited JSON: one `{"event": …}` object per
session event and one `{"snapshot": …}` object after each revision. Human
progress messages are printed to stderr instead. Example:

```sh
$ ukmm --json install my-mod.zip
{"event":{"Inspection":{"Ready":{…}}}}
{"event":{"Installed":{"id":…}}}
{"snapshot":{…}}
{"event":{"JobDone":"Ok"}}
```
