//! Program main loop

use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

static CFG: LazyLock<Config> = LazyLock::new(parse);
pub static SEARCH_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    sys_config_dir().unwrap_or_else(|| {
        log::warn!("cannot find search path as $XDG_CONFIG_HOME and $HOME are not valid");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    })
});
static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(sys_cache_dir);

#[derive(Parser)]
#[command(
    version = "1.1.0",
    about = "A daemon that adds playlists to linux-wallpaperengine"
)]
struct Cli {
    #[arg(
        short = 'p',
        long = "playlist",
        value_name = "FILE",
        help = "Path to the default playlist."
    )]
    playlist: Option<PathBuf>,

    #[arg(
        short = 'b',
        long = "binary",
        value_name = "PATH",
        help = "Path to the linux-wallpaperengine binary."
    )]
    binary: Option<String>,

    #[arg(
        short = 'a',
        long = "assets-path",
        value_name = "PATH",
        help = "Path to Wallpaper Engine assets."
    )]
    assets_path: Option<PathBuf>,

    #[arg(
        long = "standby",
        help = "Do not load the default playlist on startup."
    )]
    standby: bool,
}

struct Config {
    playlist: PathBuf,
    assets_path: Option<PathBuf>,
    binary: Option<String>,
}
fn parse() -> Config {
    let parsed = Cli::parse();
    let playlist = if let Some(value) = parsed.playlist {
        value
    } else {
        PathBuf::from("default.playlist")
    };
    Config {
        playlist,
        assets_path: parsed.assets_path,
        binary: parsed.binary,
    }
}
fn sys_cache_dir() -> PathBuf {
    // linux-wallpaperengine generates some cache
    if let Ok(mut value) = env::var("XDG_CACHE_HOME") {
        value.push_str("/lxwengd");
        return PathBuf::from(value);
    }
    if let Ok(mut value) = env::var("HOME") {
        value.push_str("/.cache/lxwengd");
        return PathBuf::from(value);
    }
    // This is not persistent anyhow
    PathBuf::from("/tmp/lxwengd")
}
fn sys_config_dir() -> Option<PathBuf> {
    let default;
    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        default = PathBuf::from(value + "/lxwengd");
    } else if let Ok(value) = env::var("HOME") {
        default = PathBuf::from(value + "/.config/lxwengd");
    } else {
        return None;
    }
    Some(default)
}
