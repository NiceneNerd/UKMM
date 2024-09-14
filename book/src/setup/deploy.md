# Deployment Config

When you actually want to use your merged mods, they will need to be deployed.
This is the most critical step to actually using mods when you play the game.

Note that you have the option to deploy automatically or not. If you do not
deploy automatically, you can make whatever changes to your load order, and even
apply them, but still not export the final merged mod to its destination until
you're ready. This is most useful for console players, who, for example, might
want to manage mods on the PC at any time but wait until they're ready to insert
their SD card before deploying the final pack. For emulator users, it is
generally more useful to use automatic deployment.

## Deployment Locations

Where should you deploy your mods and what layout should you use? It depends
mostly on where you play your game.

### Cemu

Cemu users will generally want to deploy their mods as a graphic pack. In that
case, the best idea is to set your deployment location to Cemu's `graphicPacks`
folder and turn on the With Name option for Deploy Layout. So, for example, the
full path might be something like: `C:\Cemu\graphicPacks\`. For that example,
and with the With Name option, UKMM will actually deploy to
`C:\Cemu\graphicPacks\BreathOfTheWild_UKMM`.

**Additional note for Cemu users**: You almost certainly want the "Deploy
rules.txt" option selected for Cemu integration.

### Wii U

Wii U users have a few options, but the most widely used and supported method to
load mods is via [SDCafiine for the Wii U Plugin
System](https://zeldamods.org/wiki/Help:Using_mods#Setting_up_WUPS_SDCafiine).
In that case, you would generally want your mods to end up on your SD card under
something like `/sdcafiine/<title ID>/ukmm`. To achieve that, you could set the
folder directly and choose the Without Name option for Deploy Layout.

You could also set the deployment location to `/sdcafiine/<title ID>` and choose
the With Name option, and UKMM will add the final folder on its own.

If you use UKMM while your SD card is not in, however, you might want to set a
temporary directory for deploying mods, or you can merge without the SD card but
wait and deploy when the SD card is mounted.

### Switch

With the Switch, you generally want your mods to end up on your SD card under
`/atmosphere/contents`, and you will always want to use Without Name for the
Deploy Layout. If you use UKMM while your SD card is not in, however, you might
want to set a temporary directory for deploying mods, or you can merge without
the SD card but wait and deploy when the SD card is mounted.

### Yuzu or Ryujinx

Yuzu and Ryujinx both allow you to install mods in two different locations, one
specific to their own files and the other for emulating Atmosphere's LayeredFS
setup on SD card. You may use either arrangement, but you *must* choose the
correct Deploy Layout, or the emulator will not read the merged mod correctly.

So, for example, if you want to use Yuzu with the Atmosphere implementation, then
the Yuzu user storage folder is `C:\Users\[USER]\AppData\Roaming\yuzu` on Windows
or `~/.local/share/yuzu` on Linux. In this case, you want your deployment folder at
`[USER-FOLDER]/sdmc/atmosphere/contents` and you want your Deploy Layout set to
Without Name.

If you want to use Yuzu's specific mod loader implementation, then that will read
from `[YUZU-DIRECTORY]/load`, so you will set that as your deployment location and
set your Deploy Layout to With Name.

## Deployment Methods

UKMM offers three methods to deploy mods. Which one is best depends heavily on
your system, so I recommend taking careful note of these options and how they
work.

### Copy

The simplest option. It just copies everything from UKMM's internal merging
folder into the deployment folder.

**Advantages**
- Easy
- Pretty much always works

**Disadvantages**
- Can be very slow
- Wastes disk space

**Best for**: SD cards

### Hard Links

A safe option to save space when everything is on the same volume/partition. It
creates a hard link of every file from UKMM's internal merging folder into the
deployment folder, which uses no additional disk space. Both copies are
literally the same file.

!["Two" hard linked files are just one file with two
paths](../images/hard-link.jpg)

**Advantages**
- Pretty fast
- No wasted disk space

**Disadvantages**
- Only works if everything is on the same volume/partition
- Slower than symlinks

**Best for**: Windows systems where everything is on one volume

### Symlink

Turns the deployment folder into a mere link to the UKMM's internal merged
folder. This means deployment isn't even needed; all changes to your load order
are automatically present wherever you have your mods deployed.

Unfortunately, this is also the weirdest option on Windows. (On Linux it should
pretty much Just Work™.) Windows is weird about symbolic links. Because of this,
UKMM will first attempt to use a "directory junction," a dumb alternative to a
symbolic link which only works on internal drives. Removeable drives and
networked drives are not supported. If that fails, it will try to use a regular
directory symbolic link. These have fewer restrictions, but usually (for some
dumb reason) require administrator permissions to create.[^1]

So, in sum:

**Advantages**
- Instant, transparent deployment
- No wasted disk space

**Disadvantages**
- Windows support is complicated
- No chance to change your mind before deploying mods after applying load order
  changes

**Best for**: Linux systems, or advanced users on Windows

## Deployment Layouts

### Without Name

UKMM will not add any folders called `BreathOfTheWild_UKMM` without you telling it.
On WiiU, this means that content files will be deployed to `[Output Folder]/content`
and dlc files will be deployed to `[Output Folder]/aoc`. On Switch, this means that
content files will be deployed to `[Output Folder]/01007EF00011E000/romfs` and dlc
files will be deployed to `[Output Folder]/01007EF00011F001/romfs`.

This is useful for if you're following an old setup tutorial that tells you to put a
specific folder for your mod manager in the output path, if you're on a Switch
console, or if you're on a Switch emulator and using the atmosphere mod directory for
it.

This is how BCML and previous beta builds of UKMM always handled deployment. If
you are upgrading from BCML or an old build of UKMM and your paths already work
for you, then you can leave this as your deployment layout and it will just work.

### With Name

UKMM will add folders called `BreathOfTheWild_UKMM` to the appropriate places when
deploying. On WiiU, this means that content files will be deployed to
`[Output Folder]/BreathOfTheWild_UKMM/content` and dlc fils will be deployed to
`[Output Folder]/BreathOfTheWild_UKMM/aoc`. On Switch, this means that content files
will be deployed to `[Output Folder]/01007EF00011E000/BreathOfTheWild_UKMM/romfs`
and dlc files will be deployed to
`[Output Folder]/01007EF00011F001/BreathOfTheWild_UKMM/romfs`.

This is useful if you just want to point UKMM at your Cemu graphic pack folder or
WiiU SD Card and call it a day, or if you're using a Switch emulator and deploying
to the regular mods directory so that you can activate/deactivate mods in the
in-emulator menu.

---

[^1]: Starting back in Windows 10, build 14972, it has been possible to create
symbolic links on Windows without administrator permissions, but it's not
automatic. Check [the Windows blog
announcement](https://blogs.windows.com/windowsdeveloper/2016/12/02/symlinks-windows-10/)
for more information.
