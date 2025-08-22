//! Defines commands the daemon can identify.
//!
//! Also provides a function to parse strings to commands.

use std::{collections::HashMap, str::SplitWhitespace, time::Duration};
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub enum Command {
    /// Displays the wallpaper with given id for given duration.
    /// Third argument indicates whether this wallpaper will be displayed forever.
    /// Last arguments are a list of key-value pairs for recognised properties.
    Wallpaper(u32, Duration, bool, HashMap<String, String>),
    /// Sleeps for given duration.
    /// TODO: Remove this
    Wait(Duration),
    /// Ends the playlist.
    End,
    /// Sets default properties for all wallpapers.
    Default(HashMap<String, String>),
}

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    /// Indicates that this line is not a recognised command.
    /// Blank lines are also treated as invalid commands.
    #[error("unrecognised command")]
    CommandNotFound,
    /// The command requires some arguments, but not enough are provided.
    /// Note that if you use `#` to comment in the line, anything after that `#` will be ignored.
    #[error("not enough arguments to this command")]
    NotEnoughArguments,
    /// The command requires an argument of a specific type, but the given one cannot be parsed
    /// into that type.
    #[error("invalid arguments give to this command")]
    InvalidArgument,
}

/// Parse strings.
///
/// Returns the parsed command with `Ok()` if successful.
///
/// # Errors
/// If fails to identify the given string, a [`ParseError`] is returned.
#[allow(clippy::missing_panics_doc)]
pub fn identify(str: &str) -> Result<Command, ParseError> {
    let mut segment = str.split_whitespace();
    match segment.next() {
        Some("end") => Ok(Command::End),
        Some("wait") => {
            let duration_str = segment.next().ok_or(ParseError::NotEnoughArguments)?;
            let duration =
                duration_str::parse(duration_str).map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Wait(duration))
        }
        Some("default") => {
            let properties = extract_properties(&mut segment)?;
            Ok(Command::Default(properties))
        }
        // Might be a wallpaper
        Some(value) => {
            let id = value
                .parse::<u32>()
                .map_err(|_| ParseError::CommandNotFound)?;
            let duration_str = segment.next();
            if duration_str.is_some_and(|value| !value.starts_with('#')) {
                let value = duration_str.unwrap();
                let properties = extract_properties(&mut segment)?;
                if value == "forever" {
                    return Ok(Command::Wallpaper(id, Duration::ZERO, true, properties));
                }
                let duration =
                    duration_str::parse(value).map_err(|_| ParseError::InvalidArgument)?;
                return Ok(Command::Wallpaper(id, duration, false, properties));
            }
            Ok(Command::Wallpaper(id, Duration::ZERO, true, HashMap::new()))
        }

        _ => Err(ParseError::CommandNotFound),
    }
}

fn extract_properties(
    segment: &mut SplitWhitespace,
) -> Result<HashMap<String, String>, ParseError> {
    let mut result = HashMap::new();
    for value in segment.by_ref() {
        if value.starts_with('#') {
            break;
        }
        let (key, value) = value.split_once('=').ok_or(ParseError::InvalidArgument)?;
        result.insert(key.to_owned(), value.to_owned());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_commands() {
        let cmd = "wait 165";
        assert_eq!(identify(cmd), Ok(Command::Wait(Duration::new(165, 0))));
        let cmd = "end";
        assert_eq!(identify(cmd), Ok(Command::End));
        let cmd = "114514 5h";
        assert_eq!(
            identify(cmd),
            Ok(Command::Wallpaper(
                114_514,
                Duration::new(5 * 60 * 60, 0),
                false,
                HashMap::new()
            ))
        );
    }

    #[test]
    fn identify_errors() {
        let cmd = "this is a very long string containing nothing but garbage";
        assert_eq!(identify(cmd), Err(ParseError::CommandNotFound));
        let cmd = "";
        assert_eq!(identify(cmd), Err(ParseError::CommandNotFound));
        let cmd = "wait    ";
        assert_eq!(identify(cmd), Err(ParseError::NotEnoughArguments));
    }

    #[test]
    fn identify_properties() {
        let cmd = "114514 15m dps=15 cup=superbigcup";
        let mut expected = HashMap::new();
        expected.insert(String::from("dps"), String::from("15"));
        expected.insert(String::from("cup"), String::from("superbigcup"));
        assert_eq!(
            identify(cmd),
            Ok(Command::Wallpaper(
                114_514,
                Duration::from_secs(15 * 60),
                false,
                expected
            ))
        );
        let cmd = "114514 # Very beautiful wallpaper";
        assert_eq!(
            identify(cmd),
            Ok(Command::Wallpaper(
                114_514,
                Duration::ZERO,
                true,
                HashMap::new()
            ))
        );
        let cmd = "114514 forever ooh=hoo";
        let mut expected = HashMap::new();
        expected.insert(String::from("ooh"), String::from("hoo"));
        assert_eq!(
            identify(cmd),
            Ok(Command::Wallpaper(114_514, Duration::ZERO, true, expected))
        );
        let cmd = "114514 5min some=ok hello kids I'm here to destroy the Earth";
        assert_eq!(identify(cmd), Err(ParseError::InvalidArgument));
    }
}
