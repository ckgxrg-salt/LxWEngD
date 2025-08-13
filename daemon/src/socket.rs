//! Handles the socket and possible daemon commands

use smol::{
    io::AsyncReadExt,
    net::unix::{UnixListener, UnixStream},
    stream::StreamExt,
};
use std::{path::PathBuf, str::FromStr};
use thiserror::Error;

/// Possible daemon commands.
///
/// All `Option<String>` below means the arguments can be omitted, when this happens the program
/// apply them on the lastly operated runner.
#[derive(Debug, PartialEq)]
pub enum DaemonCmd {
    /// Load a playlist from the given path to a runner named as a given string.
    Load {
        path: PathBuf,
        id: String,
        paused: bool,
        resume: ResumeMode,
    },
    /// Stops and destroys the runner with the given name, the bool argument indicates whether a
    /// resume file should *NOT* be generated.
    Unload(bool, Option<String>),
    /// Pauses the given runner, the bool argument indicates whether `linux-wallpaperengine` should
    /// be SIGHUP-ed or terminated.
    Pause(bool, Option<String>),
    /// Resumes the given runner.
    Play(Option<String>),
    /// Toggles play/pause of the given runner.
    Toggle(Option<String>),
    /// Return status information.
    Status,
    /// Quit `LxWEngd`
    Quit,
}

/// How should a runner treat resume files on startup.
#[derive(Debug, PartialEq)]
pub enum ResumeMode {
    /// Ignore the resume file for the playlist.
    Ignore,
    /// Ignore and delete the resume file for the playlist.
    IgnoreDel,
    /// Apply but delete the resume file for the playlist.
    ApplyDel,
    /// Apply the resume file for the playlist. This is the default behaviour.
    Apply,
}
impl FromStr for ResumeMode {
    type Err = SocketError;
    fn from_str(value: &str) -> Result<Self, SocketError> {
        match value {
            "ignore" => Ok(Self::Ignore),
            "ignoredel" => Ok(Self::IgnoreDel),
            "applydel" => Ok(Self::ApplyDel),
            "apply" => Ok(Self::Apply),
            _ => Err(SocketError::UnknownCmd),
        }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum SocketError {
    #[error("failed to initialise socket")]
    InitFailed,
    #[error("unrecognised commmand")]
    UnknownCmd,
}

struct Socket {
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

    /// Listens for connections
    pub async fn listen(&self) {
        let mut incoming = self.listener.incoming();
        while let Some(Ok(mut conn)) = incoming.next().await {
            let mut content = String::new();
            let _ = conn.read_to_string(&mut content).await;
            match parse_cmd(&content) {
                Ok(DaemonCmd::Play(target)) => todo!(),
                Ok(DaemonCmd::Pause(keep, target)) => todo!(),
                Ok(DaemonCmd::Toggle(target)) => todo!(),
                Ok(DaemonCmd::Status) => todo!(),
                Ok(DaemonCmd::Load {
                    path,
                    id,
                    paused,
                    resume,
                }) => todo!(),
                Ok(DaemonCmd::Unload(no_resume, target)) => todo!(),
                Ok(DaemonCmd::Quit) => break,
                Err(err) => {
                    log::error!("{err}");
                }
            }
        }
    }
}

/// Parse commands from clients
fn parse_cmd(cmd: &str) -> Result<DaemonCmd, SocketError> {
    let mut segments = cmd.split_whitespace();
    match segments.next() {
        Some("quit") => Ok(DaemonCmd::Quit),
        Some("status") => Ok(DaemonCmd::Status),
        Some("toggle") => {
            if let Some(target) = segments.next() {
                return Ok(DaemonCmd::Toggle(Some(target.to_string())));
            }
            Ok(DaemonCmd::Toggle(None))
        }
        Some("play") => {
            if let Some(target) = segments.next() {
                return Ok(DaemonCmd::Play(Some(target.to_string())));
            }
            Ok(DaemonCmd::Play(None))
        }
        Some("pause") => {
            let keep = segments
                .next()
                .ok_or(SocketError::UnknownCmd)?
                .parse::<bool>()
                .map_err(|_| SocketError::UnknownCmd)?;
            if let Some(target) = segments.next() {
                return Ok(DaemonCmd::Pause(keep, Some(target.to_string())));
            }
            Ok(DaemonCmd::Pause(keep, None))
        }
        Some("unload") => {
            let no_resume = segments
                .next()
                .ok_or(SocketError::UnknownCmd)?
                .parse::<bool>()
                .map_err(|_| SocketError::UnknownCmd)?;
            if let Some(target) = segments.next() {
                return Ok(DaemonCmd::Unload(no_resume, Some(target.to_string())));
            }
            Ok(DaemonCmd::Unload(no_resume, None))
        }
        Some("load") => {
            let path = segments
                .next()
                .ok_or(SocketError::UnknownCmd)?
                .parse::<PathBuf>()
                .map_err(|_| SocketError::UnknownCmd)?;
            let id = segments.next().ok_or(SocketError::UnknownCmd)?.to_string();
            let paused = segments
                .next()
                .ok_or(SocketError::UnknownCmd)?
                .parse::<bool>()
                .map_err(|_| SocketError::UnknownCmd)?;
            let resume = segments
                .next()
                .ok_or(SocketError::UnknownCmd)?
                .parse::<ResumeMode>()?;
            Ok(DaemonCmd::Load {
                path,
                id,
                paused,
                resume,
            })
        }

        _ => Err(SocketError::UnknownCmd),
    }
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
            Ok(DaemonCmd::Load {
                path: PathBuf::from("/tmp/test.playlist"),
                id: "eDP-1".to_string(),
                paused: false,
                resume: ResumeMode::IgnoreDel
            })
        );

        let cmd = "play DP-1";
        assert_eq!(
            parse_cmd(cmd),
            Ok(DaemonCmd::Play(Some("DP-1".to_string())))
        );

        let cmd = "unload true";
        assert_eq!(parse_cmd(cmd), Ok(DaemonCmd::Unload(true, None)));

        let cmd = "status";
        assert_eq!(parse_cmd(cmd), Ok(DaemonCmd::Status));
    }

    #[test]
    fn sending_quit() {
        let socket = Socket {
            listener: smol::net::unix::UnixListener::bind("/tmp/lxwengd-test.sock").unwrap(),
        };

        let _quit = smol::spawn(async {
            let mut conn = smol::net::unix::UnixStream::connect("/tmp/lxwengd-test.sock")
                .await
                .unwrap();
            conn.write_all(b"quit").await.unwrap();
        });

        smol::block_on(async {
            smol::future::race(
                async {
                    socket.listen().await;
                    std::fs::remove_file("/tmp/lxwengd-test.sock").unwrap();
                },
                async {
                    smol::Timer::after(Duration::from_secs(1)).await;
                    std::fs::remove_file("/tmp/lxwengd-test.sock").unwrap();
                    panic!("timeout waiting for `quit` command");
                },
            )
            .await;
        });
    }
}
