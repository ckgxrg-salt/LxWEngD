//! A runner is created with a list of [`Command`]s that it will execute.
//!
//! After init, it awaits on 3 tasks:
//! - Timer: which wakes it when the designated [`WallpaperDuration`] has elapsed.
//! - Command: which wakes it when the subprocess exits.
//! - Message: which wakes it when it's interrupted externally.
//!
//! A runner registers it with the daemon.
//! When it exits, it clears its own entry in the registered runners.

use smol::channel::{Receiver, Sender, TrySendError};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

use crate::backends::{Backend, LxWEng};
use crate::runner::Action;
use crate::utils::command::{CmdDuration, Command};
use crate::utils::playlist;

pub struct Runner<T: Backend> {
    index: usize,
    commands: Vec<Command>,

    path: PathBuf,

    backend: T,

    tx: Sender<Action>,
    rx: Receiver<Action>,
}

#[derive(Debug, PartialEq, Error)]
pub enum RunnerError {
    #[error("Runner init failed")]
    InitFailed,
    #[error("Cannot spawn `linux-wallpaperengine`")]
    CannotSpawn,
    #[error("`linux-wallpaperengine` unexpectedly exited")]
    EngineDied,
}

/// How should a runner treat state files on startup.
#[derive(Debug, PartialEq)]
pub enum ResumeMode {
    /// Ignore the state file for the playlist.
    Ignore,
    /// Ignore and delete the state file for the playlist.
    IgnoreDel,
    /// Apply but delete the state file for the playlist.
    ApplyDel,
    /// Apply the state file for the playlist. This is the default behaviour.
    Apply,
}

impl FromStr for ResumeMode {
    type Err = RunnerError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ignore" => Ok(Self::Ignore),
            "ignoredel" => Ok(Self::IgnoreDel),
            "applydel" => Ok(Self::ApplyDel),
            "apply" => Ok(Self::Apply),
            _ => Err(Self::Err::InitFailed),
        }
    }
}

/// Runner's state
enum State {
    Running,
    /// Stores [`Duration`] that has elapsed since last command.
    Paused(Duration),
    /// Received a [`RunnerAction`] and is executing that command.
    Execute(Command),
}

// Currently only `linux-wallpaperengine` is supported.
impl Runner<LxWEng> {
    /// Creates a new Runner that operates the given playlist.
    ///
    /// # Errors
    /// If the given playlist cannot be parsed, or is empty, this will return [`RunnerError::InitFailed`].
    pub fn new(monitor: Option<String>, path: PathBuf) -> Result<Self, RunnerError> {
        match playlist::open(&path) {
            Ok(file) => {
                let (tx, rx) = smol::channel::unbounded();
                let commands = playlist::parse(&path, &file).ok_or(RunnerError::InitFailed)?;
                Ok(Self {
                    commands,
                    index: 0,
                    path,
                    backend: LxWEng::new(monitor),
                    tx,
                    rx,
                })
            }
            Err(err) => {
                log::error!("{err}");
                Err(RunnerError::InitFailed)
            }
        }
    }

    /// The main runner task.
    pub async fn run(&mut self) {
        loop {
            // By default go back to the beginning when reached the end
            if self.index >= self.commands.len() {
                self.index = 0;
            }
            let Some(current_cmd) = self.commands.get(self.index) else {
                log::error!("Got invalid command");
                self.index += 1;
                continue;
            };
            self.index += 1;
            match current_cmd {
                Command::Wallpaper(id, duration, properties) => {}
                Command::Sleep(duration) => {
                    // smol::Timer::after(*duration).await;
                }
                Command::Default(properties) => {
                    self.backend.update_default_props(properties);
                }
                Command::End => {
                    break;
                }
            }
        }
    }

    /// Interrupts the runner to perform an [`Action`].
    ///
    /// # Errors
    /// See [`smol::channel::Sender::try_send`].
    pub fn interrupt(&mut self, action: Action) -> Result<(), TrySendError<Action>> {
        self.tx.try_send(action)
    }
}
