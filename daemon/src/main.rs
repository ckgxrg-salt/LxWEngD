//! # `LxWEngd` entry
//!
//! The daemon that operates `linux-wallpaperengine`.
//! Unless `--standby` is passed in the arguments, the programs attempts to find the default
//! playlist and runs it on all possible monitors.
//!
//! TODO:The daemon listens commands from a socket and executes them until it's stopped.

use clap::Parser;
use std::env;
use std::path::PathBuf;
use std::sync::{LazyLock, mpsc};
use std::thread;
use std::{collections::HashMap, sync::mpsc::Sender};

use lxwengd::{Runner, RuntimeError};

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

static CFG: LazyLock<Config> = LazyLock::new(parse);
static SEARCH_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    sys_config_dir().unwrap_or_else(|err| {
        log::warn!("{err}");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    })
});
static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(sys_cache_dir);

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
fn summon_runner(id: u8, playlist: PathBuf) -> Result<thread::JoinHandle<()>, RuntimeError> {
    let mut runner = Runner::new(0, &SEARCH_PATH, &CACHE_PATH);
    runner.assets_path(CFG.assets_path.as_deref());
    runner.binary(CFG.binary.as_deref());
    let Ok(thread) = thread::Builder::new()
        .name(format!("Runner {id}"))
        .spawn(move || {
            runner.init(playlist);
            runner.dispatch();
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
    if !CACHE_PATH.is_dir() {
        if let Err(err) = std::fs::create_dir(CACHE_PATH.as_path()) {
            eprintln!("Failed to create the cache directory: {err}");
            return Err(RuntimeError::InitFailed);
        }
    }

    setup_logger().map_err(|_| RuntimeError::InitFailed)?;

    // Begin creating first runner
    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();

    runners.insert(0, summon_runner(0, CFG.playlist.clone())?);

    // Listen to commands
    while !runners.is_empty() {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_location() {
        unsafe {
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
    }

    #[test]
    fn cache_location() {
        unsafe {
            env::set_var("XDG_CACHE_HOME", ".");
            assert_eq!(sys_cache_dir(), PathBuf::from("./lxwengd"));
            env::remove_var("XDG_CACHE_HOME");
            env::set_var("HOME", ".");
            assert_eq!(sys_cache_dir(), PathBuf::from("./.cache/lxwengd"));
            env::remove_var("HOME");
            assert_eq!(sys_cache_dir(), PathBuf::from("/tmp/lxwengd"));
        }
    }
}
