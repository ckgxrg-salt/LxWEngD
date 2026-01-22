//! Finds playlist files in some given search path.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::command::{Command, parse as parse_cmd};
use crate::daemon::SEARCH_PATH;

/// Searches the given playlist in the given search path.
///
/// # Return
/// If `filename` is a fully qualified path to an existing file, just open that file and return it wrapped with `Ok()`. `search_path` will be ignored in this case.
/// Otherwise, first tries to find a file relative to `search_path` with exactly the same name.
/// Finally, tries to find a file relative to `search_path` with name `filename.playlist`.
///
/// # Errors
/// If none of these approaches can find a playlist file, returns an [`std::io::Error`].
pub fn open(filename: &Path) -> std::io::Result<File> {
    // Fully qualified path
    if filename.is_file() {
        return Ok(File::open(filename)?);
    }

    // Relative to default with extension
    let mut real_path = SEARCH_PATH.to_path_buf();
    real_path.push(filename);
    if real_path.is_file() {
        return Ok(File::open(&real_path)?);
    }

    // Relative to default without extension
    let mut real_path = SEARCH_PATH.to_path_buf();
    real_path.push(filename);
    let mut temp = real_path.into_os_string();
    temp.push(".playlist");
    let real_path = PathBuf::from(temp);
    Ok(File::open(&real_path)?)
}

/// Parses a playlist file and generates a list of [`Command`]s.
pub fn parse(path: &Path, file: &File) -> Vec<Command> {
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(line_no, line)| match line {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    None
                } else {
                    match parse_cmd(trimmed) {
                        Ok(cmd) => Some(cmd),
                        Err(err) => {
                            log::warn!(
                                "{}:{} error: {}, skipping",
                                path.to_string_lossy(),
                                line_no + 1,
                                err
                            );
                            None
                        }
                    }
                }
            }
            Err(err) => {
                log::warn!(
                    "{}:{} error: {}, ignoring",
                    path.to_string_lossy(),
                    line_no + 1,
                    err,
                );
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::command::WallpaperDuration;
    use std::collections::HashMap;
    use std::io::Read;
    use std::time::Duration;

    #[test]
    fn find_playlist() {
        let mut content: String = String::new();
        // Fully qualified path
        open(&PathBuf::from("../playlists/open_test.playlist"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "=)\n");
    }

    #[test]
    fn parse_playlist() {
        let playlist = PathBuf::from("../playlists/default.playlist");
        let commands = parse(&playlist, &open(&playlist).unwrap());

        let expected = vec![
            Command::Wallpaper(
                "1".to_string(),
                WallpaperDuration::Finite(Duration::from_secs(15 * 60)),
                HashMap::new(),
            ),
            Command::Wallpaper(
                "2".to_string(),
                WallpaperDuration::Finite(Duration::from_secs(60 * 60)),
                HashMap::new(),
            ),
            Command::Wallpaper(
                "3".to_string(),
                WallpaperDuration::Finite(Duration::from_secs(360)),
                HashMap::new(),
            ),
            Command::Sleep(WallpaperDuration::Finite(Duration::from_secs(5 * 60))),
            Command::End,
        ];
        assert_eq!(commands, expected);
    }
}
