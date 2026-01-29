//! Async tasks

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use smol::channel::Receiver;
use smol::process::Child;
use std::time::{Duration, Instant};

use crate::backends::Backend;
use crate::runner::{Action, CmdDuration, Command, RunnerError};

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
    pub(super) duration: Option<Duration>,
    pub(super) start: Instant,
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
                // TODO: Error handling
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

    /// Sleeps indefinitely and wait for an [`Action`].
    async fn wait_action(rx: &Receiver<Action>) -> ExecResult {
        match rx.recv().await {
            Ok(action) => ExecResult::Interrupted(action),
            Err(_) => ExecResult::Error,
        }
    }

    pub async fn result(&mut self) -> ExecResult {
        match &mut self.kind {
            ExecType::Supervise { child } => {
                if let Some(duration) = self.info.duration {
                    smol::future::race(
                        smol::future::race(
                            async {
                                smol::Timer::after(duration).await;
                                ExecResult::Elapsed
                            },
                            async {
                                let _ = child.status().await;
                                ExecResult::Error
                            },
                        ),
                        Self::wait_action(&self.interrupt_rx),
                    )
                    .await
                } else {
                    smol::future::race(
                        async {
                            let _ = child.status().await;
                            ExecResult::Error
                        },
                        Self::wait_action(&self.interrupt_rx),
                    )
                    .await
                }
            }
            ExecType::Sleep => {
                if let Some(duration) = self.info.duration {
                    smol::future::race(
                        async {
                            smol::Timer::after(duration).await;
                            ExecResult::Elapsed
                        },
                        Self::wait_action(&self.interrupt_rx),
                    )
                    .await
                } else {
                    Self::wait_action(&self.interrupt_rx).await
                }
            }
        }
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
                if child
                    .try_status()
                    .map_err(|_| RunnerError::CleanupFail)?
                    .is_none()
                {
                    let pid =
                        Pid::from_raw(child.id().try_into().expect("PID should not be that large"));

                    kill(Pid::from_raw(pid.into()), Signal::SIGTERM)
                        .map_err(|_| RunnerError::CleanupFail)
                } else {
                    Ok(())
                }
            }
            ExecType::Sleep => Ok(()),
        }
    }
}
