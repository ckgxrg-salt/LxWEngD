//! Defines commands the daemon can identify.
//!
//! Also provides a function to parse strings to commands.
#![warn(clippy::pedantic)]

use std::error::Error;
use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum Command {
    /// Displays the wallpaper with given id for given duration.
    Wallpaper(u32, Duration),
    /// Sleeps for given duration.
    Wait(Duration),
    /// Ends the playlist.
    End,
    /// Jump to a line in the playlist.
    Goto(usize, u32),
    /// Make the current runner execute another playlist.
    Replace(PathBuf),
    /// Requests the main thread to summon a new runner executing another playlist.
    Summon(PathBuf),
    /// Changes the monitor the current runner operating on.
    Monitor(String),
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// Indicates that this line is not a recognised command.
    /// Blank lines are also treated as invalid commands.
    CommandNotFound,
    /// The command requires some arguments, but not enough are provided.
    /// Note that if you use `#` to comment in the line, anything after that `#` will be ignored.
    NotEnoughArguments,
    /// The command requires an argument of a specific type, but the given one cannot be parsed
    /// into that type.
    InvalidArgument,
}
impl Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::CommandNotFound => {
                write!(f, "Unrecognised command")
            }
            ParseError::NotEnoughArguments => {
                write!(f, "Not enough arguments are given to the command")
            }
            ParseError::InvalidArgument => {
                write!(f, "Arguments given to the command are invalid")
            }
        }
    }
}

/// Parse strings.
///
/// Returns the parsed command with `Ok()` if successful.
///
/// # Errors
/// If fails to identify the given string, a [`ParseError`] is returned.
pub fn identify(str: &str) -> Result<Command, ParseError> {
    let mut segment = str.split_whitespace();
    match segment.next() {
        Some("wait") => {
            let duration_str = segment.next().ok_or(ParseError::NotEnoughArguments)?;
            let duration =
                duration_str::parse(duration_str).map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Wait(duration))
        }

        Some("goto") => {
            let loc = segment
                .next()
                .ok_or(ParseError::NotEnoughArguments)?
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidArgument)?;
            let count = segment
                .next()
                .unwrap_or("0")
                .parse::<u32>()
                .map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Goto(loc, count))
        }
        Some("loop") => Ok(Command::Goto(1, 0)),
        Some("end") => Ok(Command::End),

        Some("replace") => {
            let path = segment
                .next()
                .ok_or(ParseError::NotEnoughArguments)?
                .parse::<PathBuf>()
                .map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Replace(path))
        }
        Some("summon") => {
            let path = segment
                .next()
                .ok_or(ParseError::NotEnoughArguments)?
                .parse::<PathBuf>()
                .map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Summon(path))
        }

        Some("monitor") => {
            let name = segment
                .next()
                .ok_or(ParseError::NotEnoughArguments)?
                .parse::<String>()
                .map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Monitor(name))
        }

        // Might be a wallpaper
        Some(value) => {
            let id = value
                .parse::<u32>()
                .map_err(|_| ParseError::CommandNotFound)?;
            let duration_str = segment.next().ok_or(ParseError::NotEnoughArguments)?;
            let duration =
                duration_str::parse(duration_str).map_err(|_| ParseError::InvalidArgument)?;
            Ok(Command::Wallpaper(id, duration))
        }

        _ => Err(ParseError::CommandNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_commands() {
        let cmd = "wait 165";
        assert_eq!(identify(cmd), Ok(Command::Wait(Duration::new(165, 0))));
        let cmd = "goto 165";
        assert_eq!(identify(cmd), Ok(Command::Goto(165, 0)));
        let cmd = "loop";
        assert_eq!(identify(cmd), Ok(Command::Goto(1, 0)));
        let cmd = "end";
        assert_eq!(identify(cmd), Ok(Command::End));
        let cmd = "replace some";
        assert_eq!(identify(cmd), Ok(Command::Replace(PathBuf::from("some"))));
        let cmd = "summon other";
        assert_eq!(identify(cmd), Ok(Command::Summon(PathBuf::from("other"))));
        let cmd = "114514 5h";
        assert_eq!(
            identify(cmd),
            Ok(Command::Wallpaper(114514, Duration::new(5 * 60 * 60, 0)))
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
        let cmd = "goto some great place";
        assert_eq!(identify(cmd), Err(ParseError::InvalidArgument));
    }
}
