# linux-wallpaperengine Daemon

# This project is WIP and broken right now, until most parts are finished it's not functional.

[linux-wallpaperengine](https://github.com/Almamu/linux-wallpaperengine) is an awesome project that provides Wallpaper Engine functionality on linux.
However, that project is more like implementation of the "core", which means there are no playlist support, etc.

This project aims to provide:
- `lxwengd`, that runs in the background and summons `linux-wallpaperengine` periodically according to "playlist" files.
- `lxwengctl`, the CLI to interact with `lxwengd`.

# Playlists

The playlist file is a plain text file that records what wallpaper to show, in what ways and for how long.
Playlist files usually resides in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd` and have the extension `.playlist`.
If both "<filename>" and "<filename>.playlist" are found in the search path, the program prefers <filename>.playlist.

`#` may be used to comment in the file. Empty lines will also be ignored.

Define a wallpaper in this format:
```
<wallpaper> <duration> [property1=value] [property2=value] ...
```

`<wallpaper>` must be specified, which is exactly what you will specify as in `linux-wallpaperengine`'s cmdline.
This could be a Steam Workshop ID or a path to the background folder.

`<duration>` must be specified, indicates how long this wallpaper should be displayed.
Plain numbers will be treated as seconds, you may also use a value plus an unit such as `15m`, `1h`,
or `infinite`, which displays the wallpaper until updated by `lxwengctl`.

Note that `infinite` does not necessarily means the end of playlist,
if the child process failed or killed externally, the daemon will forward to next command.

A list of properties may be passed using key=value pairs.
`lxwengd` will simply pass them as-is to `linux-wallpaperengine`.
Here's a list of general properties:
- audio=\[true | false\], corresponding to `--no-audio-processing` in `linux-wallpaperengine`.
- volume=\[volume\], corresponding to `--volume`.
- automute=\[true | false\], corresponding to `--automute`.
- window=\[geometry\], corresponding to `--window`.
- fps=\[max fps\], corresponding to `--fps`.
- fullscreen-pause=\[true | false\], corresponding to `--no-fullscreen-pause`.
- mouse=\[true | false\], corresponding to `--disable-mouse`.
- scaling=\[stretch | fit | fill | default\], corresponding to `--scaling`.
- clamping=\[clamp | border | repeat\], corresponding to `--clamping`.

See `linux-wallpaperengine --list-properties <wallpaper id>` for a list of other properties specific to wallpapers.

`default` command may be used to set up default properties of wallpapers.
```
default [property1=value] [property2=value] ...
```

Each `default` will clear previously defined default properties, so you cannot write
```
default a=b
default c=d
```
instead of `default a=b c=d`.

There are some other commands to use in the file:
- `sleep <duration>`
- `end`

When the daemon reaches the end of the playlist file, it by default returns to the beginning.

# Usage

You must have a working installation of `linux-wallpaperengine` on your system.
If the `linux-wallpaperengine` command is not in your $PATH, you should use the `--binary` argument to identify the path to the binary.

```
$ lxwengd
```

The daemon by default searches for playlist files in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd`.
Use `--standby` to cancel this action.

# Troubleshooting

Any issues, feature requests or pull requests are welcomed!
