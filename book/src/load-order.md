# Load Order

When mods make conflicting changes, they will be applied in a designated order
of priority. Priority starts at 0. Priority 0 is the lowest priority, and
anything with a *higher* priority will *overwrite* it where necessary. 

***To repeat**: higher priority mods overwrite lower priority mods.*

Here are some general tips about load order:
- In general, skins should be higher than edits to behaviour or stats, at least
  for the same actors. For example: the Linkle mod should be higher than a mod
  which edits armour stats, otherwise you could have texture bugs.
- Optional components, addons, compatibility patches, or any mods that are based
  on other mods should *always* be given higher priority than the mods they're
  based on.
- Large overhaul-type mods (e.g. Second Wind or Relics of the Past) are
  complicated. When possible, they should take lower priority than other mods,
  functioning like an extension of the base game. They may, however, sometimes
  need to take priority over some or most other mods if more complex features
  (like some of those in Survival of the Wild) are not working properly.
- Any time you experience crashing or odd glitches, it can be worth it to try
  rearranging your load order.

Apart from these basic guidelines, the merging process tends to make load order
a fairly forgiving system. For the most part, load order will only matter if you
have two mods that make obviously incompatible edits, in which case it's often
as simple as just making sure you have the mod with the preferred behavior set
higher in priority.
