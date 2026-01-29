//! `LxWEngd` entry
//!
//! The daemon that operates `linux-wallpaperengine`.
//! Unless `--standby` is passed in the arguments, the programs attempts to find the default
//! playlist and runs it on all possible monitors.

use smol::lock::Mutex;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use thiserror::Error;

use crate::cli::{Config, configure};
use crate::runner::NOMONITOR_INDICATOR;
use crate::runner::{Action, Runner, RunnerHandle};
use crate::utils::ipc::IPCCmd;

pub static CFG: LazyLock<Config> = LazyLock::new(configure);
pub static SEARCH_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    find_search_path().unwrap_or_else(|| {
        log::warn!("Cannot find search path");
        // If fully qualified name is passed, this value does not matter
        PathBuf::from("")
    })
});
pub static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(find_cache_path);

pub struct LxWEngd {
    runners: HashMap<String, Arc<Mutex<RunnerHandle>>>,
    socket: UnixListener,
}

impl Drop for LxWEngd {
    fn drop(&mut self) {
        let addr = self.socket.local_addr().unwrap();
        let path = addr.as_pathname().unwrap();
        std::fs::remove_file(path).expect("Failed to unbind socket")
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum DaemonError {
    #[error("Failed to initialise socket")]
    InitSocket,
    #[error("Failed to initialise logger")]
    InitLogger,
    #[error("Failed to initialise cache")]
    InitCache,
    #[error("No such runner")]
    NoSuchRunner,
}

fn find_cache_path() -> PathBuf {
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

fn find_search_path() -> Option<PathBuf> {
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

impl LxWEngd {
    pub fn init() -> Result<Self, DaemonError> {
        let mut socket_path = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string());
        socket_path.push_str("/lxwengd.sock");
        let _ = std::fs::remove_file(&socket_path);
        let socket = UnixListener::bind(socket_path).map_err(|_| DaemonError::InitSocket)?;

        setup_logger().map_err(|_| DaemonError::InitLogger)?;

        // If cache directory does not exist, create it
        if !CACHE_PATH.is_dir() {
            std::fs::create_dir(CACHE_PATH.as_path()).map_err(|err| {
                log::error!("Failed to create cache directory: {err}");
                DaemonError::InitCache
            })?;
        }

        Ok(Self {
            runners: HashMap::new(),
            socket,
        })
    }

    /// The real start.
    ///
    /// # Errors
    /// Fatal errors that will cause the program to exit will be returned here.
    #[allow(clippy::map_entry)]
    pub fn start(&mut self) {
        if !CFG.standby {
            let monitor = CFG
                .default_monitor
                .clone()
                .unwrap_or(NOMONITOR_INDICATOR.to_string());
            match Runner::new(monitor.clone(), CFG.default_playlist.clone()) {
                Ok((mut runner, handle)) => {
                    // One runner runs on one monitor
                    self.runners.insert(monitor, handle);
                    smol::spawn(async move {
                        runner.run().await;
                    })
                    .detach();
                }
                Err(err) => {
                    log::error!("{err}");
                }
            }
        }
        loop {
            if let Ok((mut conn, _)) = self.socket.accept() {
                let mut content = String::new();
                let _ = BufReader::new(&conn).read_line(&mut content);
                let cmd = IPCCmd::from_str(&content);
                match cmd {
                    Ok(IPCCmd::Load {
                        path,
                        monitor,
                        paused: _,
                        resume_mode: _,
                    }) => {
                        Self::try_cleanup(&mut self.runners);
                        if self.runners.contains_key(&monitor) {
                            let err = format!("Already have a runner on {monitor}");
                            log::error!("{err}");
                            let _ = conn.write_all(&err.into_bytes());
                        } else {
                            match Runner::new(monitor.clone(), path) {
                                Ok((mut runner, handle)) => {
                                    // One runner runs on one monitor
                                    self.runners.insert(monitor, handle);
                                    smol::spawn(async move {
                                        runner.run().await;
                                    })
                                    .detach();
                                    let _ = conn.write_all(b"OK");
                                }
                                Err(err) => {
                                    log::error!("{err}");
                                    let _ = conn.write_all(&err.to_string().into_bytes());
                                }
                            }
                        }
                    }

                    // TODO: Add resume support
                    Ok(IPCCmd::Unload {
                        no_save: _,
                        monitor,
                    }) => {
                        Self::try_cleanup(&mut self.runners);
                        if self.forward_action(&monitor, Action::Exit).is_ok() {
                            let _ = conn.write_all(b"OK");
                        } else {
                            let _ = conn.write_all(b"No such runner");
                        }
                    }

                    Ok(IPCCmd::Pause { clear, monitor }) => {
                        Self::try_cleanup(&mut self.runners);
                        if self.forward_action(&monitor, Action::Pause(clear)).is_ok() {
                            let _ = conn.write_all(b"OK");
                        } else {
                            let _ = conn.write_all(b"No such runner");
                        }
                    }

                    Ok(IPCCmd::Play { monitor }) => {
                        Self::try_cleanup(&mut self.runners);
                        if self.forward_action(&monitor, Action::Next).is_ok() {
                            let _ = conn.write_all(b"OK");
                        } else {
                            let _ = conn.write_all(b"No such runner");
                        }
                    }

                    Ok(IPCCmd::Status) => {
                        Self::try_cleanup(&mut self.runners);
                        let status = self.status_string();
                        let _ = conn.write_all(&status.into_bytes());
                    }

                    Ok(IPCCmd::Quit) => {
                        // Exit all runners to prevent orphan subprocesses.
                        self.runners.keys().for_each(|k| {
                            let _ = self.forward_action(k, Action::Exit);
                        });
                        let _ = conn.write_all(b"OK");
                        break;
                    }

                    Ok(IPCCmd::Toggle { monitor: _ }) => {
                        todo!()
                    }

                    Err(err) => {
                        log::error!("{err}");
                        let _ = conn.write_all(&err.to_string().into_bytes());
                    }
                }
            }
        }
    }

    /// When [`Runner`]s exit, they set their state to [`State::Exited`].
    /// However, they are not automatically deregistered.
    ///
    /// This method will remove [`Runner`]s with [`State::Exited`].
    /// This is intented to be invoked before accessing the runners map.
    fn try_cleanup(runners: &mut HashMap<String, Arc<Mutex<RunnerHandle>>>) {
        let mut result = HashMap::new();
        for (id, runner) in runners.drain() {
            if !runner.lock_blocking().exited() {
                result.insert(id, runner);
            }
        }
        *runners = result;
    }

    /// Get all [`Runner`]s' status as a single [`String`].
    fn status_string(&self) -> String {
        let mut result = vec![];
        for (monitor, runner) in &self.runners {
            let str = runner.lock_blocking().to_string();
            result.push(format!("Runner {}\n{str}\n", monitor));
        }
        result.concat()
    }

    /// Forward an [`IPCCmd`] to the given [`Runner`] as an [`Action`].
    fn forward_action(&self, monitor: &str, action: Action) -> Result<(), DaemonError> {
        let lock = self.runners.get(monitor).ok_or(DaemonError::NoSuchRunner)?;
        lock.lock_blocking()
            .interrupt(action)
            // `.interrupt()` returns an error when the channel is closed
            // This means the runner no longer exists.
            .map_err(|_| DaemonError::NoSuchRunner)?;
        Ok(())
    }
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
            assert_eq!(find_search_path().unwrap(), PathBuf::from("./lxwengd"));
            env::remove_var("XDG_CONFIG_HOME");
            env::set_var("HOME", ".");
            assert_eq!(
                find_search_path().unwrap(),
                PathBuf::from("./.config/lxwengd")
            );
            env::remove_var("HOME");
            assert!(find_search_path().is_none());

            env::set_var("XDG_CACHE_HOME", "/some_cachey_place");
            assert_eq!(
                find_cache_path(),
                PathBuf::from("/some_cachey_place/lxwengd")
            );
            env::remove_var("XDG_CACHE_HOME");
            env::set_var("HOME", "/somewhere");
            assert_eq!(
                find_cache_path(),
                PathBuf::from("/somewhere/.cache/lxwengd")
            );
            env::remove_var("HOME");
            assert_eq!(find_cache_path(), PathBuf::from("/tmp/lxwengd"));
        }
    }
}
