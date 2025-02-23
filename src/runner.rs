//! Each runner holds a playlist file and executes it.
//!
//! The runner may fail to initialise due to some errors, if this happens, the runner will report
//! to the main thread using [`DaemonRequest::Abort`].
//! Other errors are printed to stderr and the runner will continue to operate.
#![warn(clippy::pedantic)]

use duration_str::HumanFormat;

use crate::commands::{identify, Command};
use crate::playlist;
use crate::wallpaper;
use crate::DaemonRequest;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

pub struct Runner<'a> {
    // Basic info
    id: u8,
    file: PathBuf,
    index: usize,
    channel: mpsc::Sender<DaemonRequest>,

    // Flag
    dry_run: bool,

    // Runtime info
    search_path: &'a Path,
    cache_path: &'a Path,
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
        file: PathBuf,
        search_path: &'a Path,
        cache_path: &'a Path,
        channel: mpsc::Sender<DaemonRequest>,
        dry_run: bool,
    ) -> Self {
        Self {
            id,
            file,
            channel,
            index: 0,
            search_path,
            cache_path,
            assets_path: None,
            stored_gotos: Vec::new(),
            monitor: None,
            dry_run,
            default: HashMap::new(),
        }
    }

    /// Sets the assets path of this runner
    pub fn assets_path(&mut self, path: Option<&'a Path>) -> &mut Self {
        self.assets_path = path;
        self
    }

    /// The thread main method.
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
        let Ok(mut raw_file) = playlist::find(&self.file, self.search_path) else {
            // Aborts if no file found
            self.channel
                .send(DaemonRequest::Abort(
                    self.id,
                    RuntimeError::FileNotFound(self.file.clone()),
                ))
                .unwrap();
            return;
        };
        let mut lines: Vec<String> = BufReader::new(&raw_file)
            .lines()
            .map(|line| {
                line.unwrap_or_else(|err| {
                    eprintln!(
                        "\"{0}\" line {1}: {2}, ignoring",
                        self.file.to_str().unwrap(),
                        self.index,
                        err
                    );
                    String::new()
                })
                .trim()
                .to_string()
            })
            .collect();
        loop {
            let Some(current_line) = lines.get(self.index) else {
                self.index = 0;
                continue;
            };
            self.index += 1;
            // Ignore comments
            if current_line.starts_with('#') || current_line.is_empty() {
                continue;
            };
            let cmd = match identify(current_line) {
                Ok(cmd) => cmd,
                Err(err) => {
                    eprintln!(
                        "\"{0}\" line {1}: {2}, skipping",
                        self.file.to_str().unwrap(),
                        self.index,
                        err
                    );
                    continue;
                }
            };
            match cmd {
                Command::Wallpaper(id, duration, forever, properties) => {
                    let cmd = wallpaper::get_cmd(
                        id,
                        self.cache_path,
                        self.assets_path,
                        self.monitor.as_deref(),
                        &properties,
                        &self.default,
                    );
                    if forever {
                        if self.dry_run {
                            println!(
                                "{0} line {1}: Display wallpaper ID: {2} forever",
                                self.file.to_string_lossy(),
                                self.index,
                                id,
                            );
                            println!("Run: {}", cmd.to_cmdline_lossy());
                            self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                            break;
                        }
                        let err = wallpaper::summon_forever(cmd);
                        eprintln!(
                            "{0} line {1}: {2}, skipping",
                            self.file.to_string_lossy(),
                            self.index,
                            err
                        );
                        continue;
                    }
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Display wallpaper ID: {2} for {3}",
                            self.file.to_string_lossy(),
                            self.index,
                            id,
                            duration.human_format()
                        );
                        println!("Run: {}", cmd.to_cmdline_lossy());
                        thread::sleep(duration);
                        continue;
                    }
                    if let Err(err) = wallpaper::summon(cmd, duration) {
                        eprintln!(
                            "{0} line {1}: {2}, skipping",
                            self.file.to_string_lossy(),
                            self.index,
                            err
                        );
                        continue;
                    };
                }
                Command::Wait(duration) => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Sleep for {2}",
                            self.file.to_string_lossy(),
                            self.index,
                            duration.human_format()
                        );
                    }
                    thread::sleep(duration);
                }
                Command::Goto(line, count) => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Goto line {2}",
                            self.file.to_string_lossy(),
                            self.index,
                            line
                        );
                    }
                    if count != 0 {
                        self.cache_goto(line, count);
                    } else {
                        self.index = line - 1;
                    }
                    continue;
                }
                Command::Summon(path) => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Summon a new runner for playlist {2}",
                            self.file.to_string_lossy(),
                            self.index,
                            path.to_string_lossy()
                        );
                    }
                    self.channel.send(DaemonRequest::NewRunner(path)).unwrap();
                }
                Command::Replace(path) => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Replace the playlist with {2}",
                            self.file.to_string_lossy(),
                            self.index,
                            path.to_string_lossy()
                        );
                    }
                    self.file = path;
                    let Ok(new_file) = playlist::find(&self.file, self.search_path) else {
                        // Aborts if no file found
                        self.channel
                            .send(DaemonRequest::Abort(
                                self.id,
                                RuntimeError::FileNotFound(self.file.clone()),
                            ))
                            .unwrap();
                        return;
                    };
                    raw_file = new_file;
                    lines = BufReader::new(&raw_file)
                        .lines()
                        .map(|line| {
                            line.unwrap_or_else(|err| {
                                eprintln!(
                                    "\"{0}\" line {1}: {2}, ignoring",
                                    self.file.to_str().unwrap(),
                                    self.index,
                                    err
                                );
                                String::new()
                            })
                            .trim()
                            .to_string()
                        })
                        .collect();
                    self.index = 0;
                    continue;
                }
                Command::Default(properties) => self.default = properties,
                Command::Monitor(name) => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Operate on monitor {2}",
                            self.file.to_string_lossy(),
                            self.index,
                            name
                        );
                    }
                    self.monitor = Some(name);
                }
                Command::End => {
                    if self.dry_run {
                        println!(
                            "{0} line {1}: Reached the end",
                            self.file.to_string_lossy(),
                            self.index
                        );
                    }
                    self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                    break;
                }
            }
        }
    }

    fn search_cached_gotos(&self, line: usize) -> Option<usize> {
        for (index, any) in self.stored_gotos.iter().enumerate() {
            if any.location == line {
                return Some(index);
            }
        }
        None
    }
    fn cache_goto(&mut self, line: usize, count: u32) {
        if let Some(index) = self.search_cached_gotos(line) {
            let existing = self.stored_gotos.get_mut(index).unwrap();
            if existing.remaining <= 1 {
                self.stored_gotos.remove(index);
                if self.dry_run {
                    println!("This goto is no longer effective");
                }
            } else {
                existing.remaining -= 1;
                self.index = line - 1;
                if self.dry_run {
                    println!("Remaining times for this goto: {0}", existing.remaining);
                }
            }
        } else {
            if self.dry_run {
                println!("Remaining times for this goto: {count}");
            }
            let cached = StoredGoto {
                location: line,
                remaining: count,
            };
            self.stored_gotos.push(cached);
            self.index = line - 1;
        }
    }
}
