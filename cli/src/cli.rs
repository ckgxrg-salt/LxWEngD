//! cli parameters

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version = "1.1.0",
    about="CLI tool for manipulating linux-wallpaperengine.",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Load playlists")]
    Playlist,
    #[command(about = "Pause a playlist")]
    Pause,
    #[command(about = "Resume/Play a playlist")]
    Play,
    #[command(about = "Toggle play/pause for a playlist")]
    Toggle,
    #[command(about = "Stop and unload a playlist")]
    Stop,
    #[command(about = "Show LxWEngd status")]
    Status,
}
