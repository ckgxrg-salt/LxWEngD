//! Defines commands the daemon can identify.
//!
//! Also provides a function to parse strings to commands.

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::character::complete::{alphanumeric1, char, space0};
use nom::combinator::{map, map_res, opt, rest};
use nom::multi::separated_list0;
use nom::sequence::{pair, separated_pair};
use nom::{Finish, IResult, Parser};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub enum Command {
    /// Displays the wallpaper with given id for given duration.
    /// Third argument indicates whether this wallpaper will be displayed forever.
    /// Last arguments are a list of key-value pairs for recognised properties.
    Wallpaper(String, WallpaperDuration, HashMap<String, String>),
    /// Sleeps for given duration.
    Sleep(WallpaperDuration),
    /// Ends the playlist.
    End,
    /// Sets default properties for all wallpapers.
    Default(HashMap<String, String>),
}

#[derive(Debug, PartialEq)]
pub enum WallpaperDuration {
    Finite(Duration),
    Infinite,
}

impl TryFrom<&str> for WallpaperDuration {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "infinite" => Ok(WallpaperDuration::Infinite),
            s => {
                let duration = duration_str::parse(s).map_err(|_| ParseError::InvalidArgument)?;
                Ok(WallpaperDuration::Finite(duration))
            }
        }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    /// Indicates that this line is not a recognised command.
    /// Blank lines are also treated as invalid commands.
    #[error("Unrecognised command")]
    CommandNotFound,
    /// The command requires some arguments, but not enough are provided.
    /// Note that if you use `#` to comment in the line, anything after that `#` will be ignored.
    #[error("Not enough arguments")]
    NotEnoughArguments,
    /// The command requires an argument of a specific type, but the given one cannot be parsed
    /// into that type.
    #[error("Invalid arguments")]
    InvalidArgument,
}

fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = pair(char('#'), rest).parse(input)?;
    Ok((input, ()))
}

fn parse_properties(input: &str) -> IResult<&str, HashMap<String, String>> {
    // `parse_comment` will eat the input if it succeeds
    let (input, _) = opt(parse_comment).parse(input)?;
    let list_parser = separated_list0(
        space0,
        separated_pair(alphanumeric1, char('='), alphanumeric1),
    );
    let mut prop_parser = map(list_parser, |list: Vec<(&str, &str)>| {
        list.into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>()
    });
    prop_parser.parse(input)
}

fn parse_duration(input: &str) -> IResult<&str, WallpaperDuration> {
    // `parse_comment` will eat the input if it succeeds
    let (input, _) = opt(parse_comment).parse(input)?;
    map_res(
        take_till1(|c: char| c.is_whitespace()),
        WallpaperDuration::try_from,
    )
    .parse(input)
}

fn parse_end(input: &str) -> IResult<&str, Command> {
    map(tag("end"), |_| Command::End).parse(input)
}

fn parse_sleep(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("sleep")(input)?;
    let (input, _) = space0(input)?;
    map(parse_duration, Command::Sleep).parse(input)
}

fn parse_default(input: &str) -> IResult<&str, Command> {
    let (input, _) = tag("default")(input)?;
    let (input, _) = space0(input)?;
    let (input, props) = parse_properties(input)?;
    Ok((input, Command::Default(props)))
}

fn parse_wallpaper(input: &str) -> IResult<&str, Command> {
    let (input, id) = take_till1(|c: char| c.is_whitespace() || c == '#')(input)?;
    let (input, _) = space0(input)?;

    let (input, duration) = parse_duration(input)?;
    let (input, _) = space0(input)?;

    let (input, props) = parse_properties(input)?;

    Ok((input, Command::Wallpaper(id.to_string(), duration, props)))
}

fn parse_command(input: &str) -> IResult<&str, Command> {
    alt((parse_end, parse_sleep, parse_default, parse_wallpaper)).parse(input)
}

/// Parse a string.
///
/// Returns the parsed [`Command`] with if successful.
///
/// This function expects that a valid [`Command`] can be parsed from the input.
/// Supplying a comment-only line to it will also result in a failure.
/// To use it, filter any comment-only line beforehand.
///
/// This function also don't trim the input string.
/// The parsing might fail because of leading space.
///
/// # Errors
/// If fails to parse the given string, a [`ParseError`] is returned.
pub fn parse(input: &str) -> Result<Command, ParseError> {
    match parse_command(input).finish() {
        Ok((_, cmd)) => Ok(cmd),
        Err(nom::error::Error {
            input: _,
            code: nom::error::ErrorKind::TakeTill1,
        }) => Err(ParseError::NotEnoughArguments),
        Err(nom::error::Error {
            input: _,
            code: nom::error::ErrorKind::MapRes,
        }) => Err(ParseError::InvalidArgument),
        Err(_) => Err(ParseError::CommandNotFound),
    }
}

impl TryFrom<&str> for Command {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn individual_parsers() {
        assert_eq!(parse_comment("# whatever"), Ok(("", ())));
        let mut expected = HashMap::new();
        expected.insert(String::from("k1"), String::from("v1"));
        expected.insert(String::from("k2"), String::from("v2"));
        assert_eq!(parse_properties("k1=v1 k2=v2"), Ok(("", expected.clone())));
        assert_eq!(
            parse_duration("1s"),
            Ok(("", WallpaperDuration::Finite(Duration::from_secs(1))))
        );
        assert_eq!(
            parse_duration("infinite"),
            Ok(("", WallpaperDuration::Infinite))
        );

        assert_eq!(parse_end("end"), Ok(("", Command::End)));
        // assert_eq!(parse_end("nope"), Ok(("", Command::End)));
        assert_eq!(
            parse_sleep("sleep 1"),
            Ok((
                "",
                Command::Sleep(WallpaperDuration::Finite(Duration::new(1, 0)))
            ))
        );
        assert_eq!(
            parse_default("default k1=v1 k2=v2"),
            Ok(("", Command::Default(expected)))
        );
    }

    #[test]
    fn comment_position() {
        assert_eq!(
            parse_command("end # whatever"),
            Ok((" # whatever", Command::End))
        );
        assert_eq!(
            parse_command("sleep infinite # whatever"),
            Ok((" # whatever", Command::Sleep(WallpaperDuration::Infinite)))
        );
    }

    #[test]
    fn identify_commands() {
        let cmd = "sleep 165";
        assert_eq!(
            parse(cmd),
            Ok(Command::Sleep(WallpaperDuration::Finite(Duration::new(
                165, 0
            ))))
        );
        let cmd = "end";
        assert_eq!(parse(cmd), Ok(Command::End));
        let cmd = "114514 5h";
        assert_eq!(
            parse(cmd),
            Ok(Command::Wallpaper(
                "114514".to_string(),
                WallpaperDuration::Finite(Duration::new(5 * 60 * 60, 0)),
                HashMap::new()
            ))
        );
    }

    #[test]
    fn identify_errors() {
        let cmd = "this is a very long string containing nothing but garbage";
        assert_eq!(parse(cmd), Err(ParseError::InvalidArgument));
        let cmd = "";
        assert_eq!(parse(cmd), Err(ParseError::NotEnoughArguments));
        let cmd = "wait    ";
        assert_eq!(parse(cmd), Err(ParseError::NotEnoughArguments));
    }

    #[test]
    fn identify_properties() {
        let cmd = "114514 15m dps=15 cup=superbigcup";
        let mut expected = HashMap::new();
        expected.insert(String::from("dps"), String::from("15"));
        expected.insert(String::from("cup"), String::from("superbigcup"));
        assert_eq!(
            parse(cmd),
            Ok(Command::Wallpaper(
                "114514".to_string(),
                WallpaperDuration::Finite(Duration::from_secs(15 * 60)),
                expected
            ))
        );
        let cmd = "114514 infinite ooh=hoo";
        let mut expected = HashMap::new();
        expected.insert(String::from("ooh"), String::from("hoo"));
        assert_eq!(
            parse(cmd),
            Ok(Command::Wallpaper(
                "114514".to_string(),
                WallpaperDuration::Infinite,
                expected
            ))
        );
    }
}
