//! `LxWEngd` entry
//!
//! The daemon that operates `linux-wallpaperengine`.
//! Unless `--standby` is passed in the arguments, the programs attempts to find the default
//! playlist and runs it on all possible monitors.

use smol::io::AsyncReadExt;
use smol::io::AsyncWriteExt;
use smol::lock::Mutex;
use smol::net::unix::UnixListener;
use smol::stream::StreamExt;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;
use thiserror::Error;

use crate::backends::Backend;
use crate::cli::{Config, configure};
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

#[derive(Debug, PartialEq, Error)]
pub enum DaemonError {
    #[error("Failed to initialise socket")]
    InitFailed,
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
        let mut path = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string());
        path.push_str("lxwengd.sock");
        let socket = UnixListener::bind(path).map_err(|_| DaemonError::InitFailed)?;
        Ok(Self {
            runners: HashMap::new(),
            socket,
        })
    }

    /// The real start.
    ///
    /// # Errors
    /// Fatal errors that will cause the program to exit will be returned here.
    pub async fn start<T: Backend>(&mut self) -> Result<(), DaemonError> {
        setup_logger().map_err(|_| DaemonError::InitFailed)?;

        // If cache directory does not exist, create it
        if !CACHE_PATH.is_dir() {
            std::fs::create_dir(CACHE_PATH.as_path()).map_err(|err| {
                log::error!("Failed to create cache directory: {err}");
                DaemonError::InitFailed
            })?;
        }

        loop {
            let mut incoming = self.socket.incoming();
            if let Some(Ok(mut conn)) = incoming.next().await {
                let mut content = String::new();
                let _ = conn.read_to_string(&mut content).await;
                let cmd = IPCCmd::from_str(&content);
                match cmd {
                    Ok(IPCCmd::Load {
                        path,
                        monitor,
                        paused,
                        resume_mode,
                    }) => {
                        Self::try_cleanup(&mut self.runners).await;
                        if self.runners.contains_key(&monitor) {
                            let err = format!("Already have a runner on {monitor}");
                            log::error!("{err}");
                            let _ = conn.write_all(&err.into_bytes()).await;
                        } else {
                            match Runner::new(monitor.clone(), path) {
                                Ok((mut runner, handle)) => {
                                    // One runner runs on one monitor
                                    self.runners.insert(monitor, handle);
                                    smol::spawn(async move {
                                        runner.run();
                                    });
                                    conn.write_all(b"OK");
                                }
                                Err(err) => {
                                    log::error!("{err}");
                                    conn.write_all(&err.to_string().into_bytes());
                                }
                            }
                        }
                    }

                    // TODO: Add resume support
                    Ok(IPCCmd::Unload { no_save, monitor }) => {
                        if let Some(handle) = self.runners.remove(&monitor) {
                            handle.lock().await.interrupt(Action::Exit);
                        } else {
                            log::error!("No such runner")
                        }
                    }

                    Ok(IPCCmd::Pause { clear, monitor }) => {
                        Self::try_cleanup(&mut self.runners);
                        self.forward_action(&monitor, Action::Pause(clear));
                        conn.write_all(b"OK");
                    }

                    Ok(IPCCmd::Play { monitor }) => {
                        Self::try_cleanup(&mut self.runners);
                        self.forward_action(&monitor, Action::Next);
                        conn.write_all(b"OK");
                    }

                    Ok(IPCCmd::Status) => {
                        Self::try_cleanup(&mut self.runners);
                        let status = self.status_string().await;
                        conn.write_all(&status.into_bytes());
                    }

                    Ok(IPCCmd::Quit) => {
                        conn.write_all(b"OK");
                        break;
                    }

                    Ok(IPCCmd::Toggle { monitor }) => {
                        todo!()
                    }

                    Err(err) => {
                        log::error!("{err}");
                        conn.write_all(&err.to_string().into_bytes());
                    }
                }
            }
        }

        Ok(())
    }

    /// When [`Runner`]s exit, they set their state to [`State::Exited`].
    /// However, they are not automatically deregistered.
    ///
    /// This method will remove [`Runner`]s with [`State::Exited`].
    /// This is intented to be invoked before accessing the runners map.
    async fn try_cleanup(runners: &mut HashMap<String, Arc<Mutex<RunnerHandle>>>) {
        let mut result = HashMap::new();
        for (id, runner) in runners.drain() {
            if !runner.lock().await.exited() {
                result.insert(id, runner);
            }
        }
        *runners = result;
    }

    /// Get all [`Runner`]s' status as a single [`String`].
    async fn status_string(&self) -> String {
        let mut result = vec![];
        for (monitor, runner) in &self.runners {
            let str = runner.lock().await.to_string();
            result.push(format!("Runner {}\n{str}\n", monitor.to_string()));
        }
        result.concat()
    }

    /// Forward an [`IPCCmd`] to the given [`Runner`] as an [`Action`].
    async fn forward_action(&self, monitor: &str, action: Action) -> Result<(), DaemonError> {
        let lock = self.runners.get(monitor).ok_or(DaemonError::NoSuchRunner)?;
        lock.lock()
            .await
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
