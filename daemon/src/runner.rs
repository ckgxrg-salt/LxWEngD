//! Each runner holds a playlist file and executes it.
//!
//! Errors are printed to stderr and the runner will continue to operate.

use crate::{commands::Command, playlist, wallpaper};

use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use smol::channel::{Receiver, Sender, TrySendError};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
use thiserror::Error;

pub struct Runner {
    name: String,
    index: usize,
    path: PathBuf,
    commands: BTreeMap<usize, Command>,
    default: HashMap<String, String>,

    channel: (Sender<RunnerAction>, Receiver<RunnerAction>),
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

/// How the subprocess task exited.
enum ExitType {
    Success,
    Exited,
    WithAction(RunnerAction),
}

/// An action can be requested for the [`Runner`] to perform.
pub enum RunnerAction {
    /// Jump to next [`Command`]
    Next,
    /// Jump to previous [`Command`]
    Prev,
    /// Pause current [`Command`]. bool indicates whether to SIGHUP the child instead
    /// of terminating.
    Pause(bool),
    /// Similar to above, but instead of terminating child, SIGHUP them.
    Resume,
    /// Terminates the [`Runner`] because of user request
    Exit,
    /// Terminates the [`Runner`] because of an error
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
        let Some((&length, _)) = self.commands.last_key_value() else {
            log::error!("invalid playlist file");
            return;
        };

        loop {
            if self.index > length {
                self.index = 0;
            }
            let Some(current_cmd) = self.commands.get(&self.index) else {
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
    ///
    /// # Errors
    /// See [`smol::channel::Sender::try_send`].
    /// If an error happens here, the [`Runner`] should be terminated forcefully.
    pub fn request_action(
        &mut self,
        action: RunnerAction,
    ) -> Result<(), TrySendError<RunnerAction>> {
        self.channel.0.try_send(action)
    }

    /// Displays a wallpaper.
    ///
    /// # Returns
    /// - `None` if everything good.
    /// - `Some(RunnerAction)` if an action needs to be performed by the main task.
    async fn summon_wallpaper(
        &self,
        id: u32,
        properties: &HashMap<String, String>,
    ) -> Option<RunnerAction> {
        // TODO: check whether monitor is valid
        let mut cmd = wallpaper::get_cmd(id, Some(&self.name), properties, &self.default);
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();

        let exit = smol::future::race(
            async {
                let _ = child.status().await;
                ExitType::Exited
            },
            async {
                let action = self.channel.1.recv().await.unwrap_or(RunnerAction::Error);
                ExitType::WithAction(action)
            },
        )
        .await;

        match exit {
            ExitType::Exited => {
                log::warn!(
                    "{} line {}: `linux-wallpaperengine` unexpectedly exited",
                    self.path.to_string_lossy(),
                    self.index,
                );
                None
            }
            ExitType::WithAction(action) => {
                if kill(
                    Pid::from_raw(pid.try_into().expect("pid won't go that large")),
                    Signal::SIGTERM,
                )
                .is_err()
                {
                    log::warn!(
                        "{} line {}: failed to terminate `linux-wallpaperengine`",
                        self.path.to_string_lossy(),
                        self.index,
                    );
                }
                Some(action)
            }
            ExitType::Success => unreachable!(),
        }
    }
}
