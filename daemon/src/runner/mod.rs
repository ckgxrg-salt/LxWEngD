//! Each runner holds a playlist file and executes it.
//!
//! Errors are printed to stderr and the runner will continue to operate.

mod pause;
mod subprocess;

use crate::{commands::Command, utils::playlist};

use smol::channel::{Receiver, Sender, TrySendError};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use thiserror::Error;

pub struct Runner {
    name: String,
    index: usize,
    path: PathBuf,
    commands: Vec<Command>,
    default: HashMap<String, String>,

    channel: (Sender<Action>, Receiver<Action>),
}

#[derive(Debug, PartialEq, Error)]
pub enum RunnerError {
    #[error("runner init failed")]
    InitFailed,
    #[error("cannot spawn `linux-wallpaperengine`")]
    CannotSpawn,
    #[error("`linux-wallpaperengine` unexpectedly exited")]
    EngineDied,
}

/// Runner's state
enum State {
    Running,
    /// Stores [`Duration`] that has elapsed since last command.
    Paused(Duration),
    /// Received a [`RunnerAction`] and is executing that command.
    Execute(Command),
}

/// An action can be requested for the [`Runner`] to perform.
pub enum Action {
    /// Jump to next [`Command`].
    Next,
    /// Jump to previous [`Command`].
    Prev,
    /// Jump to a certain [`Command`].
    Goto(usize),

    /// Execute a [`Command`] specified by the user manually.
    Exec(Command),
    /// Pause current [`Command`]. bool indicates whether to SIGHUP the child instead
    /// of terminating.
    Pause(bool),

    /// Resumes normal operation.
    Resume,

    /// Terminates the [`Runner`] because of user request.
    Exit,
    /// Terminates the [`Runner`] because of an error.
    Error,
}

impl Runner {
    /// Creates a new Runner that operates the given playlist.
    ///
    /// # Errors
    /// If the given path is either invalid or cannot be opened, this returns a
    /// [`RunnerError::InitFailed`].
    pub fn new(name: String, path: PathBuf) -> Result<Self, RunnerError> {
        match playlist::open(&path) {
            Ok(file) => Ok(Self {
                commands: playlist::parse(&path, &file),
                name,
                index: 0,
                path,
                default: HashMap::new(),

                channel: smol::channel::unbounded(),
            }),
            Err(err) => {
                log::error!("{err}");
                Err(RunnerError::InitFailed)
            }
        }
    }

    /// The main runner task.
    pub async fn run(&mut self) {
        if self.commands.is_empty() {
            log::error!("no commands to execute, exiting");
            return;
        }

        loop {
            if self.index > self.commands.len() {
                self.index = 0;
            }
            let Some(current_cmd) = self.commands.get(self.index) else {
                self.index += 1;
                continue;
            };
            self.index += 1;
            match current_cmd {
                Command::Wallpaper(id, duration, forever, properties) => {}
                Command::Wait(duration) => {
                    smol::Timer::after(*duration).await;
                }
                Command::Default(properties) => {
                    properties.clone_into(&mut self.default);
                }
                Command::End => {
                    break;
                }
            }
        }
    }

    /// Requests the runner to perform an [`RunnerAction`].
    /// Doing so will interrupt this [`Runner`]'s current task.
    ///
    /// # Errors
    /// See [`smol::channel::Sender::try_send`].
    /// If an error happens here, the [`Runner`] should be terminated forcefully.
    pub fn request_action(&mut self, action: Action) -> Result<(), TrySendError<Action>> {
        self.channel.0.try_send(action)
    }
}
