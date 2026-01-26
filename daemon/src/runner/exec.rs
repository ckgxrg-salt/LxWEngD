//! Async tasks

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use smol::channel::Receiver;
use smol::process::Child;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::backends::Backend;
use crate::runner::{Action, CmdDuration, Command, Runner, RunnerError};

pub struct Execution {
    kind: ExecType,
    info: ExecInfo,
    interrupt_rx: Receiver<Action>,
}

/// Data held during an execution
///
/// This data structure is only for long-running async tasks.
/// Oneshot commands should be handled by [`Runner`] directly.
enum ExecType {
    Supervise { child: Child },
    Sleep,
}

/// This struct contains status information for the current execution.
#[derive(Clone)]
pub struct ExecInfo {
    duration: Option<Duration>,
    start: Instant,
}

pub enum ExecResult {
    /// The timer elapsed, this is the normal behaviour
    Elapsed,
    /// User requested an [`Action`] to be done
    Interrupted(Action),
    /// The backend program died unexpectedly, this should be an error
    Error,
}

impl Execution {
    /// Begins execution of a [`Command`].
    ///
    /// This immediately begins the execution, to get the result, `.await` on `.result()`.
    pub fn begin<T: Backend>(cmd: Command, backend: &T, interrupt_rx: Receiver<Action>) -> Self {
        match cmd {
            Command::Wallpaper(name, duration, properties) => {
                let mut sys_cmd = backend.get_sys_command(&name, &properties);
                let child = sys_cmd.spawn().unwrap();
                match duration {
                    CmdDuration::Finite(duration) => Self {
                        kind: ExecType::Supervise { child },
                        info: ExecInfo {
                            duration: Some(duration),
                            start: Instant::now(),
                        },
                        interrupt_rx,
                    },
                    CmdDuration::Infinite => Self {
                        kind: ExecType::Supervise { child },
                        info: ExecInfo {
                            duration: None,
                            start: Instant::now(),
                        },
                        interrupt_rx,
                    },
                }
            }
            Command::Sleep(duration) => match duration {
                CmdDuration::Finite(duration) => Self {
                    kind: ExecType::Sleep,
                    info: ExecInfo {
                        duration: Some(duration),
                        start: Instant::now(),
                    },
                    interrupt_rx,
                },
                CmdDuration::Infinite => Self {
                    kind: ExecType::Sleep,
                    info: ExecInfo {
                        duration: None,
                        start: Instant::now(),
                    },
                    interrupt_rx,
                },
            },
            _ => unreachable!(),
        }
    }

    pub fn info(&self) -> ExecInfo {
        self.info.clone()
    }

    pub async fn result(&self) -> ExecResult {
        todo!()
    }

    pub fn remaining(&self) -> Option<Duration> {
        let end = Instant::now();
        let actual = end.duration_since(self.info.start);
        self.info.duration.map(|expected| expected - actual)
    }

    /// Kills the child process.
    ///
    /// This consumes the execution.
    pub fn cleanup(&mut self) -> Result<(), RunnerError> {
        match &mut self.kind {
            ExecType::Supervise { child } => {
                // If the child is still alive. We send a SIGTERM instead of a SIGKILL.
                if let None = child.try_status().map_err(|_| RunnerError::CleanupFail)? {
                    let pid =
                        Pid::from_raw(child.id().try_into().expect("PID should not be that large"));

                    kill(
                        Pid::from_raw(pid.try_into().expect("pid won't go that large")),
                        Signal::SIGTERM,
                    )
                    .map_err(|_| RunnerError::CleanupFail)
                } else {
                    Ok(())
                }
            }
            ExecType::Sleep => Ok(()),
        }
    }
}

impl<T: Backend> Runner<T> {
    /// Sleeps indefinitely and wait for an [`Action`].
    pub(super) async fn wait_action(&mut self) -> ExecResult {
        if let Ok(action) = self.rx.recv().await {
            ExecResult::Interrupted(action)
        } else {
            ExecResult::Error
        }
    }

    /// Sleeps for a given duration or an [`Action`].
    pub(super) async fn sleep(&mut self, duration: Duration) -> ExecResult {
        let start = Instant::now();
        smol::future::race(
            async {
                smol::Timer::after(duration).await;
                ExecResult::Elapsed
            },
            async {
                match self.rx.recv().await {
                    Ok(action) => ExecResult::Interrupted(action),
                    Err(_) => ExecResult::Error,
                }
            },
        )
        .await
    }

    /// Summons a wallpaper indefinitely.
    pub(super) async fn wallpaper_infinite(
        &mut self,
        name: &str,
        properties: &HashMap<String, String>,
    ) -> ExecResult {
        todo!()
    }

    /// Summons a wallpaper for a given duration or an [`Action`].
    pub(super) async fn wallpaper(
        &mut self,
        name: &str,
        duration: Duration,
        properties: &HashMap<String, String>,
    ) -> ExecResult {
        let mut sys_cmd = self.backend.get_sys_command(name, properties);
        let mut child = sys_cmd.spawn().unwrap();
        let pid = child.id();
        let result = smol::future::race(
            smol::future::race(
                async {
                    smol::Timer::after(duration).await;
                    ExecResult::Elapsed
                },
                async {
                    child.status().await;
                    ExecResult::Error
                },
            ),
            self.wait_action(),
        )
        .await;

        result
    }
}
