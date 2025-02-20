//! # Entry of LxWEngD
//!
//! Upon starting, the main thread starts the first runner thread and add it to known runners.   
//!
//! The main thread then listens for requests from runner threads.   
//! DaemonRequest::NewRunner can ask the main thread to summon a new runner thread and add it to
//! known runners.   
//!
//! When a runner finishes its job, either gracefully or unexpectedly, they report to the main
//! thread using DaemonRequest::Exit or DaemonRequest::Abort.   
//! The main thread will remove the runner from known runners, then.   
//!
//! When all known runners exited, the main thread also quits.   

mod commands;
mod playlist;
mod runner;
mod wallpaper;

use clap::Parser;
use lazy_static::lazy_static;
use runner::RuntimeError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

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
    static ref SearchPath: PathBuf = playlist::config_dir().unwrap_or_else(|err| {
        eprintln!("{}", err);
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    });
    static ref CachePath: PathBuf = wallpaper::cache_dir();
}

/// Accepted requests to the main thread   
/// - NewRunner(playlist file): Summons a new runner thread operating the given playlist.   
/// - Exit(id): Reports that runner is gracefully exiting.   
/// - Abort(id, error): Reports that runner encountered an error, and is halting.   
enum DaemonRequest {
    NewRunner(PathBuf),
    Exit(u8),
    Abort(u8, runner::RuntimeError),
}

fn main() -> Result<(), runner::RuntimeError> {
    // If cache directory does not exist, create it
    if !CachePath.is_dir() {
        if let Err(err) = std::fs::create_dir(CachePath.as_path()) {
            eprintln!("Failed to create the cache directory: {}", err);
            return Err(RuntimeError::InitFailed);
        };
    }

    // Begin creating first runner
    let (tx, rx) = mpsc::channel::<DaemonRequest>();
    let mut first_runner = runner::Runner::new(
        0,
        Cfg.playlist.clone(),
        &SearchPath,
        tx.clone(),
        Cfg.dry_run,
    );
    first_runner.assets_path(Cfg.assets_path.as_deref());
    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();
    let first = thread::spawn(move || first_runner.run());
    runners.insert(0, first);

    // Listen to commands
    while !runners.is_empty() {
        let Ok(message) = rx.recv() else {
            eprintln!("Channel to runners has closed, aborting");
            break;
        };
        match message {
            DaemonRequest::Exit(id) => {
                runners.remove(&id);
            }
            DaemonRequest::Abort(id, error) => {
                eprintln!("Runner {0} aborted with error: {1}", id, error);
                runners.remove(&id);
            }
            DaemonRequest::NewRunner(playlist) => {
                if let Ok(id) = available_id(&runners) {
                    let mut runner =
                        runner::Runner::new(id, playlist, &SearchPath, tx.clone(), Cfg.dry_run);
                    runner.assets_path(Cfg.assets_path.as_deref());
                    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();
                    let Ok(thread) = thread::Builder::new()
                        .name(format!("Runner {}", id))
                        .spawn(move || runner.run())
                    else {
                        eprintln!("Failed to summon new runner due to OS error");
                        continue;
                    };
                    runners.insert(id, thread);
                } else {
                    eprintln!("Cannot allocate an id for new runner, perhaps upper limit has been reached?");
                }
            }
        };
    }
    Ok(())
}

fn available_id(map: &HashMap<u8, thread::JoinHandle<()>>) -> Result<u8, ()> {
    for id in 0..u8::MAX {
        if map.get(&id).is_none() {
            return Ok(id);
        }
    }
    Err(())
}
