//! Handles the socket and possible daemon commands

use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::character::complete::space0;
use nom::combinator::{map, map_res, opt};
use nom::{IResult, Parser};

use smol::io::AsyncReadExt;
use smol::net::unix::UnixListener;
use smol::stream::StreamExt;
use std::path::PathBuf;
use thiserror::Error;

use crate::runner::ResumeMode;

/// Possible daemon commands.
///
/// All `Option<String>` below means the arguments can be omitted, when this happens the program
/// apply them on the lastly operated runner.
#[derive(Debug, PartialEq)]
pub enum DaemonCmd {
    /// Load a playlist from the given path to a runner named as a given string.
    Load {
        path: PathBuf,
        monitor: String,
        paused: bool,
        resume_mode: ResumeMode,
    },
    /// Destroys the runner with the given name, the bool argument indicates whether a
    /// resume file should *NOT* be generated.
    Unload { no_save: bool, id: Option<String> },

    /// Pauses the given runner, the bool argument indicates whether `linux-wallpaperengine` should
    /// be SIGHUP-ed or terminated.
    Pause { hold: bool, id: Option<String> },
    /// Resumes the given runner.
    Play { id: Option<String> },
    /// Toggles play/pause of the given runner.
    Toggle { id: Option<String> },

    /// Return status information.
    Status,
    /// Quit `lxwengd`
    Quit,
}

#[derive(Debug, PartialEq, Error)]
pub enum SocketError {
    #[error("Failed to initialise socket")]
    InitFailed,
    #[error("Unrecognised commmand")]
    UnknownCmd,
    #[error("Invalid argument")]
    InvalidArgument,
    #[error("Socket internal error")]
    InternalError,
}

pub struct Socket {
    listener: UnixListener,
}

impl Socket {
    /// Initialises the socket
    pub fn new() -> Result<Self, SocketError> {
        let mut path = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".to_string());
        path.push_str("lxwengd.sock");

        Ok(Self {
            listener: UnixListener::bind(path).map_err(|_| SocketError::InitFailed)?,
        })
    }

    /// Listens for connections, and then handles [`DaemonCmd`]s.
    /// This reads the first possible [`DaemonCmd`] and then shuts down the connection.
    /// TODO: Possibly hold the connection until terminated.
    /// TODO: Finish this off
    pub async fn listen(&self) {
        loop {
            let mut incoming = self.listener.incoming();
            if let Some(Ok(mut conn)) = incoming.next().await {
                let mut content = String::new();
                let _ = conn.read_to_string(&mut content).await;
                let cmd = parse_cmd(&content);
            }
        }
    }
}

fn parse_quit(input: &str) -> IResult<&str, DaemonCmd> {
    map(tag("quit"), |_| DaemonCmd::Quit).parse(input)
}
fn parse_status(input: &str) -> IResult<&str, DaemonCmd> {
    map(tag("status"), |_| DaemonCmd::Status).parse(input)
}

fn parse_toggle(input: &str) -> IResult<&str, DaemonCmd> {
    let (input, _) = tag("toggle")(input)?;
    let (input, _) = space0(input)?;
    map(
        opt(take_till1(|c: char| c.is_whitespace())),
        |id: Option<&str>| DaemonCmd::Toggle {
            id: id.map(String::from),
        },
    )
    .parse(input)
}
fn parse_play(input: &str) -> IResult<&str, DaemonCmd> {
    let (input, _) = tag("play")(input)?;
    let (input, _) = space0(input)?;
    map(
        opt(take_till1(|c: char| c.is_whitespace())),
        |id: Option<&str>| DaemonCmd::Play {
            id: id.map(String::from),
        },
    )
    .parse(input)
}

fn parse_pause(input: &str) -> IResult<&str, DaemonCmd> {
    let (input, _) = tag("pause")(input)?;
    let (input, _) = space0(input)?;
    let (input, hold) =
        map_res(take_till1(|c: char| c.is_whitespace()), str::parse::<bool>).parse(input)?;
    let (input, _) = space0(input)?;
    map(
        opt(take_till1(|c: char| c.is_whitespace())),
        |id: Option<&str>| DaemonCmd::Pause {
            hold,
            id: id.map(String::from),
        },
    )
    .parse(input)
}
fn parse_unload(input: &str) -> IResult<&str, DaemonCmd> {
    let (input, _) = tag("unload")(input)?;
    let (input, _) = space0(input)?;
    let (input, no_save) =
        map_res(take_till1(|c: char| c.is_whitespace()), str::parse::<bool>).parse(input)?;
    let (input, _) = space0(input)?;
    map(
        opt(take_till1(|c: char| c.is_whitespace())),
        |id: Option<&str>| DaemonCmd::Unload {
            no_save,
            id: id.map(String::from),
        },
    )
    .parse(input)
}

fn parse_load(input: &str) -> IResult<&str, DaemonCmd> {
    let (input, _) = tag("load")(input)?;
    let (input, _) = space0(input)?;
    let (input, path) = map_res(
        take_till1(|c: char| c.is_whitespace()),
        str::parse::<PathBuf>,
    )
    .parse(input)?;
    let (input, _) = space0(input)?;
    let (input, id) = take_till1(|c: char| c.is_whitespace())(input)?;
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
        DaemonCmd::Load {
            path,
            monitor: id.to_string(),
            paused,
            resume_mode,
        },
    ))
}

/// Parse commands from clients
fn parse_cmd(input: &str) -> IResult<&str, DaemonCmd> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use smol::io::AsyncWriteExt;
    use std::time::Duration;

    #[test]
    fn test_parse_cmd() {
        let cmd = "load /tmp/test.playlist eDP-1 false ignoredel";
        assert_eq!(
            parse_cmd(cmd),
            Ok((
                "",
                DaemonCmd::Load {
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
                DaemonCmd::Play {
                    id: Some("DP-1".to_string())
                }
            ))
        );

        let cmd = "unload true";
        assert_eq!(
            parse_cmd(cmd),
            Ok((
                "",
                DaemonCmd::Unload {
                    no_save: true,
                    id: None
                }
            ))
        );

        let cmd = "status";
        assert_eq!(parse_cmd(cmd), Ok(("", DaemonCmd::Status)));
    }

    // #[test]
    // fn receiving_command() {
    //     let socket = Socket {
    //         listener: smol::net::unix::UnixListener::bind("/tmp/lxwengd-test.sock").unwrap(),
    //     };
    //
    //     let _quit = smol::spawn(async {
    //         let mut conn = smol::net::unix::UnixStream::connect("/tmp/lxwengd-test.sock")
    //             .await
    //             .unwrap();
    //         conn.write_all(b"quit").await.unwrap();
    //     });
    //
    //     smol::block_on(async {
    //         smol::future::race(
    //             async {
    //                 let result = socket.listen().await;
    //                 std::fs::remove_file("/tmp/lxwengd-test.sock").unwrap();
    //                 assert_eq!(result, Ok(("", DaemonCmd::Quit)));
    //             },
    //             async {
    //                 smol::Timer::after(Duration::from_secs(1)).await;
    //                 std::fs::remove_file("/tmp/lxwengd-test.sock").unwrap();
    //                 panic!("timeout waiting for `quit` command");
    //             },
    //         )
    //         .await;
    //     });
    // }
}
