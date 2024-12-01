# Installation

Once you have all your prerequisites in order, you can begin to use UKMM.

UKMM consists of a single binary with all dependencies statically linked, making
installation over 9000 times easier than BCML.

## Steps

- [Download the latest UKMM release for your operating system from
  GitHub](https://github.com/GingerAvalanche/UKMM/releases/latest)
- Extract the archive (ZIP on Windows, tar on Linux) wherever suits you
- (Linux only) You may need to set the executable bit: `chmod +x ./ukmm`
- Run the executable. You have UKMM ready to go!

## Configuration Storage

By default, UKMM will store your settings and similar data in an appropriate
configuration folder for your platform. On Windows this should be
`%APPDATA%\ukmm` and on Linux `~/.config/ukmm`.

You can alternatively launch UKMM with the `--portable` flag, in which case it
will use a `config` folder next to the UKMM executable.

## 

![See, that wasn't so hard!](../images/that-wasnt-so-hard.gif)
