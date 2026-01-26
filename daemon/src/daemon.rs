//! `LxWEngd` entry
//!
//! The daemon that operates `linux-wallpaperengine`.
//! Unless `--standby` is passed in the arguments, the programs attempts to find the default
//! playlist and runs it on all possible monitors.

use clap::Parser;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::backends::Backend;
use crate::runner::Runner;
use crate::utils::socket::Socket;

pub static CFG: LazyLock<Config> = LazyLock::new(parse);
pub static SEARCH_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    sys_config_dir().unwrap_or_else(|| {
        log::warn!("Cannot find search path as $XDG_CONFIG_HOME and $HOME are not valid");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    })
});
pub static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(sys_cache_dir);

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

pub struct Config {
    pub default_playlist: PathBuf,
    pub assets_path: Option<PathBuf>,
    pub binary: Option<String>,
    pub standby: bool,
}
fn parse() -> Config {
    let parsed = Cli::parse();
    let default_playlist = if let Some(value) = parsed.playlist {
        value
    } else {
        PathBuf::from("default.playlist")
    };
    Config {
        default_playlist,
        assets_path: parsed.assets_path,
        binary: parsed.binary,
        standby: parsed.standby,
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

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                message
            ));
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

/// The real start.
///
/// # Errors
/// Fatal errors that will cause the program to exit will be returned here.
pub async fn start<T: Backend>() -> Result<(), Box<dyn Error>> {
    // If cache directory does not exist, create it
    if !CACHE_PATH.is_dir() {
        std::fs::create_dir(CACHE_PATH.as_path()).inspect_err(|err| {
            eprintln!("failed to create cache directory: {err}");
        })?;
    }
    setup_logger()?;

    let socket =
        Socket::new().inspect_err(|err| eprintln!("failed to create unix socket: {err}"))?;
    let runners: HashMap<String, Runner<T>> = HashMap::new();

    socket.listen().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Due to [`env::set_var()`] not being thread-safe, just chain them so the variables are not
    // messed around.
    #[test]
    fn getting_locations() {
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
            assert!(sys_config_dir().is_none());

            env::set_var("XDG_CACHE_HOME", "/some_cachey_place");
            assert_eq!(sys_cache_dir(), PathBuf::from("/some_cachey_place/lxwengd"));
            env::remove_var("XDG_CACHE_HOME");
            env::set_var("HOME", "/somewhere");
            assert_eq!(sys_cache_dir(), PathBuf::from("/somewhere/.cache/lxwengd"));
            env::remove_var("HOME");
            assert_eq!(sys_cache_dir(), PathBuf::from("/tmp/lxwengd"));
        }
    }
}
