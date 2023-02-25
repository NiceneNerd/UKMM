# Mod Format

UKMM stores mods in its own, highly efficient and compressed format. For
technical details, see the final section of this page.

## Format for Development

As a mod developer, you don't need to know anything about the internal format.
Rather, you can continue to create mods with the traditional graphic pack/RomFS
structure, with extensions for mod options, detailed below. There are no 
differences between this format between BCML and UKMM.

**Wii U**

```
.
├── content
├── aoc (optional: for DLC files)
└── options (optional: for optional mod components)
    ├── option1 (any name allowed)
    │   ├── content
    │   └── aoc
    ├── option2
    │   └── content
    └── ...
```

**Switch**

```
.
├── 01007EF00011E000
│   └── romfs
├── 01007EF00011F001 (optional: for DLC files)
│   └── romfs
└── options (optional: for optional mod components)
    ├── option1 (any name allowed)
    │   ├── 01007EF00011E000
    │   │   └── romfs
    │   └── 01007EF00011F001
    │       └── romfs
    ├── option2
    │   └── 01007EF00011E000
    │       └── romfs
    └── ...
```

## Dependencies and Options

You can specify any number of other mods as dependencies for your mod. If the
user attempts to install without the necessary mod(s), UKMM will throw an error.

You can also specify optional components for your mod. To add mod options, first
create an "options" folder in the mod root. Then make subfolders for each option
you want to add. In each subfolder, you will need to replicate a normal mod
structure, but containing only files different from the main mod.

Options are placed in groups, offering either multiple or exclusive choice. 
While there are no requirements about how multiple-choice options are grouped,
for exclusive choice, only one option in that group can be selected.

## Cross-platform Mods

UKMM has limited support for mods that work with both the Wii U and Switch
versions of the game. This is possible if and only if the mod consists solely of
mergeable assets. While I cannot easily provide a complete list of mergeable
assets (other than by referring you to the source code), in general this most
commonly excludes models, textures, audio, and Havok physics. To create a
cross-platform mod, check the "Mark as cross-platform" option in the mod
packaging view.

## Internal Format Details

UKMM mods are packaged in ordinary ZIP files. The contents include mod metadata,
a manifest of modified files, and UKMM-processed resources stored at their
[canonical resource paths](https://zeldamods.org/wiki/Canonical_resource_path).
An example contents in this format is below:

```
.
├── Actor
│   └── ActorInfo.product.byml
├── Map
│   └── MainField
│       └── Static.mubin
├── manifest.yml
└── meta.yml
```

### Compression

No ZIP-wide compression is used. The manifest and meta files are stored without
compression, whereas mod files are compressed with `zstd`. This makes it quick
and easy to parse mod information while nonetheless storing the real contents
with an optimal balance of size and decompresison performance.

### Meta File

Mod metadata is stored in the YAML format under `meta.yml` in the ZIP root. It
contains the mod name, description, option information, etc. Example contents:

```yaml
name: Test Mod
version: 1.0.0
author: Nicene Nerd
category: Other
description: A sample UKMM mod
platform: !Specific Wii U
url: null
option_groups = []
masters = {}
```

### Manifest File

A manifest of all real files (as opposed to canonical resources) included in the
mod is stored in YAML format under `manifest.yml` in the ZIP root. It contains
separately a list of each base game file and each DLC file. Example contents:

```yaml
content:
- Actor/ActorInfo.product.sbyml
- Actor/Pack/AncientBallSwitch2C.sbactorpack
aoc:
- Map/CDungeon/Static.smubin
- Pack/AocMainField.pack
```

### Resources

All modified files, included nested files stored in SARCs, are stored at their
canonical resource paths. **Special note**: *These are not stored as ordinary
game files in their original formats.* Rather, UKMM parses most files into
diffable, mergeable data structures representing their semantic content, and
then stores only the diffs, serialized to [CBOR](https://cbor.io/) using
[`minicbor-ser`](https://crates.io/crates/minicbor-ser).

Even files which UKMM cannot parse and merge are still stored with CBOR metadata
and thus cannot be used in the game as-is.

### Mod Options

Each option is stored in an `options` folder roughly the same layout as it is in
the pre-packaging development format, but each option includes its own manifest
and canonical resources.
