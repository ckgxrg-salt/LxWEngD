//! Each runner holds a playlist file and executes it.
//!
//! Errors are printed to stderr and the runner will continue to operate.

use crate::commands::Command;
use crate::playlist;
use crate::wallpaper;

use smol::Task;
use smol::process::Command as _Command;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

pub struct Runner {
    name: String,
    index: usize,
    path: PathBuf,
    commands: BTreeMap<usize, Command>,
    default: HashMap<String, String>,
    current_task: Option<Task<()>>,
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

impl Runner {
    /// Creates a new Runner that operates the given playlist.
    pub fn new(name: String, path: PathBuf) -> Result<Self, RunnerError> {
        match playlist::open(&path) {
            Ok(file) => Ok(Self {
                commands: playlist::parse(&path, &file),
                name,
                index: 0,
                path,
                default: HashMap::new(),
                current_task: None,
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
                Command::Wallpaper(id, duration, forever, properties) => {
                    self.summon_wallpaper(*id, *duration, *forever, properties);
                }
                Command::Wait(duration) => {
                    smol::Timer::after(*duration).await;
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
        let cmd = wallpaper::get_cmd(id, &self.name, properties, &self.default);
        if forever {
            let err = summon_forever(cmd);
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.path.to_string_lossy(),
                self.index,
                err
            );
            return;
        }
        if let Err(err) = summon_duration(cmd, duration) {
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.path.to_string_lossy(),
                self.index,
                err
            );
        }
    }

    async fn display_forever(&mut self, mut cmd: _Command) -> Result<(), RunnerError> {
        let mut child = cmd.spawn().map_err(|_| RunnerError::CannotSpawn)?;
        let subprocess = smol::spawn(async move {
            child.status().await;
        });

        subprocess.await;

        Err(RunnerError::EngineDied)
    }
}
