//! # Runners
//!
//! Each runner holds a playlist file and executes it.   
//!
//! The runner may fail to initialise due to some errors, if this happens, the runner will report
//! to the main thread using DaemonRequest::Abort.   
//! Other errors are printed to stderr and the runner will continue to operate.   

use crate::commands::{identify, Command};
use crate::playlist;
use crate::wallpaper;
use crate::DaemonRequest;
use std::error::Error;
use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

pub struct Runner<'a> {
    id: u8,
    file: PathBuf,
    index: usize,
    channel: mpsc::Sender<DaemonRequest>,

    search_path: PathBuf,
    assets_path: Option<&'a Path>,
    stored_gotos: Vec<StoredGoto>,
    monitor: Option<String>,
}

struct StoredGoto {
    location: usize,
    remaining: u32,
}

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    FileNotFound(PathBuf),
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
        }
    }
}

impl<'a> Runner<'a> {
    /// Creates a new Runner that operates the given playlist.
    pub fn new(
        id: u8,
        file: PathBuf,
        search_path: PathBuf,
        channel: mpsc::Sender<DaemonRequest>,
    ) -> Self {
        Self {
            id,
            file,
            channel,
            // This is the index of array, not the line number
            index: 0,
            search_path,
            assets_path: None,
            // Just a placeholder
            stored_gotos: Vec::new(),
            monitor: None,
        }
    }

    /// Sets the assets path of this runner
    pub fn assets_path(&mut self, path: Option<&'a Path>) -> &mut Self {
        self.assets_path = path;
        self
    }

    /// The thread main method   
    ///
    /// # Errors
    /// Errors that will halt the runner will be reported using DaemonRequest::Abort.   
    /// Other errors are printed to stderr, and runner skips that command.   
    pub fn run(&mut self) {
        let Ok(mut raw_file) = playlist::find(&self.file, &self.search_path) else {
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
            .map(|line| line.unwrap().trim().to_string())
            .filter(|line| !line.is_empty())
            .filter(|line| !line.starts_with("#"))
            .collect();
        loop {
            let current_line = if let Some(value) = lines.get(self.index) {
                value
            } else {
                self.index = 0;
                continue;
            };
            self.index += 1;
            let cmd = match identify(current_line) {
                Ok(cmd) => cmd,
                Err(err) => {
                    eprintln!(
                        "\"{0}\" line {1}: {2}, skipping",
                        self.file.to_str().unwrap(),
                        self.index + 1,
                        err
                    );
                    continue;
                }
            };
            match cmd {
                Command::Wallpaper(id, duration) => {
                    let cmd = wallpaper::get_cmd(id, self.assets_path, self.monitor.as_deref());
                    if let Err(err) = wallpaper::summon(cmd, duration) {
                        eprintln!(
                            "\"{0}\" line {1}: {2}, skipping",
                            self.file.to_str().unwrap(),
                            self.index + 1,
                            err
                        );
                        continue;
                    };
                }
                Command::Wait(duration) => thread::sleep(duration),
                Command::Goto(line, count) => {
                    if count != 0 {
                        self.cache_goto(&line, &count);
                    }
                    self.index = line;
                    continue;
                }
                Command::Summon(path) => {
                    self.channel.send(DaemonRequest::NewRunner(path)).unwrap();
                }
                Command::Replace(path) => {
                    self.file = path;
                    let Ok(new_file) = playlist::find(&self.file, &self.search_path) else {
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
                        .map(|line| line.unwrap().trim().to_string())
                        .filter(|line| !line.is_empty())
                        .filter(|line| !line.starts_with("#"))
                        .collect();
                    self.index = 0;
                    continue;
                }
                Command::Monitor(name) => self.monitor = Some(name),
                Command::End => {
                    self.channel.send(DaemonRequest::Exit(self.id)).unwrap();
                    break;
                }
            }
        }
    }

    fn search_cached_gotos(&self, line: &usize) -> Option<usize> {
        for (index, any) in self.stored_gotos.iter().enumerate() {
            if any.location == *line {
                return Some(index);
            }
        }
        None
    }
    fn cache_goto(&mut self, line: &usize, count: &u32) {
        if let Some(index) = self.search_cached_gotos(line) {
            let existing = self.stored_gotos.get_mut(index).unwrap();
            if existing.remaining <= 1 {
                self.stored_gotos.remove(index);
            } else {
                existing.remaining -= 1;
            }
        } else {
            let cached = StoredGoto {
                location: *line,
                remaining: *count,
            };
            self.stored_gotos.push(cached);
        }
    }
}
