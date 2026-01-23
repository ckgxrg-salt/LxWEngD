lxwengctl playlist ... # Loads a playlist
lxwengctl playlist --paused
lxwengctl playlist --resume=[ignore(false)|delete|true]
lxwengctl playlist --monitor ...

lxwengctl pause
lxwengctl pause --clear # Terminate linux-wallpaperengine

lxwengctl play
lxwengctl toggle

lxwengctl stop # Unloads current playlist
lxwengctl stop --no-resume
lxwengctl status

lxwengctl exec "<some command>"

lxwengd # Looks for default
lxwengd --standby # Do nothing until `lxwengctl playlist`

Daemon tasks:
1. Unless `--standby`, check for default playlist and load.
2. Listen on socket for new commands(DaemonRequest enum)
3. When creating runner, perform the parsing, instantiate one and put in a BTreeMap then .await on it.
