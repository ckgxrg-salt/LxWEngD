//! Define commands the daemon can identify

use std::path::PathBuf;

#[derive(Debug, PartialEq)]
enum Command {
    // id, duration
    Wallpaper(u32, u32),
    // duration
    Wait(u32),
    // end
    End,
    // location, number
    Goto(u32, u32),
    // path
    Replace(PathBuf),
    Summon(PathBuf),
}

// Converts literal commands into tuples
fn identify(str: &str) -> Result<Command, String> {
    let mut segment = str.split_whitespace();
    match segment.next() {
        Some("wait") => {
            let duration = segment
                .next()
                .ok_or("Expected an argument for \"wait\" command".to_string())?
                .parse::<u32>()
                .map_err(|_| "Invalid argument".to_string())?;
            Ok(Command::Wait(duration))
        }

        Some("goto") => {
            let loc = segment
                .next()
                .ok_or("Expected an argument for \"goto\" command".to_string())?
                .parse::<u32>()
                .map_err(|_| "Invalid argument".to_string())?;
            let count = segment
                .next()
                .unwrap_or("0")
                .parse::<u32>()
                .map_err(|_| "Invalid argument".to_string())?;
            Ok(Command::Goto(loc, count))
        }
        Some("loop") => Ok(Command::Goto(1, 0)),
        Some("end") => Ok(Command::End),

        Some("replace") => {
            let path = segment
                .next()
                .ok_or("Expected an argument for \"replace\" command".to_string())?
                .parse::<PathBuf>()
                .map_err(|_| "Invalid argument".to_string())?;
            Ok(Command::Replace(path))
        }
        Some("summon") => {
            let path = segment
                .next()
                .ok_or("Expected an argument for \"summon\" command".to_string())?
                .parse::<PathBuf>()
                .map_err(|_| "Invalid argument".to_string())?;
            Ok(Command::Summon(path))
        }

        // Might be a wallpaper
        Some(value) => {
            let id = value
                .parse::<u32>()
                .map_err(|_| "Invalid command".to_string())?;
            let duration = segment
                .next()
                .ok_or("Expected a duration for wallpaper".to_string())?
                .parse::<u32>()
                .map_err(|_| "Invalid argument".to_string())?;
            Ok(Command::Wallpaper(id, duration))
        }

        _ => Err("Invalid command".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_commands() {
        let cmd = "wait 165";
        assert_eq!(identify(cmd), Ok(Command::Wait(165)));
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
        let cmd = "114514 360";
        assert_eq!(identify(cmd), Ok(Command::Wallpaper(114514, 360)));
    }

    #[test]
    fn identify_errors() {
        let cmd = "this is a very long string containing nothing but garbage";
        assert_eq!(identify(cmd), Err("Invalid command".to_string()));
        let cmd = "";
        assert_eq!(identify(cmd), Err("Invalid command".to_string()));
        let cmd = "wait    ";
        assert_eq!(
            identify(cmd),
            Err("Expected an argument for \"wait\" command".to_string())
        );
        let cmd = "goto some great place";
        assert_eq!(identify(cmd), Err("Invalid argument".to_string()));
    }
}
