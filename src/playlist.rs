//! This module interprets playlist files

use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum PlaylistError {
    DirectoryNotFound,
    FileNotFound,
}
impl Error for PlaylistError {}
impl Display for PlaylistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TODO")
    }
}

pub fn config_dir() -> Result<PathBuf, PlaylistError> {
    let default;
    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        default = PathBuf::from(value + "/lxwengd");
    } else if let Ok(value) = env::var("HOME") {
        default = PathBuf::from(value + "/.config/lxwengd");
    } else {
        return Err(PlaylistError::DirectoryNotFound);
    }
    Ok(default)
}

pub fn find(filename: &Path, search_path: &Path) -> Result<File, PlaylistError> {
    // Fully qualified path
    if let Ok(content) = File::open(filename) {
        return Ok(content);
    }

    // Relative to default with extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    if let Ok(content) = File::open(real_path) {
        return Ok(content);
    }

    // Relative to default without extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    let mut temp = real_path.into_os_string();
    temp.push(".playlist");
    let real_path = PathBuf::from(temp);
    if let Ok(content) = File::open(real_path) {
        return Ok(content);
    }

    Err(PlaylistError::FileNotFound)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn playlist_location() {
        env::set_var("XDG_CONFIG_HOME", ".");
        assert_eq!(config_dir().unwrap(), PathBuf::from("./lxwengd"));
        env::remove_var("XDG_CONFIG_HOME");
        env::set_var("HOME", ".");
        assert_eq!(config_dir().unwrap(), PathBuf::from("./.config/lxwengd"));
        env::remove_var("HOME");
        assert!(config_dir().is_err());
    }

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
