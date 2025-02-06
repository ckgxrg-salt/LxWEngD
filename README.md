# linux-wallpaperengine Daemon
[linux-wallpaperengine](https://github.com/Almamu/linux-wallpaperengine) is an awesome project that provides Wallpaper Engine functionality on linux.   
However, that project is more like implementation of the "core", which means there are no playlist support, etc.   

This project aims to provide a daemon that runs in the background and summons `linux-wallpaperengine` periodically according to a "playlist" file.   

# Playlist file
The playlist file is a plain text file that records what wallpaper to show, in what ways and for how long.   
Playlist files usually resides in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd` and have the extension `.playlist`.   

`#` may be used to comment in the file. Empty lines will be ignored.   

Each line of the file should be in this format:   

```
<wallpaper id> [duration] [property1=value] [property2=value] ...
```
`<wallpaper id>` must be specified, which is exactly what you will specify as in `linux-wallpaperengine`'s cmdline. View this id from Steam Workshop URL or anywhere you wish.   

`[duration]` indicates how long this wallpaper should be displayed. Plain numbers will be treated as seconds, you may also use a value plus an unit such as `15m`, `1h`. If duration is not specified, it is equivalent as `forever`, which displays the wallpaper until the daemon quits. Of course this is essentially the end of the playlist.   

TODO: Properties

There are also some other commands to use in the file:
- `wait <duration>` pauses the daemon's activity for the given duration. Note that when the daemon is displaying a wallpaper, it will not execute the next command until the end of the current display time ends. So use this when you want some space between wallpapers where `linux-wallpaperengine` will **not** be running.   
- `end` makes the daemon quit immediately. It's possible to write lines after `end`, if you feel boring.   
- `loop` makes the daemon go back to the beginning of the playlist file and do it over again and again. It's a short form of `goto 1 inf`
- `goto <location> [times]` jumps to a specific line of the playlist and executes the command on that line. `<location>` of course is the line number starting from 1. `[times]` indicates how many times this `goto` sentence may have effect, so `goto 0 1` makes the playlist play over again, but will not go back again because of this `goto`, `inf` and `0` indicates this `goto` sentence have permanent effect.   

If any sentence contains invalid value, or has bad syntax, the daemon will skip that line and generate a warning only.   
If `linux-wallpaperengine` reports an error, the daemon will forward the error and exit.   
TODO: Probably code a `lxwengctl` to validate syntax

When the daemon reaches the end of the playlist file, it by default returns to the beginning as if there's a `loop`.   

# Usage
```
$ lxwengd [-p | --playlist playlist name]
```

The daemon by default searches for playlist files in `$XDG_CONFIG_HOME/lxwengd` or `$HOME/.config/lxwengd`.   
For other files not in the default search path, you may use the full path with the `--playlist` argument.   

It expects playlist files to have the extension `.playlist`, for which case you can just use the filename without extension. For any other extensions, use the full filename with extension.   
When invoked with no arguments, it searches for `default.playlist` and will fail if it didn't find any.   
