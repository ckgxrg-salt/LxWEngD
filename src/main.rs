mod playlist;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version = "0.1", about, long_about = None)]
struct Cli {
    #[arg(short = 'p', long = "playlist", value_name = "FILE")]
    playlist: Option<PathBuf>,
}

fn main() {
    let parsed = Cli::parse();
    let playlist = if let Some(value) = parsed.playlist.as_deref() {
        value
    } else {
        &PathBuf::from("default.playlist")
    };
    println!("{:?}", playlist);
}
