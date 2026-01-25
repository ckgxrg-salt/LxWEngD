//! A runner is created with a list of [`Command`]s that it will execute.
//!
//! A runner registers itself with the daemon.
//!
//! Working cycle of a Runner:
//! 1. Check and fetch next [`Command`].
//! 2. Directly execute any oneshot [`Command`] and continue to next loop.
//! 3. [`Execution`] of long-running [`Command`]s.
//! 4. Handle the [`ExecResult`] reported by the [`Execution`] future.
//!
//! When it exits, it clears its own entry in the registered runners.

use std::path::PathBuf;

use crate::backends::LxWEng;
use crate::runner::exec::{ExecResult, Execution};
use crate::runner::{Action, Command, Runner, RunnerError, State};
use crate::utils::playlist;

/// A flag to break the outer loop.
enum LoopFlag {
    Break,
    Continue,
    Nothing,
}

/// Currently only `linux-wallpaperengine` is supported.
impl Runner<LxWEng> {
    /// Creates a new Runner that operates the given playlist.
    ///
    /// # Errors
    /// If the given playlist cannot be parsed, or is empty, this will return [`RunnerError::InitFailed`].
    pub fn new(monitor: Option<String>, path: PathBuf) -> Result<Self, RunnerError> {
        match playlist::open(&path) {
            Ok(file) => {
                let (tx, rx) = smol::channel::unbounded();
                let commands = playlist::parse(&path, &file).ok_or(RunnerError::InitFailed)?;
                Ok(Self {
                    commands,
                    index: 0,
                    state: State::Ready,
                    path,
                    backend: LxWEng::new(monitor),
                    tx,
                    rx,
                })
            }
            Err(err) => {
                log::error!("{err}");
                Err(RunnerError::InitFailed)
            }
        }
    }

    /// The main runner task.
    pub async fn run(&mut self) {
        loop {
            // By default go back to the beginning when reached the end
            if self.index >= self.commands.len() {
                self.index = 0;
            }
            // TODO: Possibly eliminate the Clone by Mutex?
            let Some(current_cmd) = self.commands.get(self.index).cloned() else {
                log::error!("Got invalid command");
                self.index += 1;
                continue;
            };
            match current_cmd {
                Command::Default(props) => self.backend.update_default_props(&props),
                Command::End => break,
                cmd => match self.exec_async(cmd).await {
                    LoopFlag::Nothing => (),
                    LoopFlag::Break => break,
                    LoopFlag::Continue => continue,
                },
            }
            self.index += 1;
        }
        self.state = State::Exited;
    }

    /// Handles long-running tasks
    async fn exec_async(&mut self, cmd: Command) -> LoopFlag {
        let mut exec = Execution::begin(cmd, &self.backend, self.rx.clone());
        self.state = State::Running(exec);
        let result = exec.result().await;
        let flag = match result {
            ExecResult::Elapsed => LoopFlag::Nothing,
            ExecResult::Error => LoopFlag::Nothing,
            ExecResult::Interrupted(action) => match action {
                Action::Next => LoopFlag::Nothing,
                Action::Prev => {
                    self.index -= 1;
                    LoopFlag::Continue
                }
                Action::Goto(i) => {
                    self.index = i;
                    LoopFlag::Continue
                }
                Action::Exec(cmd) => {
                    exec.cleanup();
                    return self.exec_async(cmd).await;
                }
                Action::Pause(clear) => {
                    if clear {
                        exec.cleanup();
                    }
                    self.state = State::Paused(exec.remaining());
                    self.rx.recv().await;
                    return LoopFlag::Nothing;
                }
                Action::Exit => LoopFlag::Break,
            },
        };
        exec.cleanup();
        flag
    }
}
