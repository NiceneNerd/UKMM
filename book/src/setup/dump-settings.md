# Dump Settings

As noted [earlier](dump.md), you need a dump of BOTW on your PC to use UKMM. (If
perchance you are curious why, [here's an explanation](../faq.md).) I'll break
down how to set this correctly per platform.

## Wii U

For Wii U, you have two supported dump options: unpacked MLC files (most common)
or a `.wua` file (Cemu-specific format).

For information on `.wua` files, check [the changelog for Cemu
v1.27.0b](https://cemu.info/changelog.html) or the [ZArchive
repo](https://github.com/Exzap/ZArchive). The rest of this guide will focus on
an unpacked dump.

### Unpacked Dump

There are three folders to specify for an unpacked game dump.

- **Base Folder**: This folder is the root of the plain, v1.0 BOTW assets which
  were included on the disk. If you are using Cemu, it will usually be in your
  MLC folder, with a path such as this (part of the title ID will be different
  for the EU or JP versions):  
  `mlc01/usr/title/00050000/101C9400/content`  

  You can verify the path is correct if it contains `Pack/Dungeon001.pack`.
- **Update Folder**: The contains the BOTW v1.5.0 update data. It is absolutely
  necessary for the game to even run. If you are using Cemu, it will usually
  have a similar path to the base folder, but with an `E` at the end of the
  first half of the title ID:  
  `mlc01/usr/title/0005000E/101C9400/content`  

  You can verify the path is correct if it contains over 7000 files in the
  `Actor/Pack` folder.
- **DLC Folder**: This contains most of the assets for the BOTW DLC. This one
  does *not* usually end in `content`, but must go one level further into a
  `0010` folder because of the way multiple kinds of add-on content are handled.
  If you are using Cemu, it will usually have a similar path to the base folder,
  but with a `C` at the end of the first half of the title ID:  
  `mlc01/usr/title/0005000C/101C9400/content/0010`

  You can verify the path is correct if it contains `Pack/AocMainField.pack`.

## Switch

At present only unpacked RomFS dumps are supported, but in the future NSP or XCI
support is planned.

### Unpacked Dump

There are three folders to specify for an unpacked game dump.

- **Base Folder**: On Switch, following the usual guides with `nxdumptool`, this
  will usually be the combined base game and v1.6.0 update files. The path will
  probably contain the title ID of `01007EF00011E000` and end in `romfs`.

  You can verify the path is correct if it contains over 7000 files in the
  `Actor/Pack` folder.
- **DLC Folder**: This contains most of the assets for the BOTW DLC. The path
  will probably contain a title ID like `01007EF00011E001` and end in `romfs`.

  You can verify the path is correct if it contains `Pack/AocMainField.pack`.
