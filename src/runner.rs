//! Each runner holds a playlist file and execute it until quits.

use crate::commands::{identify, Command};
use crate::playlist;
use std::error::Error;
use std::fmt::Display;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

struct Runner {
    file: PathBuf,
    search_path: PathBuf,
    index: usize,
    command: Command,
    stored_gotos: Vec<StoredGoto>,
}

struct StoredGoto {
    location: usize,
    remaining: u32,
}

#[derive(Debug, PartialEq)]
enum RuntimeError {}
impl Error for RuntimeError {}
impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TODO")
    }
}

impl Runner {
    /// Creates a new Runner that operates the given playlist.
    pub fn new(file: PathBuf, search_path: PathBuf) -> Self {
        Self {
            file,
            search_path,
            // This is the index of array, not the line number
            index: 0,
            // Just a placeholder
            command: Command::End,
            stored_gotos: Vec::new(),
        }
    }

    /// The thread main function
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let raw_file = playlist::find(&self.file, &self.search_path)?;
        let lines: Vec<String> = BufReader::new(&raw_file)
            .lines()
            .filter(|line| line.as_ref().is_ok_and(|val| val.is_empty()))
            .map(|line| line.unwrap())
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
                Command::Wallpaper(id, duration) => self.wallpaper(),
                Command::Wait(duration) => self.wait(),
                Command::Goto(line, count) => {
                    if count != 0 {
                        self.goto(&line, &count);
                    }
                    self.index = line;
                    continue;
                }
                Command::Summon(path) => self.summon(),
                Command::Replace(path) => self.replace(),
                Command::End => break Ok(()),
            }
        }
    }

    fn wallpaper(&self) {}
    fn wait(&self) {}

    fn search_cached_gotos(&self, line: &usize) -> Option<usize> {
        for (index, each) in self.stored_gotos.iter().enumerate() {
            if each.location == *line {
                return Some(index);
            }
        }
        None
    }
    fn goto(&mut self, line: &usize, count: &u32) {
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
