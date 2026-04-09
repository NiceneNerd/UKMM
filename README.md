# UKMM: U-King Mod Manager

U-King Mod Manager is a tool for managing and merging mods for *The Legend of
Zelda: Breath of the Wild*. It should be considered a successor to
[BCML](https://github.com/NiceneNerd/BCML). 

## Why?

Because BOTW is a fun, high quality game with a powerful engine and a fairly
robust content/engine distinction, it is in many ways the perfect game to mod.
Unfortunately, since it was designed to work on underpowered consoles,
particularly the Wii U, its ROM structure is designed more for maximum
efficiency and performance than for extensibility. Features like the [Resource
Size Table](https://zeldamods.org/wiki/Resource_system#Resource_size_table) and
large archives to handle diverse content loaded in single batches are excellent
as optimizations but make modding a more brittle and conflict-prone process.
(For more information on the many kinds of mod conflicts that tend to be
ubiquitous in BOTW using ordinary game assets, see [this reference on
ZeldaMods](https://zeldamods.org/wiki/Help:Resolving_mod_conflicts). There is
also [a post about this on
GBAtemp](https://gbatemp.net/threads/dont-use-bcml-for-switch.590409/post-10030639)
which summarizes the problem fairly concisely.)

Since BOTW mods are prone to such problems, when using more than one, they must
often be processed and merged together to ensure a stable, playable experience
(and in many cases just to get the game to boot at all). UKMM offers a powerful,
robust, and highly accurate merging process to ensure the most optimal result: a
modpack that pretty much just works.

In contrast to BCML, UKMM also tried to provide a more generally satisfying mod
management experience not just narrowly focused on successful merging, drawing
inspiration in some ways from the famous [Mod Organizer
2](https://github.com/ModOrganizer2/modorganizer).

## Setup and Use

Unlike BCML, setup is pretty much just download and unzip. For more details
consult [the Book](https://nicenenerd.github.io/UKMM/).

## Building from Source

Unlike BCML, building from source is easy.

Requirements:

- Recent Rust toolchain (MSRV 1.80)
- A compiler that supports C++17
- Modern CMake (3.12+)

Nothing else special is required. Generate a release build by running `cargo
build --release` in the root repo folder. It may take a while.

## Contributing 

Issues: <https://github.com/NiceneNerd/UKMM/issues>

Contributions are always welcome. Some notes about the process:

- At present, some tests need to be setup manually, as they require you to have
  a game dump and a Wii U mod to test on. However, all tests in the `uk-content`
  package are standalone and should be pass without any setup.
- As this codebase grew is complicated ways over a period of a few years now,
  some parts might not be entirely consistent in conventions or could use
  refactoring. If you work on any part of the code that could use improvement in
  consistency or sustainability, please go ahead and do what seems right in your
  eyes.

## License 

This software is licensed under the terms of the GNU General Public License,
version 3 or later. The source is publicly available on
[GitHub](https://github.com/NiceneNerd/UKMM).

## Special Thanks

[Léo Lam](https://github.com/leoetlino): oead, many other BotW libraries  
[Anna Clemens](https://github.com/anna-is-cute): original BotW MSBT libraries  
Gray: Dutch localization  
[Nebroc](https://gamebanana.com/members/1920307): French localization  
Waikuteru: German localization  
𝘽𝙤𝙤𝙢𝙞𝙚𝙨!★: Italian localization  
[carbonatedtea](https://github.com/k-carbonatedtea): Simplified Chinese localization  
