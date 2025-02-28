//! Finds playlist files in some given search path.
#![warn(clippy::pedantic)]

use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum PlaylistError {
    FileNotFound,
}
impl Error for PlaylistError {}
impl Display for PlaylistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaylistError::FileNotFound => write!(f, "Cannot find the playlist file"),
        }
    }
}

/// Searches the given playlist in the given search path.
///
/// # Return
/// If `filename` is a fully qualified path to an existing file, just open that file and return it wrapped with `Ok()`. `search_path` will be ignored in this case.
/// Otherwise, first tries to find a file relative to `search_path` with exactly the same name.
/// Finally, tries to find a file relative to `search_path` with name `filename.playlist`.
///
/// # Errors
/// If none of these approaches can find a playlist file, returns `PlaylistError::FileNotFound`.
pub fn find(filename: &Path, search_path: &Path) -> Result<File, PlaylistError> {
    // Fully qualified path
    if filename.is_file() {
        return Ok(File::open(filename).map_err(|_| PlaylistError::FileNotFound))?;
    }

    // Relative to default with extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    if real_path.is_file() {
        return Ok(File::open(real_path).map_err(|_| PlaylistError::FileNotFound))?;
    }

    // Relative to default without extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    let mut temp = real_path.into_os_string();
    temp.push(".playlist");
    let real_path = PathBuf::from(temp);
    if real_path.is_file() {
        return Ok(File::open(real_path).map_err(|_| PlaylistError::FileNotFound))?;
    }

    Err(PlaylistError::FileNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn find_playlist() {
        let mut content: String = String::from("");
        // Fully qualified path
        find(
            &PathBuf::from("./playlists/open_test.playlist"),
            &PathBuf::from("Nothing"),
        )
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
        assert_eq!(content, "=)\n");

        // No extension
        content.clear();
        find(&PathBuf::from("open_test"), &PathBuf::from("playlists"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "=)\n");

        // With extension
        content.clear();
        find(
            &PathBuf::from("open_test.playlist"),
            &PathBuf::from("playlists"),
        )
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
        assert_eq!(content, "=)\n");
    }
}
