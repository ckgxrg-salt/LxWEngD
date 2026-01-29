//! Parses IPC commands

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::character::complete::space0;
use nom::combinator::{map, map_res};
use nom::{Finish, IResult, Parser};
use std::path::PathBuf;
use std::str::FromStr;

use crate::runner::ResumeMode;
use crate::utils::ParseError;

/// Possible daemon commands.
///
/// All `Option<String>` below means the arguments can be omitted, when this happens the program
/// apply them on the lastly operated runner.
#[derive(Debug, PartialEq)]
pub enum IPCCmd {
    /// Load a playlist from the given path to a runner named as a given string.
    Load {
        path: PathBuf,
        monitor: String,
        paused: bool,
        resume_mode: ResumeMode,
    },
    /// Destroys the runner with the given name, the bool argument indicates whether a
    /// resume file should *NOT* be generated.
    Unload { no_save: bool, monitor: String },

    /// Pauses the given runner, the bool argument indicates whether `linux-wallpaperengine` should
    /// be terminated or kept.
    Pause { clear: bool, monitor: String },
    /// Resumes the given runner.
    Play { monitor: String },
    /// Toggles play/pause of the given runner.
    Toggle { monitor: String },

    /// Return status information.
    Status,
    /// Quit `lxwengd`
    Quit,
}

fn parse_quit(input: &str) -> IResult<&str, IPCCmd> {
    map(tag("quit"), |_| IPCCmd::Quit).parse(input)
}
fn parse_status(input: &str) -> IResult<&str, IPCCmd> {
    map(tag("status"), |_| IPCCmd::Status).parse(input)
}

fn parse_toggle(input: &str) -> IResult<&str, IPCCmd> {
    let (input, _) = tag("toggle")(input)?;
    let (input, _) = space0(input)?;
    map(take_till1(|c: char| c.is_whitespace()), |monitor: &str| {
        IPCCmd::Toggle {
            monitor: monitor.to_string(),
        }
    })
    .parse(input)
}
fn parse_play(input: &str) -> IResult<&str, IPCCmd> {
    let (input, _) = tag("play")(input)?;
    let (input, _) = space0(input)?;
    map(take_till1(|c: char| c.is_whitespace()), |monitor: &str| {
        IPCCmd::Play {
            monitor: monitor.to_string(),
        }
    })
    .parse(input)
}

fn parse_pause(input: &str) -> IResult<&str, IPCCmd> {
    let (input, _) = tag("pause")(input)?;
    let (input, _) = space0(input)?;
    let (input, clear) =
        map_res(take_till1(|c: char| c.is_whitespace()), str::parse::<bool>).parse(input)?;
    let (input, _) = space0(input)?;
    map(take_till1(|c: char| c.is_whitespace()), |monitor: &str| {
        IPCCmd::Pause {
            clear,
            monitor: monitor.to_string(),
        }
    })
    .parse(input)
}
fn parse_unload(input: &str) -> IResult<&str, IPCCmd> {
    let (input, _) = tag("unload")(input)?;
    let (input, _) = space0(input)?;
    let (input, no_save) =
        map_res(take_till1(|c: char| c.is_whitespace()), str::parse::<bool>).parse(input)?;
    let (input, _) = space0(input)?;
    map(take_till1(|c: char| c.is_whitespace()), |monitor: &str| {
        IPCCmd::Unload {
            no_save,
            monitor: monitor.to_string(),
        }
    })
    .parse(input)
}

fn parse_load(input: &str) -> IResult<&str, IPCCmd> {
    let (input, _) = tag("load")(input)?;
    let (input, _) = space0(input)?;
    let (input, path) = map_res(
        take_till1(|c: char| c.is_whitespace()),
        str::parse::<PathBuf>,
    )
    .parse(input)?;
    let (input, _) = space0(input)?;
    let (input, monitor) = take_till1(|c: char| c.is_whitespace())(input)?;
    let (input, _) = space0(input)?;
    let (input, paused) =
        map_res(take_till1(|c: char| c.is_whitespace()), str::parse::<bool>).parse(input)?;
    let (input, _) = space0(input)?;
    let (input, resume_mode) = map_res(
        take_till1(|c: char| c.is_whitespace()),
        str::parse::<ResumeMode>,
    )
    .parse(input)?;
    Ok((
        input,
        IPCCmd::Load {
            path,
            monitor: monitor.to_string(),
            paused,
            resume_mode,
        },
    ))
}

/// Parse commands from clients
fn parse_cmd(input: &str) -> IResult<&str, IPCCmd> {
    alt((
        parse_play,
        parse_pause,
        parse_toggle,
        parse_status,
        parse_load,
        parse_unload,
        parse_quit,
    ))
    .parse(input)
}

/// Parse a string.
///
/// Returns the parsed [`IPCCmd`] with if successful.
///
/// # Errors
/// If fails to parse the given string, a [`ParseError`] is returned.
pub fn parse(input: &str) -> Result<IPCCmd, ParseError> {
    match parse_cmd(input).finish() {
        Ok((_, cmd)) => Ok(cmd),
        Err(nom::error::Error {
            input: _,
            code: nom::error::ErrorKind::Tag,
        }) => Err(ParseError::InvalidArgument),
        Err(nom::error::Error {
            input: _,
            code: nom::error::ErrorKind::MapRes,
        }) => Err(ParseError::InvalidArgument),
        Err(_) => Err(ParseError::CommandNotFound),
    }
}

impl FromStr for IPCCmd {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_cmd() {
        let cmd = "load /tmp/test.playlist eDP-1 false ignoredel";
        assert_eq!(
            parse_cmd(cmd),
            Ok((
                "",
                IPCCmd::Load {
                    path: PathBuf::from("/tmp/test.playlist"),
                    monitor: "eDP-1".to_string(),
                    paused: false,
                    resume_mode: ResumeMode::IgnoreDel
                }
            ))
        );

        let cmd = "play DP-1";
        assert_eq!(
            parse_cmd(cmd),
            Ok((
                "",
                IPCCmd::Play {
                    monitor: "DP-1".to_string()
                }
            ))
        );

        let cmd = "unload true NOMONITOR";
        assert_eq!(
            parse_cmd(cmd),
            Ok((
                "",
                IPCCmd::Unload {
                    no_save: true,
                    monitor: "NOMONITOR".to_string()
                }
            ))
        );

        let cmd = "status";
        assert_eq!(parse_cmd(cmd), Ok(("", IPCCmd::Status)));
    }

    #[test]
    fn parsing_error() {
        assert_eq!(
            parse_cmd("play").finish(),
            Err(nom::error::Error {
                input: "play",
                code: nom::error::ErrorKind::Tag,
            })
        );
    }
}
