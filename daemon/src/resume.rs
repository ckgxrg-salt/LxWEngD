//! Resume
//!
//! Handles behaviours when received termination signal.
//! Usually, this saves a file named "<playlist>.resume" containing the line number of where
//! terminted.
//! Later, this file can be load and restore the state of a runner.
//! This file only exists when a runner is not running, it will be removed once the runner loaded
//! the state, regardless successfully or not.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Errors may happen in resume process
#[derive(Debug, PartialEq)]
pub enum ResumeError {
    /// Failed to store resume data.
    StoreError,
    /// Failed to read resume data.
    LoadError,
    /// Stored line number exceeds current max line number.
    ExceedMaxLine,
}

/// Saves the line number to a File.
///
/// # Parameters
/// - line: Line number.
/// - path: Path of the playlist file.
pub fn save_state(line: usize, path: &Path) -> Result<(), ResumeError> {
    let mut temp = path.to_path_buf().into_os_string();
    temp.push(".resume");

    let mut file = fs::File::create(temp).map_err(|_| ResumeError::StoreError)?;
    file.write_all(&line.to_be_bytes())
        .map_err(|_| ResumeError::StoreError)?;
    Ok(())
}

/// Loads the line number from a File.
///
/// # Parameters
/// - path: Path of the *playlist* file, the ".resume" suffic will be added automatically.
/// - max: Max length of the playlist.
///
/// # Errors
/// This function will check whether the stored line number exceeds the current total number of
/// lines, which may happen if the playlist file is modified.
/// Returns a [`ResumeError`] in this case.
pub fn load_state(path: &Path, max: usize) -> Result<usize, ResumeError> {
    let mut temp = path.to_path_buf().into_os_string();
    temp.push(".resume");

    let mut file = fs::File::open(&temp).map_err(|_| ResumeError::LoadError)?;
    let mut buffer = [0_u8; std::mem::size_of::<usize>()];
    file.read_exact(&mut buffer)
        .map_err(|_| ResumeError::LoadError)?;
    fs::remove_file(temp).map_err(|_| ResumeError::LoadError)?;
    let line = usize::from_be_bytes(buffer);

    if line <= max {
        Ok(line)
    } else {
        Err(ResumeError::ExceedMaxLine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_save() {
        save_state(5, &PathBuf::from("save.playlist")).unwrap();

        let mut file = fs::File::open("save.playlist.resume").unwrap();
        let mut buffer = [0_u8; std::mem::size_of::<usize>()];
        file.read_exact(&mut buffer).unwrap();
        fs::remove_file("save.playlist.resume").unwrap();

        assert_eq!(usize::from_be_bytes(buffer), 5_usize);
    }

    #[test]
    fn test_load() {
        let mut file = fs::File::create("load.playlist.resume").unwrap();
        file.write_all(&8_usize.to_be_bytes()).unwrap();

        let result = load_state(&PathBuf::from("load.playlist"), 10);
        assert_eq!(result, Ok(8_usize));
    }

    #[test]
    fn test_exceed() {
        let mut file = fs::File::create("exceed.playlist.resume").unwrap();
        file.write_all(&8_usize.to_be_bytes()).unwrap();

        let result = load_state(&PathBuf::from("exceed.playlist"), 4);
        assert_eq!(result, Err(ResumeError::ExceedMaxLine));
    }
}
