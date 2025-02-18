mod commands;
mod playlist;
mod runner;
mod wallpaper;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version = "0.1", about, long_about = None)]
struct Cli {
    #[arg(
        short = 'p',
        long = "playlist",
        value_name = "FILE",
        help = "Indicate a playlist file to use."
    )]
    playlist: Option<PathBuf>,

    #[arg{
        short = 'b',
        long = "binary",
        value_name = "PATH",
        help = "Path to the linux-wallpaperengine binary, will search in $PATH if not given."
    }]
    binary: Option<PathBuf>,

    #[arg{
        short = 'a',
        long = "assets-path",
        value_name = "PATH",
        help = "Path to Wallpaper Engine assets."
    }]
    assets_path: Option<PathBuf>,

    #[arg(
        long = "dry-run",
        help = "Prints what would be done, but not really doing so."
    )]
    dry_run: bool,
}

fn main() {
    let parsed = Cli::parse();
    let playlist = if let Some(value) = parsed.playlist.as_deref() {
        value
    } else {
        &PathBuf::from("default.playlist")
    };
    let assets_path = parsed.assets_path.as_deref();

    let mut runner = runner::Runner::new(playlist.to_owned(), playlist::config_dir().unwrap());
    if let Some(value) = assets_path {
        runner.assets_path(value);
    }
    runner.run().unwrap();
}
