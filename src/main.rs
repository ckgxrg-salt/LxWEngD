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
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{collections::HashMap, sync::mpsc::Sender};

use lxwengd::{DaemonRequest, Runner, RuntimeError};

#[derive(Parser)]
#[command(
    version = "0.1.3",
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
    binary: Option<String>,

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
    binary: Option<String>,
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
        log::warn!("{err}");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    });
    static ref CachePath: PathBuf = sys_cache_dir();
}

// logging
fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                thread::current().name().unwrap(),
                record.level(),
                message
            ));
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

/// Creates a new runner operating the given playlist
fn summon_runner(
    id: u8,
    playlist: PathBuf,
    channel: Sender<DaemonRequest>,
) -> Result<thread::JoinHandle<()>, RuntimeError> {
    let mut runner = Runner::new(0, &SearchPath, &CachePath, channel);
    runner.assets_path(Cfg.assets_path.as_deref());
    runner.binary(Cfg.binary.as_deref());
    let Ok(thread) = thread::Builder::new()
        .name(format!("Runner {id}"))
        .spawn(move || {
            runner.init(playlist);
            runner.dispatch(Cfg.dry_run);
        })
    else {
        log::warn!("Failed to summon new runner due to OS error");
        return Err(RuntimeError::InitFailed);
    };
    Ok(thread)
}

// Cli main entry
fn main() -> Result<(), RuntimeError> {
    // If cache directory does not exist, create it
    if !CachePath.is_dir() {
        if let Err(err) = std::fs::create_dir(CachePath.as_path()) {
            eprintln!("Failed to create the cache directory: {err}");
            return Err(RuntimeError::InitFailed);
        };
    }

    setup_logger().map_err(|_| RuntimeError::InitFailed)?;

    // Begin creating first runner
    let (tx, rx) = mpsc::channel::<DaemonRequest>();
    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();

    runners.insert(0, summon_runner(0, Cfg.playlist.clone(), tx.clone())?);

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
                    let Ok(thread) = summon_runner(id, playlist, tx.clone()) else {
                        log::warn!("Failed to summon new runner due to OS error");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_location() {
        env::set_var("XDG_CONFIG_HOME", ".");
        assert_eq!(sys_config_dir().unwrap(), PathBuf::from("./lxwengd"));
        env::remove_var("XDG_CONFIG_HOME");
        env::set_var("HOME", ".");
        assert_eq!(
            sys_config_dir().unwrap(),
            PathBuf::from("./.config/lxwengd")
        );
        env::remove_var("HOME");
        assert!(sys_config_dir().is_err());
    }

    #[test]
    fn cache_location() {
        env::set_var("XDG_CACHE_HOME", ".");
        assert_eq!(sys_cache_dir(), PathBuf::from("./lxwengd"));
        env::remove_var("XDG_CACHE_HOME");
        env::set_var("HOME", ".");
        assert_eq!(sys_cache_dir(), PathBuf::from("./.cache/lxwengd"));
        env::remove_var("HOME");
        assert_eq!(sys_cache_dir(), PathBuf::from("/tmp/lxwengd"));
    }
}
