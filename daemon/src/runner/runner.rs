//! A runner is created with a list of [`Command`]s that it will execute.
//!
//! After init, it awaits on 3 tasks:
//! - Timer: which wakes it when the designated [`WallpaperDuration`] has elapsed.
//! - Command: which wakes it when the subprocess exits.
//! - Message: which wakes it when it's interrupted externally.
//!
//! A runner registers it with the daemon.
//! When it exits, it clears its own entry in the registered runners.

use std::path::PathBuf;

use crate::backends::LxWEng;
use crate::runner::exec::ExecResult;
use crate::runner::{Runner, RunnerError, State};
use crate::utils::command::{CmdDuration, Command};
use crate::utils::playlist;

/// Currently only `linux-wallpaperengine` is supported.
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
                    state: State::Running,
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
                Command::Wallpaper(name, CmdDuration::Finite(duration), properties) => {
                    self.wallpaper(name, *duration, properties).await
                }
                Command::Sleep(duration) => match duration {
                    CmdDuration::Finite(duration) => self.sleep(*duration).await,
                    CmdDuration::Infinite => self.wait_action().await,
                },
                Command::Default(properties) => {
                    self.backend.update_default_props(properties);
                    ExecResult::Done
                }
                Command::End => {
                    break;
                }
            };
        }
    }

    async fn paused(&mut self) {
        self.wait_action().await;
    }
}
