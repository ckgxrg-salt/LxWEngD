//! Each runner holds a playlist file and executes it.
//!
//! The runner may fail to initialise due to some errors, if this happens, the runner will report
//! to the main thread using [`DaemonRequest::Abort`].
//! Other errors are printed to stderr and the runner will continue to operate.

use crate::commands::Command;
use crate::playlist;
use crate::wallpaper;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct Runner<'a> {
    // Basic info
    id: u8,
    file: PathBuf,
    commands: BTreeMap<usize, Command>,
    index: usize,

    // Runtime info
    search_path: &'a Path,
    cache_path: &'a Path,
    binary: Option<&'a str>,
    assets_path: Option<&'a Path>,
    monitor: Option<String>,
    default: HashMap<String, String>,
}

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    FileNotFound(PathBuf),
    InitFailed,
    EngineDied,
}
impl Error for RuntimeError {}
impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::FileNotFound(path) => {
                write!(
                    f,
                    "Cannot find playlist file \"{}\"",
                    path.to_str().unwrap()
                )
            }
            RuntimeError::EngineDied => {
                write!(f, "linux-wallpaperengine unexpectedly exited")
            }
            RuntimeError::InitFailed => {
                write!(f, "Initialisation process failed")
            }
        }
    }
}

impl<'a> Runner<'a> {
    /// Creates a new Runner that operates the given playlist.
    #[must_use]
    pub fn new(id: u8, search_path: &'a Path, cache_path: &'a Path) -> Self {
        Self {
            id,
            file: PathBuf::new(),
            index: 0,
            search_path,
            cache_path,
            binary: None,
            assets_path: None,
            commands: BTreeMap::new(),
            monitor: None,
            default: HashMap::new(),
        }
    }

    /// Sets the assets path of this runner
    pub fn assets_path(&mut self, path: Option<&'a Path>) -> &mut Self {
        self.assets_path = path;
        self
    }
    /// Sets the binary path of this runner
    pub fn binary(&mut self, path: Option<&'a str>) -> &mut Self {
        self.binary = path;
        self
    }

    /// Sets the playlist file of this runner.
    ///
    /// # Panics
    /// When the runner needs to report to the main thread using its channel, but the
    /// channel is already closed, panic will occur.
    /// However, if the channel is already closed, it's impossible to make a graceful exit since
    /// there's no way to send a [`DaemonRequest::Abort`] or [`DaemonRequest::Exit`].   
    pub fn init(&mut self, path: PathBuf) -> &mut Self {
        let Ok(file) = playlist::find(&path, self.search_path) else {
            log::error!("Cannot find playlist file {}", path.to_string_lossy());
            return self;
        };
        self.commands = playlist::parse(&path, &file);
        self.file = path;
        self
    }

    /// Starts the job of the runner.
    /// This is just a wrapper that determines whether a Runner normally runs or dry-runs.
    pub fn dispatch(&mut self) {
        self.run();
    }

    /// The main runner method.
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
        let cmd = wallpaper::get_cmd(
            id,
            self.cache_path,
            self.binary,
            self.assets_path,
            self.monitor.as_deref(),
            properties,
            &self.default,
        );
        if forever {
            let err = wallpaper::summon_forever(cmd);
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.file.to_string_lossy(),
                self.index,
                err
            );
            return;
        }
        if let Err(err) = wallpaper::summon(cmd, duration) {
            log::warn!(
                "{0} line {1}: {2}, skipping",
                self.file.to_string_lossy(),
                self.index,
                err
            );
        }
    }
}
