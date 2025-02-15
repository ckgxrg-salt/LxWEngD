//! Define commands the daemon can identify

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, PartialEq)]
enum Command {
    // id, duration
    Wallpaper(u32, Duration),
    // duration
    Wait(Duration),
    // end
    End,
    // location, number
    Goto(u32, u32),
    // path
    Replace(PathBuf),
    Summon(PathBuf),
}

#[derive(Debug, PartialEq)]
enum ParseError {
    CommandNotFound,
    NotEnoughArguments,
    InvalidArgument,
}

// Converts literal commands into tuples
fn identify(str: &str) -> Result<Command, ParseError> {
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
                .parse::<u32>()
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
