//! Actual execution of commands

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::backends::Backend;
use crate::runner::{Action, Runner, State};

/// How a [`Command`] execution ended up.
pub(super) enum ExecResult {
    /// The timer elapsed, this is the normal behaviour
    Elapsed,
    /// One-shot commands done
    Done,
    /// User requested an [`Action`] to be done
    Interrupted(Action),
    /// The backend program died unexpectedly, this should be an error
    Error,
}

impl<T: Backend> Runner<T> {
    /// Sleeps indefinitely and wait for an [`Action`].
    pub(super) async fn wait_action(&self) -> ExecResult {
        if let Ok(action) = self.rx.recv().await {
            ExecResult::Interrupted(action)
        } else {
            ExecResult::Error
        }
    }

    /// Sleeps for a given duration or an [`Action`].
    pub(super) async fn sleep(&self, duration: Duration) -> ExecResult {
        smol::future::race(
            async {
                smol::Timer::after(duration).await;
                ExecResult::Elapsed
            },
            self.wait_action(),
        )
        .await
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

        let start = Instant::now();

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

        match &result {
            ExecResult::Interrupted(Action::Pause(clear)) => {
                let end = Instant::now();
                let past_time = end.duration_since(start);

                self.state = State::Paused(duration - past_time);

                if *clear
                    && kill(
                        Pid::from_raw(pid.try_into().expect("pid won't go that large")),
                        Signal::SIGTERM,
                    )
                    .is_err()
                {
                    log::warn!(
                        "{}:{} error: failed to terminate `linux-wallpaperengine`",
                        self.path.to_string_lossy(),
                        self.index + 1,
                    );
                }
            }
            _ => {
                if kill(
                    Pid::from_raw(pid.try_into().expect("pid won't go that large")),
                    Signal::SIGTERM,
                )
                .is_err()
                {
                    log::warn!(
                        "{} cmd no.{}: failed to terminate `linux-wallpaperengine`",
                        self.path.to_string_lossy(),
                        self.index + 1,
                    );
                }
            }
        };
        result
    }
}
