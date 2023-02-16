# Changelog

All notable changes to UKMM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added GitHub workflow for potential Steam Deck support

### Changed

- Improve editor UI file tree
- Implement `NamedEnumerate` for slight perf boost
- Added scrolling to options selection dialog

### Fixed

- Fixed crash from bad damage param RSTB
- Fixed missing parent folder with BNP logs
- Fixed `required` field parsing for BNP mod options
- Matching fix for certain AS nodes
- Fixed max width for busy dialog
- Fixed edit UI for `ElementParams`
- Fixed font loading on Linux

## [0.2.5] - 2023-02-11

### Added

- More mod categories
- Install multiple mods at once

### Fixed

- Fixed parent folder creation for BNP SARCs
- Fixed portable mode flag
- Statically link OpenSSL for Steam Deck support
- Fixed SARC inflation for BNP options
- Fixed repeated None radio for mod options

### Changed

- Show BNPs in file browser
- Support reading UKMM ZIPs with compressed metadata
- Updated roead for more flexible param types
- More error details for some content

## [0.2.4] - 2023-02-09

### Added

- Added welcome/changelog popup on new version

### Fixed

- Fixed BNP SARC inflation for edge cases
- Fixed issue with optional BNP option fields
- Fixed platform detection for "graphic pack" mods
- Fixed AI program serialization
- Fixed illegitimate RSTB entries causing crashes in-game (probably fixing #23)

### Changed

- Significant pointless changes to AS merger
- Fixed mod meta issues by switching to YAML
- Modified progress message to improve performance

## [0.2.3] - 2023-02-02

### Added

- More docs
- Cross-platform mods (experimental)
- Initial theme support

## [0.2.2] - 2023-02-01

### Added

- Added required setting for option groups
- Added support for converting BNP options

### Fixed

- Fixed BNP conversion missing dump reference
- Fixed mod option descriptions
- Updated roead to fix SARC debugging
- Fixed BNP SARC inflation (partially fixes #23)

### Changed

- Added alignment to SARC info

## [0.2.1] - 2023-01-30

### Changed

- Fixed mod option default display
- Fixed problem with delete collection length
- Fixed BNP temp file and improved temp file cleanup
- Fixed some tooltip formatting

## [0.2.0] - 2023-01-28

### Added 

- Readme
- Support for converting and installing BNPs (no option support yet)

### Changed

- Switched mod versions to semantic version strings
- Fixed various mod option UI limitations
