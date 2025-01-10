# Changelog

All notable changes to UKMM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

**Added**

- Added extra context to profile initialization errors
- Multiple language localization! Currently supported languages are: English,
  Dutch, French, German, Italian, Russian, Simplified Chinese, Spanish.
  - Looking for translators to translate: Japanese, Korean

**Changed**

- When a mod is installed in the wrong mode (i.e. a WiiU mod in Switch mode or
  a Switch mod in WiiU mode) the error message will now be more straightforward
- When installing a mod with no metadata, the message will be more clear, and the
  archive name will be put in the mod name field by default

**Fixed**

- Fixed a crash that could occur if you loaded a profile that you had duplicated,
  and hadn't restarted ukmm since you'd done that duplication

## [0.15.2] - 2025-01-02

**Fixed**

- Fixed Cemu settings importer on Linux
- Fixed Copy mode deployment sometimes attempting to use file copier to copy folders
- Fixed a regression that caused bnps that added new actors to crash
- Fixed Open Emulator button not opening emulator properly on Windows

## [0.15.1] - 2024-12-01

**Added**

- Added Deploy Layout settings option, to give more control over deploy output location
- Added extra context to various BNP conversion errors
- Added option to install unpacked loose file mods via rules.txt

**Changed**

- Reworked RSTB calculation, for more efficient file sizes and fewer panic moons
- Reworked deployment, to better support Switch emulators
- Reworked deployment config validation, to ensure you don't accidentally leave
  behind a bunch of useless links or files when the effective output location changes
- Updated tooltip for deploy method, to give more clarity inside the app
- Enabled vertical scrolling in package dependency window, for long mod lists

**Fixed**

- Fixed mods that include multiple occurrences of the same option in their bnp
  throwing errors on trying to add those options to the UKMM zip
- Fixed merging CDungeon/Static.smubin, fixes warping in over a chasm and
  repeatedly dying in one of the DLC shrines
- Fixed a bug where Copy mode on Windows did not copy files
- Fixed various (but not all) errors when importing improperly-made BCML mods,
  related to mod authors adding diff logs for files with no vanilla counterpart
- Fixed various bugs related to BCML migration
- Fixed a rare bug where a mod might not be properly marked for merging
- Fixed a bug where some mods that were part of a profile could not be reinstalled
  after deleting that profile
- Fixed Cemu settings import
- Fixed a bug where mods with text changes may not apply those changes, depending
  on what language was chosen in the settings and what languages other mods contained
- Fixed a bug where a different language than the one chosen in the settings would
  be used for the final merge, when mods contained too many languages

## [0.15.0] - 2024-08-29

This release is fairly significant and includes *multiple breaking changes* to
the UKMM mod format. This means there is a *high likehood* that you will need to
reinstall and/or repackage existing UKMM mods to avoid errors. Ideally, this
should be the last breaking release before stabilizaiton.

**Added**

- Added support for new Cemu config paths
- Experimental support for arm64 macOS
- Added support for complex emulator command lines

**Changed**

- Switched to symlink deployment by default for Cemu
- Minor performance improvements

**Fixed**

- Fixed settings refresh after BCML migration
- Fixed broken symlink on switching profiles
- Fixed issues with language differences in text merging
- Fixed issues with StatusEffectList and LevelSensor mergers. **This is a
  breaking change which requires reinstalling mods which modify StatusEffectList
  or LevelSensor.**
- Reverted msyt version to fix BNP compatibility. **This is a reversion of an
  unintentional breaking change in the last release. It will require
  reinstalling mods which modify game texts.**

**Removed**

- Removed merging for actor recipes (`.brecipe`). Diffs will be stored whole and
  merging will simply overwrite with the highest priority. **This is a breaking
  change which requires reinstalling all mods which modify recipe files.**

## [0.13.0] - 2024-07-28

**Added**

- Added full GameBanana 1-click and "open mod with UKMM" support
- Added Refresh button to file picker
- Added specific error about mods made with old roead versions

**Fixed**

- Fixed crash when reopening tabs closed by their buttons
- Fixed rare race condition with mod packaging

**Removed**

- Removed everything related to the mod editor tool, which will probably never
  be finished

## [0.12.1] - 2024-07-04

**Added**

- Added `cargo dist` integration to provide simpler install and update methods

**Fixed**

- Restored missing Package button to Window menu
- Fixed option descriptions not showing on multiple choice
- Fixed misidentification of Switch BNPs with a `rules.txt` as Wii U mods


## [0.12.0]

**Added**

- Added experimental "binary override," as-is storage of technically invalid
  resources for mods which work despite minor "errors"

**Changed**

- Completely reworked logger using `egui_logger`, hopefully more performant and
  maintainable
- Switched some `Arc`s to `Rc`s where possible

## [0.11.1]

**Changed**

- Stopped caching open directory contents so the file picker is accurate after
  the app restarts

**Fixed**

- Fixed serious issues with reinflating BNPs with options
- Fixed parsing numeric strings in BCML 2.*x* `deepmerge.yml` files

## [0.11.0]

**Added**

- Added update mod button for developers
- Added GUI error message for startup panics

**Changed**

- Switched back from a custom fork of egui to the latest official version.
  This brings some minor unwanted UI changes, but nothing, I think, that
  affects anything functionally.

**Fixed**

- Fixed possible errors with missing game languages

## [0.10.1]

**Added**

- Added open folder buttons to tool menu
- Added button to open emulator per-deployment config

**Changed**

- Use dictionary for ZSTD compression

**Fixed**

- Fixed missing `AocMainField.pack` in some map mods
- Minor patches to gamedata handling

## [0.10.0]

**Added**

- Added button to extract mods back into full files (graphic pack/RomFS)
- Added mod API versioning to better handle format changes across versions

**Changed**

- **Breaking change**: Updated to the newest version of roead, which supports
  BYML versions 5-7. This means *all mods that edit BYML files* may need to be
  reinstalled, which is perhaps a majority of mods. (The good news is this will
  make TOTK support easier to add in the future.)
- Updated to work on the stable Rust compiler, nightly no longer required.

**Fixed**

- Fixed "no base or DLC content folder" on some Switch mods
- Fixed panic parsing map logs with deletions in BNPs
- Fixed weird gamedata flag issue on some BNPs
- Updated RSTB library to fix mystery panics

## [0.9.0]

**Added**

- Added "Send to Profile" option for mods

**Changed**

- **Breaking change**: Deeper model data merging. This is a *breaking change*
  which will require reinstalling all mods that edit model lists (`.bmodellist`).
- Specially flag nested lookup error
- Even more workarounds for malformed recipe/drop table files

**Fixed**
- Fix BFARC and BLARC merging new files
- Removed almost all possible panics in content merging

## [0.8.2]

**Changed**

- Workaround for mods with incorrect drop or recipe `ColumnNum` values
- Workaround for mods with incorrect drop/recipe numbered names (e.g. `ItemNum001`)
- Queue errors for end of batch install
- More error details, especially to identify mods in batch operations
- Clearer "no base version" error

**Fixed**

- Hacky fix for low Switch RSTB values

## [0.8.1]

## Fixed

- Temporarily disabled complex RSTB estimates to fix crashes
- Fixed issue with handling empty `AocMainField.pack`

## [0.8.0]

**Added**

- Added CLI package command (@ArchLeaders)
- Added support for updating mods with newer versions
- Added indicator for current platform on top menu
- Added Shift-Click mod range selection

**Changed**

- Reject keyboard input under modals
- **Breaking change**: Reworked CookData merger (@GingerAvalanche).
  This will require reinstalling any mods that modify `CookData.sbyml`.
- Improved some warnings
- Ignore `AocMainField.pack` when converting BNPs
- Tolerate bad BNP version fields

**Fixed**

- Fixed panics when iterating corrupted SARCs
- Fixed default scale on Steam Deck
- Fixed GUI flag handling
- Fixed older macOS (<12) support
- Fixed packaging the contents of handled SARCs separately
- Fixed loading nested files with WUA dump

## [0.7.1]

**Changed**

- Automatically enable new mods
- Catch panic errors on manual threads

**Fixed**

- Fixed running GUI with args
- Fixed default dock/tab settings

**Removed**

- Removed "Unpack mods" setting

## [0.7.0]

**Added**

- Added drag-and-drop installation
- Now saves tab/dock layout

**Changed**

- **Breaking Change**: Make BFARC files mergeable. All mods which edit UI game
  fonts will need to be reinstalled.
- Ignore invalid gamedata flags
- Switch to folder selector for Cemu

**Fixed**

- Fixed updater
- Fixed parsing `meta.yml` for autofill
- Fixed CLI on Windows (closes #62)

## [0.6.0] - 2023-03-10

**Added**

- Added meta autofill when packaging source contains meta file
- Added setting to control system 7z
- Added macOS release

**Changed**

- Skip copying mod on install if already stored from another profile
- More error details with `anyhow_ext`

**Fixed**

- Fixed profile corruption when uninstalling a mod used by multiple profiles
- Fixed thumbnail compression issue with unpack mods setting
- Fixed unpacked ROM optional DLC folder check
- Fixed font loading on macOS and certain Linux distros
- Experimental fix for BNPs with `UNDERRIDE_CONST` in drop logs

## [0.5.0] - 2023-03-02

**Added**

- Added mod preview image packaging. You can include a preview image in a mod by
  placing it in the root folder and naming it `thumb.jpg` or similar (all
  options listed in docs).
- Added more error details

**Changed**

- _[Breaking change]_ Made BLARCs mergeable. All mods which contain
  `Bootup.pack` will need to be repackaged/reinstalled.
- Ignore zero byte when processing mods
- Further improved mod filename sanitation
- Switched to safe error for potential issues with BNP text logs
- Updated roead for MacOS support progress
- Various UI tweaks (courtesy of ArchLeaders)

**Fixed**

- Fixed handling `required` field for BNP options
- Fixed BCML settings path on Windows
- Fixed parsing empty dump folder settings in BCML migration
- Minor fixes to logging

## [0.4.0] - 2023-02-27

**Added**

- Added "Reset Pending" option to menu
- Added experimental BCML migration tool
- Added log file and panic details

**Changed**

- Moved `rules.txt` setting to deployment config
- Improved message pack processing to report the paths of problem files and
  filter for only MSBTs (should fix Zelda's Ballad compatibility)

**Fixed**

- Exclude bootup language packs from BNP pack converter to solve cross-region
  issues
- Fixed panic on zero-length mod files
- Fixed cross-region bootup language pack deployment

## [0.3.1] - 2023-02-22

**Changed**

- Default to portable storage folder in portable mode
- Clear resource cache on settings change
- Update deps to improve binary size

**Fixed**

- Fixed missing non-US languages in nested SARC map

## [0.3.0] - 2023-02-19

**Added**

- Added experimental updater
- Added support for BCML 2.x BNPs
- Added basic cross-region language support

**Changed**

- Strip and compress release builds

## [0.2.6] - 2023-02-15

**Added**

- Added GitHub workflow for potential Steam Deck support

**Changed**

- Improve editor UI file tree
- Implement `NamedEnumerate` for slight perf boost
- Added scrolling to options selection dialog

**Fixed**

- Fixed crash from bad damage param RSTB
- Fixed missing parent folder with BNP logs
- Fixed `required` field parsing for BNP mod options
- Matching fix for certain AS nodes
- Fixed max width for busy dialog
- Fixed edit UI for `ElementParams`
- Fixed font loading on Linux

## [0.2.5] - 2023-02-11

**Added**

- More mod categories
- Install multiple mods at once

**Fixed**

- Fixed parent folder creation for BNP SARCs
- Fixed portable mode flag
- Statically link OpenSSL for Steam Deck support
- Fixed SARC inflation for BNP options
- Fixed repeated None radio for mod options

**Changed**

- Show BNPs in file browser
- Support reading UKMM ZIPs with compressed metadata
- Updated roead for more flexible param types
- More error details for some content

## [0.2.4] - 2023-02-09

**Added**

- Added welcome/changelog popup on new version

**Fixed**

- Fixed BNP SARC inflation for edge cases
- Fixed issue with optional BNP option fields
- Fixed platform detection for "graphic pack" mods
- Fixed AI program serialization
- Fixed illegitimate RSTB entries causing crashes in-game (probably fixing #23)

**Changed**

- Significant pointless changes to AS merger
- Fixed mod meta issues by switching to YAML
- Modified progress message to improve performance

## [0.2.3] - 2023-02-02

**Added**

- More docs
- Cross-platform mods (experimental)
- Initial theme support

## [0.2.2] - 2023-02-01

**Added**

- Added required setting for option groups
- Added support for converting BNP options

**Fixed**

- Fixed BNP conversion missing dump reference
- Fixed mod option descriptions
- Updated roead to fix SARC debugging
- Fixed BNP SARC inflation (partially fixes #23)

**Changed**

- Added alignment to SARC info

## [0.2.1] - 2023-01-30

**Changed**

- Fixed mod option default display
- Fixed problem with delete collection length
- Fixed BNP temp file and improved temp file cleanup
- Fixed some tooltip formatting

## [0.2.0] - 2023-01-28

**Added**

- Readme
- Support for converting and installing BNPs (no option support yet)

**Changed**

- Switched mod versions to semantic version strings
- Fixed various mod option UI limitations
