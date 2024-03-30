# Troubleshooting

Here are the most general rules for troubleshooting:

- Confirm your settings are valid, especially your game dump.
- If a problem happens when installing a mod, check whether it happens to other
  mods and, if so, whether it is all of them or if there seems to be something
  they have in common.
- Read over all the docs that look remotely relevant to your problem.
- If you need help, there are two main places to go:
    - If you think the problem is probably with your own settings, a specific
      mod or mods, or anything else that could be solved without patching UKMM,
      go to [my Discord server](https://discord.gg/y7VJqMB329).
    - If you think the problem is probably with UKMM itself and would require
      changes to the code, [file an issue on
      GitHub](https://github.com/NiceneNerd/UKMM/issues/new/choose).
- In case the program crashes completely, run it from a terminal/Command Prompt
  and check for panic output.

Solutions to some known problems follow:

## The UI is scaled badly and unusable.

This happens on some systems, *particularly Steam Deck*, for unknown reasons,
but can be fixed by setting the environment variable
`WINIT_X11_SCALE_FACTOR` to `1.0`. If you launch UKMM from the terminal, running
it as `WINIT_X11_SCALE_FACTOR=1.0 ukmm` from the UKMM folder will work.
Otherwise, you may try setting it in your `~/.profile` or `~/.Xprofile`, e.g.
by adding the line `export WINIT_X11_SCALE_FACTOR=1.0`. If perchance you run
UKMM from Steam for some reason, you can set the launch options as
`WINIT_X11_SCALE_FACTOR=1.0 %command%`.

Also note that some desktop environments contain tools for setting environment
variables. If you need help with this, ArchWiki has an [excellent article on
the topic](https://wiki.archlinux.org/title/environment_variables).

## "No config for current platform"

If you see this error, it means that, for some reason, you skipped the initial
setup where you configure all your settings for your game. Go back to [this 
page](setup/configuration.md) and start there.
