# Configuration

When you first run UKMM, you will need to configure some basic settings before
you can do much of anything.

## General Settings

Settings under the General category apply to UKMM as a whole, not specifically
Wii U or Switch mode.

- **Current Mode**: Specifies whether to operate in Wii U or Switch mode. If
  using an emulator, go with the mode of your emulator's platform,[^1] so Wii U
  for Cemu or Switch for Yuzu/Ryujinx.
- **Storage Folder**: Where to store mods, profiles, mod projects, etc. Defaults
  to `~/.local/share/ukmm` on Linux or `%LOCALAPPDATA%\ukmm` on Windows. Make
  sure to change this setting if you want to store mods and merges on a
  different partition or external drive.
- **Unpack Mods**: By default UKMM stores mods as ZIP files with ZSTD
  compression. Turn on this option to unpack and decompress them instead,
  potentially improving performance at the cost of disk space.
- **Show Changelog**: Whether to show a changelog after UKMM updates. Simple
  enough, right?

## Platform-Specific Settings

Most other settings apply independently to Switch or Wii U mode. The simplest of
these is below:

- **Language**: The language and region matching your game dump and play
  settings. If you for any reason do not set this correctly, you will probably
  not see any of changes any of your mods make to in-game text (dialogue, item
  descriptions, etc.).

The rest of the platform-specific settings will be covered in more detail in
the next two sections.

---

[^1]: Should I really have to specify this? Probably not. Does *someone
somewhere* need me to? Yes.
