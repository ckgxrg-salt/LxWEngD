mod action;
mod exec;
mod runner;

pub use action::Action;

use smol::channel::{Receiver, Sender, TrySendError};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

use crate::backends::Backend;
use crate::utils::command::Command;

/// Data structure of a runner.
pub struct Runner<T: Backend> {
    index: usize,
    commands: Vec<Command>,
    state: State,

    path: PathBuf,

    backend: T,

    tx: Sender<Action>,
    rx: Receiver<Action>,
}

impl<T: Backend> Runner<T> {
    /// Interrupts the runner to perform an [`Action`].
    ///
    /// # Errors
    /// See [`smol::channel::Sender::try_send`].
    pub fn interrupt(&mut self, action: Action) -> Result<(), TrySendError<Action>> {
        self.tx.try_send(action)
    }
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
    /// Normal state
    Running,
    /// Stores the remaining [`Duration`] of the last [`Command`].
    Paused(Duration),
    /// Received an [`Action::Exec`] and is executing that [`Command`].
    Execute(Command),
}
