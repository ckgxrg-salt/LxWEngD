//! The `lxwengd` CLI

use clap::Parser;
use std::path::PathBuf;

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
        short = 'm',
        long = "monitor",
        value_name = "NAME",
        help = "Monitor to be used for the default playlist."
    )]
    monitor: Option<String>,

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

pub struct Config {
    pub default_playlist: PathBuf,
    pub default_monitor: Option<String>,
    pub assets_path: Option<PathBuf>,
    pub binary: Option<String>,
    pub standby: bool,
}

/// Reads arguments from command line and generates [`Config`].
pub fn configure() -> Config {
    let parsed = Cli::parse();
    let default_playlist = if let Some(value) = parsed.playlist {
        value
    } else {
        PathBuf::from("default.playlist")
    };
    Config {
        default_playlist,
        default_monitor: parsed.monitor,
        assets_path: parsed.assets_path,
        binary: parsed.binary,
        standby: parsed.standby,
    }
}
