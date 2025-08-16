//! Finds playlist files in some given search path.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::cli::SEARCH_PATH;
use crate::commands::{Command, identify};

#[derive(Debug, PartialEq, Error)]
#[error("cannot find playlist file `{0}`")]
pub struct FileNotFound(PathBuf);

/// Searches the given playlist in the given search path.
///
/// # Return
/// If `filename` is a fully qualified path to an existing file, just open that file and return it wrapped with `Ok()`. `search_path` will be ignored in this case.
/// Otherwise, first tries to find a file relative to `search_path` with exactly the same name.
/// Finally, tries to find a file relative to `search_path` with name `filename.playlist`.
///
/// # Errors
/// If none of these approaches can find a playlist file, returns `FileNotFound`.
pub fn open(filename: &Path) -> Result<File, FileNotFound> {
    // Fully qualified path
    if filename.is_file() {
        return Ok(File::open(filename).map_err(|_| FileNotFound(filename.to_path_buf())))?;
    }

    // Relative to default with extension
    let mut real_path = SEARCH_PATH.to_path_buf();
    real_path.push(filename);
    if real_path.is_file() {
        return Ok(File::open(&real_path).map_err(|_| FileNotFound(real_path)))?;
    }

    // Relative to default without extension
    let mut real_path = SEARCH_PATH.to_path_buf();
    real_path.push(filename);
    let mut temp = real_path.into_os_string();
    temp.push(".playlist");
    let real_path = PathBuf::from(temp);
    if real_path.is_file() {
        return Ok(File::open(&real_path).map_err(|_| FileNotFound(real_path)))?;
    }

    Err(FileNotFound(filename.to_path_buf()))
}

/// Parses a playlist file and generates a list of [`Command`]
/// This process will load the playlist file into memory, parse it, and generate a list of
/// [`Command`].
#[must_use]
pub fn parse(path: &Path, file: &File) -> BTreeMap<usize, Command> {
    let mut commands = BTreeMap::new();
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(num, line)| {
            line.unwrap_or_else(|err| {
                log::warn!(
                    "{0} line {1}: {2}, ignoring",
                    path.to_string_lossy(),
                    num + 1,
                    err
                );
                String::new()
            })
            .trim()
            .to_string()
        })
        .collect();
    for (num, each) in lines.iter().enumerate() {
        // Ignore comments
        if each.starts_with('#') || each.is_empty() {
            continue;
        };
        match identify(each) {
            Ok(cmd) => {
                commands.insert(num, cmd);
            }
            Err(err) => {
                log::warn!(
                    "{0} line {1}: {2}, skipping",
                    path.to_string_lossy(),
                    num + 1,
                    err
                );
            }
        };
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Read;
    use std::time::Duration;

    #[test]
    fn find_playlist() {
        let mut content: String = String::new();
        // Fully qualified path
        open(&PathBuf::from("./playlists/open_test.playlist"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "=)\n");

        // No extension
        content.clear();
        open(&PathBuf::from("open_test"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "=)\n");

        // With extension
        content.clear();
        open(&PathBuf::from("open_test.playlist"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "=)\n");
    }

    #[test]
    fn parse_playlist() {
        let playlist = PathBuf::from("./playlists/default.playlist");
        let commands = parse(&playlist, &open(&playlist).unwrap());

        let expected = vec![
            Command::Wallpaper(1, Duration::from_secs(15 * 60), false, HashMap::new()),
            Command::Wallpaper(2, Duration::from_secs(60 * 60), false, HashMap::new()),
            Command::Wallpaper(3, Duration::from_secs(360), false, HashMap::new()),
            Command::Wait(Duration::from_secs(5 * 60)),
            Command::End,
        ]
        .into_iter()
        .enumerate()
        .collect();
        assert_eq!(commands, expected);
    }
}
