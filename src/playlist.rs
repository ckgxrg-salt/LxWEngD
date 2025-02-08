//! This module interprets playlist files

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn config_dir() -> Result<PathBuf, String> {
    let default;
    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        default = PathBuf::from(value + "/lxwengd");
    } else if let Ok(value) = env::var("HOME") {
        default = PathBuf::from(value + "/.config/lxwengd");
    } else {
        return Err(
            "Cannot find the playlist directory, consider indicating the fully qualified path."
                .to_string(),
        );
    }
    Ok(default)
}

fn open(filename: &PathBuf, search_path: &Path) -> Result<String, String> {
    // Fully qualified path
    if let Ok(content) = fs::read_to_string(filename) {
        return Ok(content);
    }

    // Relative to default with extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    if let Ok(content) = fs::read_to_string(real_path) {
        return Ok(content);
    }

    // Relative to default without extension
    let mut real_path = search_path.to_path_buf();
    real_path.push(filename);
    let mut temp = real_path.into_os_string();
    temp.push(".playlist");
    let real_path = PathBuf::from(temp);
    if let Ok(content) = fs::read_to_string(real_path) {
        return Ok(content);
    }

    Err("Cannot find the playlist file.".to_string())
}

#[cfg(test)]
mod tests {
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
    fn open_playlist() {
        // Fully qualified path
        let full = open(
            &PathBuf::from("./playlists/open_test.playlist"),
            &PathBuf::from("Nothing"),
        )
        .unwrap();
        assert_eq!(full, "=)\n");

        // No extension
        let extless = open(&PathBuf::from("open_test"), &PathBuf::from("playlists")).unwrap();
        assert_eq!(extless, "=)\n");

        // With extension
        let extful = open(
            &PathBuf::from("open_test.playlist"),
            &PathBuf::from("playlists"),
        )
        .unwrap();
        assert_eq!(extful, "=)\n");
    }
}
