//! # Entry of `LxWEngD`
//!
//! Upon starting, the main thread starts the first runner thread and add it to known runners.
//!
//! The main thread then listens for requests from runner threads.
//! `DaemonRequest::NewRunner` can ask the main thread to summon a new runner thread and add it to
//! known runners.
//!
//! When a runner finishes its job, either gracefully or unexpectedly, they report to the main
//! thread using `DaemonRequest::Exit` or `DaemonRequest::Abort`.
//! The main thread will remove the runner from known runners, then.
//!
//! When all known runners exited, the main thread also quits.
#![warn(clippy::pedantic)]

use clap::Parser;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use lxwengd::{DaemonRequest, Runner, RuntimeError};

#[derive(Parser)]
#[command(
    version = "0.1.1",
    about="A daemon that adds playlists to linux-wallpaperengine",
    long_about = None
)]
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
fn sys_cache_dir() -> PathBuf {
    // linux-wallpaperengine generates some cache
    if let Ok(mut value) = env::var("XDG_CACHE_HOME") {
        value.push_str("/lxwengd");
        return PathBuf::from(value);
    };
    if let Ok(mut value) = env::var("HOME") {
        value.push_str("/.cache/lxwengd");
        return PathBuf::from(value);
    };
    // This is not persistent anyhow
    PathBuf::from("/tmp/lxwengd")
}
fn sys_config_dir() -> Result<PathBuf, RuntimeError> {
    let default;
    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        default = PathBuf::from(value + "/lxwengd");
    } else if let Ok(value) = env::var("HOME") {
        default = PathBuf::from(value + "/.config/lxwengd");
    } else {
        return Err(RuntimeError::InitFailed);
    }
    Ok(default)
}
lazy_static! {
    static ref Cfg: Config = parse();
    static ref SearchPath: PathBuf = sys_config_dir().unwrap_or_else(|err| {
        eprintln!("{err}");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    });
    static ref CachePath: PathBuf = sys_cache_dir();
}

fn main() -> Result<(), RuntimeError> {
    // If cache directory does not exist, create it
    if !CachePath.is_dir() {
        if let Err(err) = std::fs::create_dir(CachePath.as_path()) {
            eprintln!("Failed to create the cache directory: {err}");
            return Err(RuntimeError::InitFailed);
        };
    }

    // Begin creating first runner
    let (tx, rx) = mpsc::channel::<DaemonRequest>();
    let mut first_runner = Runner::new(
        0,
        Cfg.playlist.clone(),
        &SearchPath,
        &CachePath,
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
                eprintln!("Runner {id} aborted with error: {error}");
                runners.remove(&id);
            }
            DaemonRequest::NewRunner(playlist) => {
                if let Ok(id) = available_id(&runners) {
                    let mut runner = Runner::new(
                        id,
                        playlist,
                        &SearchPath,
                        &CachePath,
                        tx.clone(),
                        Cfg.dry_run,
                    );
                    runner.assets_path(Cfg.assets_path.as_deref());
                    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();
                    let Ok(thread) = thread::Builder::new()
                        .name(format!("Runner {id}"))
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
