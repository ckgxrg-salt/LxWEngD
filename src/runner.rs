//! Each runner holds a playlist file and executes it.
//!
//! The runner may fail to initialise due to some errors, if this happens, the runner will report
//! to the main thread using [`DaemonRequest::Abort`].
//! Other errors are printed to stderr and the runner will continue to operate.
#![warn(clippy::pedantic)]

use duration_str::HumanFormat;

use crate::commands::Command;
use crate::playlist;
use crate::resume;
use crate::wallpaper;
use crate::DaemonRequest;

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
    channel: mpsc::Sender<DaemonRequest>,

    // Runtime info
    search_path: &'a Path,
    cache_path: &'a Path,
    binary: Option<&'a str>,
    assets_path: Option<&'a Path>,
    stored_gotos: Vec<StoredGoto>,
    monitor: Option<String>,
    default: HashMap<String, String>,
}

struct StoredGoto {
    location: usize,
    remaining: u32,
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
    pub fn new(
        id: u8,
        search_path: &'a Path,
        cache_path: &'a Path,
        channel: mpsc::Sender<DaemonRequest>,
    ) -> Self {
        Self {
            id,
            channel,
            file: PathBuf::new(),
            index: 0,
            search_path,
            cache_path,
            binary: None,
            assets_path: None,
            commands: BTreeMap::new(),
            stored_gotos: Vec::new(),
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
    pub fn dispatch(&mut self, dry_run: bool) {
        if dry_run {
            self.dry_run();
        } else {
            self.run();
        }
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
        let mut replace: Option<PathBuf> = None;
        let (&last_line_num, _) = self.commands.last_key_value().unwrap();

        match resume::load_state(&self.file, last_line_num) {
            Ok(value) => self.index = value,
            Err(resume::ResumeError::ExceedMaxLine) => {
                log::warn!("Loaded saved state, but the stored line number is invalid");
            }
            Err(_) => (),
        }

        loop {
            if self.index > last_line_num {
                self.index = 0;
            }
            let Some(current_cmd) = self.commands.get(&self.index) else {
                self.index += 1;
                continue;
            };
            if resume::save_state(self.index, &self.file).is_err() {
                log::warn!("Failed to save current state due to unknown error");
            };
            self.index += 1;
            match current_cmd {
                Command::Wallpaper(id, duration, forever, properties) => {
                    self.summon_wallpaper(*id, *duration, *forever, properties);
                }
                Command::Wait(duration) => {
                    thread::sleep(*duration);
                }
                Command::Goto(line, count) => {
                    if *count == 0 {
                        self.index = line - 1;
                    } else if let Some(index) = self.search_cached_gotos(*line) {
                        let existing = self.stored_gotos.get_mut(index).unwrap();
                        if existing.remaining <= 1 {
                            self.stored_gotos.remove(index);
                        } else {
                            existing.remaining -= 1;
                            self.index = line - 1;
                        }
                    } else {
                        let cached = StoredGoto {
                            location: *line,
                            remaining: *count,
                        };
                        self.stored_gotos.push(cached);
                        self.index = line - 1;
                    }
                }
                Command::Summon(path) => {
                    self.channel
                        .send(DaemonRequest::NewRunner(path.clone()))
                        .unwrap();
                }
                Command::Replace(path) => {
                    replace = Some(path.clone());
                    if resume::clear_state(&self.file).is_err() {
                        log::warn!("Failed to clear current state due to unknown error");
                    };
                    break;
                }
                Command::Default(properties) => {
                    self.default = properties.to_owned();
                }
                Command::Monitor(name) => {
                    self.monitor = Some(name.to_string());
                }
                Command::End => {
                    if resume::clear_state(&self.file).is_err() {
                        log::warn!("Failed to clear current state due to unknown error");
                    };
                    self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                    break;
                }
            }
        }
        if let Some(value) = replace {
            self.init(value).run();
        }
    }

    /// The main runner method, but prints what would be done instead of really doing so.
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
    #[allow(clippy::too_many_lines)]
    pub fn dry_run(&mut self) {
        if self.commands.is_empty() {
            log::error!("No commands to execute, exiting");
            self.channel
                .send(DaemonRequest::Abort(self.id, RuntimeError::InitFailed))
                .unwrap();
            return;
        }
        // We cannot modify the Runner state inside the `match` block, so we save the information and do it
        // later.
        let mut replace: Option<PathBuf> = None;
        let (&last_line_num, _) = self.commands.last_key_value().unwrap();

        loop {
            if self.index > last_line_num {
                log::trace!("This playlist is infinite, exiting");
                self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                return;
            }
            let Some(current_cmd) = self.commands.get(&self.index) else {
                self.index += 1;
                continue;
            };
            self.index += 1;
            match current_cmd {
                Command::Wallpaper(id, duration, forever, properties) => {
                    self.dry_summon_wallpaper(*id, *duration, *forever, properties);
                }
                Command::Wait(duration) => {
                    log::trace!(
                        "{0} line {1}: Sleep for {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        duration.human_format()
                    );
                }
                Command::Goto(line, count) => {
                    log::trace!(
                        "{0} line {1}: Goto line {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        line
                    );
                    if *count == 0 {
                        log::trace!("This goto is infinite, exiting");
                        self.index = line - 1;
                    } else if let Some(index) = self.search_cached_gotos(*line) {
                        let existing = self.stored_gotos.get_mut(index).unwrap();
                        if existing.remaining <= 1 {
                            self.stored_gotos.remove(index);
                            log::trace!("This goto is no longer effective");
                        } else {
                            existing.remaining -= 1;
                            self.index = line - 1;
                            log::trace!("Remaining times for this goto: {0}", existing.remaining);
                        }
                    } else {
                        log::trace!("Remaining times for this goto: {count}");
                        let cached = StoredGoto {
                            location: *line,
                            remaining: *count,
                        };
                        self.stored_gotos.push(cached);
                        self.index = line - 1;
                    }
                }
                Command::Summon(path) => {
                    log::trace!(
                        "{0} line {1}: Summon a new runner for playlist {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        path.to_string_lossy()
                    );
                    self.channel
                        .send(DaemonRequest::NewRunner(path.clone()))
                        .unwrap();
                }
                Command::Replace(path) => {
                    log::trace!(
                        "{0} line {1}: Replace the playlist with {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        path.to_string_lossy()
                    );
                    replace = Some(path.clone());
                    break;
                }
                Command::Default(properties) => {
                    log::trace!(
                        "{0} line {1}: Set default properties: {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        wallpaper::pretty_print(properties)
                    );
                    self.default = properties.to_owned();
                }
                Command::Monitor(name) => {
                    log::trace!(
                        "{0} line {1}: Operate on monitor {2}",
                        self.file.to_string_lossy(),
                        self.index,
                        name
                    );
                    self.monitor = Some(name.to_string());
                }
                Command::End => {
                    log::trace!(
                        "{0} line {1}: Reached the end",
                        self.file.to_string_lossy(),
                        self.index
                    );
                    self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                    break;
                }
            }
        }
        if let Some(value) = replace {
            self.init(value).dry_run();
        }
    }

    /// Prints what command would be executed to summon a wallpaper
    fn dry_summon_wallpaper(
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
            log::trace!(
                "{0} line {1}: Display wallpaper ID: {2} forever",
                self.file.to_string_lossy(),
                self.index,
                id,
            );
            log::trace!("Run: {}", cmd.to_cmdline_lossy());
            self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
            return;
        }
        log::trace!(
            "{0} line {1}: Display wallpaper ID: {2} for {3}",
            self.file.to_string_lossy(),
            self.index,
            id,
            duration.human_format()
        );
        log::trace!("Run: {}", cmd.to_cmdline_lossy());
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
        };
    }

    fn search_cached_gotos(&self, line: usize) -> Option<usize> {
        for (index, any) in self.stored_gotos.iter().enumerate() {
            if any.location == line {
                return Some(index);
            }
        }
        None
    }
}
