# Deployment Config

When you actually want to use your merged mods, they will need to be deployed.
This is the most critical step to actually using mods when you play the game.

## Deployment Locations

Where should you deploy your mods? It depends mostly on where you play your
game.

### Cemu

Cemu users will generally want to deploy their mods as a graphic pack. In that
case you will need to set your deployment location somewhere inside Cemu's
`graphicPacks` folder. A customary option is a new folder named
`BreathOfTheWild_UKMM`. So, for example, the full path might be something like:
`C:\Cemu\graphicPacks\BreathOfTheWild_UKMM`.

### Wii U

Wii U users have a few options, but the most widely used and supported method to
load mods is via [SDCafiine for the Wii U Plugin
System](https://zeldamods.org/wiki/Help:Using_mods#Setting_up_WUPS_SDCafiine).
In that case you would generally want your mods to end up on your SD card under
something like `/sdcafiine/<title ID>/ukmm`.  If you use UKMM while your SD card
is not in, however, you might want to set a temporary directory for deploying
mods, or you can merge without the SD card but wait and deploy when the SD card
is mountained.

### Switch

With the Switch, you generally want your mods to end up on your SD card under
`/atmosphere/contents`. If you use UKMM while your SD card is not in, however,
you might want to set a temporary directory for deploying mods, or you can merge
without the SD card but wait and deploy when the SD card is mountained.

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

---

[^1] Starting back in Windows 10, build 14972, it has been possible to create
symbolic links on Windows without administrator permissions, but it's not
automatic. Check [the Windows blog
announcement](https://blogs.windows.com/windowsdeveloper/2016/12/02/symlinks-windows-10/)
for more information.