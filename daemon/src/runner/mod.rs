mod actions;
mod commands;
mod exec;
mod imp;

pub use actions::Action;
pub use commands::{CmdDuration, Command};

use smol::channel::{Receiver, Sender, TrySendError};
use smol::lock::Mutex;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::backend::Backend;
use exec::ExecInfo;

/// The special monitor name to indicate this runner has no associated monitor.
pub const NOMONITOR_INDICATOR: &str = "NOMONITOR";

pub struct RunnerHandle {
    index: usize,
    commands: Vec<Command>,
    state: State,

    path: PathBuf,

    tx: Sender<Action>,
}

/// Data structure of a runner.
pub struct Runner {
    internal: Arc<Mutex<RunnerHandle>>,
    backend: Backend,
    rx: Receiver<Action>,
}

impl RunnerHandle {
    /// Interrupts the runner to perform an [`Action`].
    ///
    /// # Errors
    /// See [`smol::channel::Sender::try_send`].
    pub fn interrupt(&mut self, action: Action) -> Result<(), TrySendError<Action>> {
        self.tx.try_send(action)
    }

    /// Returns whether this [`Runner`] has exited.
    pub fn exited(&self) -> bool {
        matches!(self.state, State::Exited)
    }
}

impl Display for RunnerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{} - Index {}",
            self.state,
            self.path.to_string_lossy(),
            self.index
        )
    }
}

impl Runner {
    async fn next(&self) {
        self.internal.lock().await.index += 1;
    }

    async fn prev(&self) {
        self.internal.lock().await.index -= 1;
    }

    async fn goto(&self, index: usize) {
        self.internal.lock().await.index = index;
    }

    async fn update_state(&self, state: State) {
        self.internal.lock().await.state = state;
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
    #[error("Failed to cleanup")]
    CleanupFail,
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
    /// This includes oneshot commands
    Ready,
    Running(ExecInfo),
    Paused(Option<Duration>),
    Exited,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            State::Ready => "Ready",
            // TODO: Use humantime for formatting
            State::Running(info) => {
                if let Some(duration) = info.duration {
                    &format!(
                        "Running - expected to take {:?} - started at {:?}",
                        duration, info.start
                    )
                } else {
                    &format!("Running - started at {:?}", info.start)
                }
            }
            State::Paused(_) => "Paused",
            State::Exited => "Exited",
        };
        write!(f, "{string}")
    }
}
