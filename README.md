# linux-wallpaperengine Daemon
# This project is still working in progress...
[linux-wallpaperengine](https://github.com/Almamu/linux-wallpaperengine) is an awesome project that provides Wallpaper Engine functionality on linux.   
However, that project is more like implementation of the "core", which means there are no playlist support, etc.   

This project aims to provide a daemon that runs in the background and summons `linux-wallpaperengine` periodically according to a "playlist" file.   

# Playlist file
The playlist file is a plain text file that records what wallpaper to show, in what ways and for how long.   
Playlist files usually resides in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd` and have the extension `.playlist`.   
If both "<filename>" and "<filename>.playlist" are found in the search path, the program prefers <filename>.playlist.   

`#` may be used to comment in the file. Empty lines will be ignored.   

In the beginning of the file, you may use this command:   
```
monitor <monitor id>
```
to indicate which monitor this playlist is going to play on.   
If you didn't indicate this, the wallpapers will play on all monitors.   

Define a wallpaper in this format:   
```
<wallpaper id> [duration] [property1=value] [property2=value] ...
```
`<wallpaper id>` must be specified, which is exactly what you will specify as in `linux-wallpaperengine`'s cmdline. View this id from Steam Workshop URL or anywhere you wish.   

`[duration]` indicates how long this wallpaper should be displayed. Plain numbers will be treated as seconds, you may also use a value plus an unit such as `15m`, `1h`. If duration is not specified, it is equivalent as `forever`, which displays the wallpaper until the daemon quits. Of course this is essentially the end of the playlist.   

TODO: Properties

There are also some other commands to use in the file:
- `wait <duration>` pauses the daemon's activity for the given duration. Note that when the daemon is displaying a wallpaper, it will not execute the next command until the end of the current display time ends. So use this when you want some space between wallpapers where `linux-wallpaperengine` will **not** be running.   
- `end` makes the daemon quit immediately. It's possible to write lines after `end`, if you feel boring.   
- `loop` makes the daemon go back to the beginning of the playlist file and do it over again and again. It's a short form of `goto 1 0`
- `goto <location> [times]` jumps to a specific line of the playlist and executes the command on that line. `<location>` of course is the line number starting from 1. `[times]` indicates how many times this `goto` sentence may have effect, so `goto 0 1` makes the playlist play over again, but will not go back again because of this `goto`, `0` indicates this `goto` sentence have permanent effect.   
- `replace <playlist file>` changes the playlist the daemon is currently playing, starting from line 1 of the given playlist. If the given playlist cannot be found, then this line will be similar to `end`.   
- `summon <playlist file>` is similar to `replace`, but instead of changing the playlist, it opens a new thread running the indicated playlist. So if the given playlist failed or quitted, the original thread is unaffected.   

If any sentence contains invalid value, or has bad syntax, the daemon will skip that line and generate a warning only.   
If `linux-wallpaperengine` reports an error, the daemon will forward the error and exit.   
TODO: Probably code a `lxwengctl` to validate syntax

When the daemon reaches the end of the playlist file, it by default returns to the beginning as if there's a `loop`.   

# Usage
You must have a working installation of `linux-wallpaperengine` on your system.   
If the `linux-wallpaperengine` command is not in your $PATH, you should use the `--binary` argument to identify the path to the binary.   
If you need to pass extra arguments to the `linux-wallpaperengine` binary, you may use the `--extra-args` argument. These arguments will be placed in the beginning of the argument sequence (If you don't know yet, sequence of arguments matter in `linux-wallpaperengine`).

```
$ lxwengd [-p | --playlist playlist name]
```

The daemon by default searches for playlist files in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd`.   
For other files not in the default search path, you may use the full path with the `--playlist` argument.   

It expects playlist files to have the extension `.playlist`, for which case you can just use the filename without extension. For any other extensions, use the full filename with extension.   
When invoked with no arguments, it searches for `default.playlist` and will fail if it didn't find any.   

# Troubleshooting
Any issues, feature requests or pull requests are welcomed!   

Despite its name, LxWEngd does not handle any graphical work on its own, it merely calls `linux-wallpaperengine` in the background.   

If you encounter such issues, you may use the `--dry-run` flag and extract the command LxWEngd executed on the problematic wallpaper and report to `linux-wallpaperengine`.   

Also, it does not check whether there's another wallpaper daemon running, even another instance of itself. This may occur when you wrote many playlists to run simultaneously but there's a clash between wallpapers or you forgot to set the `monitor` property correctly. If there are many wallpapers running simultaneously, the situation will be dependent to you window manager or anything similar...

## linux-wallpaperengine is missing assets (CAssetLoadException)
This may happen if your Steam installation (or anything that contains the assets directory `linux-wallpaperengine` requires) is not standard.   
You might need to pass the path to the **assets directory** explicitly.   
