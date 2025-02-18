//! Each runner holds a playlist file and execute it until quits.

use crate::commands::{identify, Command};
use crate::playlist;
use crate::wallpaper;
use std::error::Error;
use std::fmt::Display;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub struct Runner<'a> {
    file: PathBuf,
    search_path: PathBuf,
    assets_path: Option<&'a Path>,
    index: usize,
    stored_gotos: Vec<StoredGoto>,
    monitor: Option<String>,
}

struct StoredGoto {
    location: usize,
    remaining: u32,
}

#[derive(Debug, PartialEq)]
pub enum RuntimeError {
    EngineDied,
}
impl Error for RuntimeError {}
impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TODO")
    }
}

impl<'a> Runner<'a> {
    /// Creates a new Runner that operates the given playlist.
    pub fn new(file: PathBuf, search_path: PathBuf) -> Self {
        Self {
            file,
            search_path,
            assets_path: None,
            // This is the index of array, not the line number
            index: 0,
            // Just a placeholder
            stored_gotos: Vec::new(),
            monitor: None,
        }
    }

    /// Sets the assets path of this runner
    pub fn assets_path(&mut self, path: &'a Path) -> &mut Self {
        self.assets_path = Some(path);
        self
    }

    /// The thread main function
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut raw_file = playlist::find(&self.file, &self.search_path)?;
        let mut content = String::new();
        let lines: Vec<String> = BufReader::new(&raw_file)
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
                    eprintln!("{}", err);
                    continue;
                }
            };
            match cmd {
                Command::Wallpaper(id, duration) => {
                    let cmd = wallpaper::get_cmd(
                        id,
                        self.assets_path,
                        self.monitor.as_ref().map(|x| x.as_str()),
                    );
                    wallpaper::summon(cmd, duration)?;
                }
                Command::Wait(duration) => thread::sleep(duration),
                Command::Goto(line, count) => {
                    if count != 0 {
                        self.cache_goto(&line, &count);
                    }
                    self.index = line;
                    continue;
                }
                Command::Summon(path) => self.summon(),
                Command::Replace(path) => self.replace(),
                Command::Monitor(name) => self.monitor = Some(name),
                Command::End => break Ok(()),
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

    fn summon(&self) {}
    fn replace(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goto() {
        let instance = Runner::new(PathBuf::from("nothing"), PathBuf::from("nothing"));
    }
}
