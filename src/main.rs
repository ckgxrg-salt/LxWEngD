mod commands;
mod playlist;
mod runner;
mod wallpaper;

use clap::Parser;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::mpsc;
use std::{path::PathBuf, thread};

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

struct Config {
    playlist: PathBuf,
    assets_path: Option<PathBuf>,
    binary: Option<PathBuf>,
    dry_run: bool,
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
        dry_run: parsed.dry_run,
    }
}
lazy_static! {
    static ref Cfg: Config = parse();
}

/// Accepted requests to the main thread
/// - NewRunner(PathBuf): Summons a new runner thread operating the given playlist.
enum DaemonRequest {
    NewRunner(PathBuf),
    Exit(u8),
}
fn invoke() {
    let (tx, rx) = mpsc::channel::<DaemonRequest>();
    let mut first_runner = runner::Runner::new(
        0,
        Cfg.playlist.clone(),
        playlist::config_dir().unwrap(),
        tx.clone(),
    );
    first_runner.assets_path(Cfg.assets_path.as_deref());
    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();
    let first = thread::spawn(move || first_runner.run().unwrap());
    runners.insert(0, first);

    while !runners.is_empty() {
        let message = rx.recv().unwrap();
        match message {
            DaemonRequest::Exit(id) => {
                runners.remove(&id);
            }
            DaemonRequest::NewRunner(playlist) => {
                if let Ok(id) = available_id(&runners) {
                    let mut runner = runner::Runner::new(
                        id,
                        playlist,
                        playlist::config_dir().unwrap(),
                        tx.clone(),
                    );
                    runner.assets_path(Cfg.assets_path.as_deref());
                    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();
                    let thread = thread::spawn(move || runner.run().unwrap());
                    runners.insert(id, thread);
                } else {
                    eprintln!("Cannot allocate an id for new runner, perhaps upper limit has been reached?");
                }
            }
        };
    }
}

fn available_id(map: &HashMap<u8, thread::JoinHandle<()>>) -> Result<u8, ()> {
    for id in 0..u8::MAX {
        if map.get(&id).is_none() {
            return Ok(id);
        }
    }
    Err(())
}

fn main() {}
