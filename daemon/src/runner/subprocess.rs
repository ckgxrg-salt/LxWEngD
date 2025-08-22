//! Spawns and monitors `linux-wallpaperengine` subprocess.

use crate::runner::{Action, Runner};
use crate::utils::subprocess;

use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// How the subprocess task exited.
enum ExitType {
    Success,
    Exited,
    WithAction(Action),
}

impl Runner {
    /// Displays a wallpaper for some time.
    ///
    /// # Returns
    /// See [`ExitType`].
    async fn summon_wallpaper(
        &self,
        id: u32,
        duration: Duration,
        properties: &HashMap<String, String>,
    ) -> ExitType {
        // TODO: check whether monitor is valid
        let mut cmd = subprocess::get_cmd(id, Some(&self.name), properties, &self.default);
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();

        let start = Instant::now();

        let exit = smol::future::race(
            smol::future::race(
                async {
                    let _ = child.status().await;
                    ExitType::Exited
                },
                async {
                    smol::Timer::after(duration).await;
                    ExitType::Success
                },
            ),
            async {
                let action = self.channel.1.recv().await.unwrap_or(Action::Error);
                ExitType::WithAction(action)
            },
        )
        .await;

        match &exit {
            ExitType::Exited => {
                log::warn!(
                    "{} cmd no.{}: `linux-wallpaperengine` unexpectedly exited",
                    self.path.to_string_lossy(),
                    self.index + 1,
                );
            }
            ExitType::WithAction(Action::Pause(keep)) => {
                let end = Instant::now();
                let past_time = end.duration_since(start);

                if *keep
                    && kill(
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
            ExitType::Success | ExitType::WithAction(_) => {
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
        }
        exit
    }

    /// TODO: Resumes a wallpaper after it has been paused somewhere.
    ///
    /// # Returns
    /// See [`ExitType`].
    async fn resume_wallpaper(
        &self,
        id: u32,
        remaining_duration: Duration,
        properties: &HashMap<String, String>,
    ) -> ExitType {
        todo!()
    }

    /// Displays a wallpaper for infinite duration.
    ///
    /// # Returns
    /// See [`ExitType`].
    async fn summon_wallpaper_forever(
        &self,
        id: u32,
        properties: &HashMap<String, String>,
    ) -> ExitType {
        // TODO: check whether monitor is valid
        let mut cmd = subprocess::get_cmd(id, Some(&self.name), properties, &self.default);
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();

        let exit = smol::future::race(
            async {
                let _ = child.status().await;
                ExitType::Exited
            },
            async {
                let action = self.channel.1.recv().await.unwrap_or(Action::Error);
                ExitType::WithAction(action)
            },
        )
        .await;

        match &exit {
            ExitType::Exited => {
                log::warn!(
                    "{} cmd no.{}: `linux-wallpaperengine` unexpectedly exited",
                    self.path.to_string_lossy(),
                    self.index + 1,
                );
            }
            ExitType::WithAction(_) => {
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
            ExitType::Success => unreachable!(),
        }
        exit
    }
}
