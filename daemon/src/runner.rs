//! Each runner holds a playlist file and executes it.
//!
//! Errors are printed to stderr and the runner will continue to operate.

use crate::commands::Command;
use crate::playlist;
use crate::wallpaper;

use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use thiserror::Error;

pub struct Runner {
    // Basic info
    id: u8,
    index: usize,
    path: PathBuf,
    commands: Vec<Command>,
    default: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Error)]
pub enum RunnerError {
    #[error("runner init failed")]
    InitFailed,
    #[error("linux-wallpaperengine unexpectedly exited")]
    EngineDied,
}

impl Runner {
    /// Creates a new Runner that operates the given playlist.
    pub fn new(id: u8, path: PathBuf) -> Result<Self, RunnerError> {
        match playlist::open(&path) {
            Ok(file) => Ok(Self {
                commands: playlist::parse(&path, &file),
                id,
                index: 0,
                path,
                default: HashMap::new(),
            }),
            Err(err) => {
                log::error!("{err}");
                Err(RunnerError::InitFailed)
            }
        }
    }

    /// The main runner task.
    ///
    /// # Errors
    /// Errors that will halt the runner will be reported using [`DaemonRequest::Abort`].
    /// Other errors are printed to stderr, and runner skips that command.
    ///
    /// # Panics
    /// When the runner needs to report to the main thread using its channel, but the
    /// channel is already closed, panic will occur.
    /// However, if the channel is already closed, it's impossible to make a graceful exit since
    /// there's no way to send a [`DaemonRequest::Abort`] or [`DaemonRequest::Exit`].   
    pub fn run(&mut self) {
        if self.commands.is_empty() {
            log::error!("No commands to execute, exiting");
            return;
        }
        // We cannot modify the Runner state inside the `match` block, so we save the information and do it
        // later.
        let (&last_line_num, _) = self.commands.last_key_value().unwrap();

        loop {
            if self.index > last_line_num {
                self.index = 0;
            }
            let Some(current_cmd) = self.commands.get(&self.index) else {
                self.index += 1;
                continue;
            };
            self.index += 1;
            match current_cmd {
                Command::Wallpaper(id, duration, forever, properties) => {
                    self.summon_wallpaper(*id, *duration, *forever, properties);
                }
                Command::Wait(duration) => {
                    thread::sleep(*duration);
                }
                Command::Default(properties) => {
                    self.default = properties.to_owned();
                }
                Command::End => {
                    break;
                }
            }
        }
    }

    /// Displays a wallpaper.
    fn summon_wallpaper(
        &self,
        id: u32,
        duration: Duration,
        forever: bool,
        properties: &HashMap<String, String>,
    ) {
        let cmd = wallpaper::get_cmd(id, self.cache_path, properties, &self.default);
        if forever {
            let err = wallpaper::summon_forever(cmd);
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.path.to_string_lossy(),
                self.index,
                err
            );
            return;
        }
        if let Err(err) = wallpaper::summon(cmd, duration) {
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.path.to_string_lossy(),
                self.index,
                err
            );
        }
    }
}
