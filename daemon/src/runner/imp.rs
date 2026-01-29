//! A runner is created with a list of [`Command`]s that it will execute.
//!
//! Working cycle of a Runner:
//! 1. Check and fetch next [`Command`].
//! 2. Directly execute any oneshot [`Command`] and continue to next loop.
//! 3. [`Execution`] of long-running [`Command`]s.
//! 4. Handle the [`ExecResult`] reported by the [`Execution`] future.

use async_recursion::async_recursion;
use smol::lock::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

use crate::backends::{Backend, LxWEng};
use crate::runner::exec::{ExecResult, Execution};
use crate::runner::{
    Action, Command, NOMONITOR_INDICATOR, Runner, RunnerError, RunnerHandle, State,
};
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
    /// The special monitor name "NOMONITOR" is to indicate this runner has no associated monitor.
    ///
    /// # Errors
    /// If the given playlist cannot be parsed, or is empty, this will return [`RunnerError::InitFailed`].
    pub fn new(
        monitor: String,
        path: PathBuf,
    ) -> Result<(Self, Arc<Mutex<RunnerHandle>>), RunnerError> {
        let monitor = if monitor == NOMONITOR_INDICATOR {
            None
        } else {
            Some(monitor)
        };

        match playlist::open(&path) {
            Ok(file) => {
                let (tx, rx) = smol::channel::unbounded();
                let commands = playlist::parse(&path, &file).ok_or(RunnerError::InitFailed)?;
                let backend = LxWEng::new(monitor);

                let handle = Arc::new(Mutex::new(RunnerHandle {
                    index: 0,
                    commands,
                    state: State::Ready,
                    path,
                    backend_name: LxWEng::get_name(),
                    tx,
                }));

                Ok((
                    Self {
                        internal: handle.clone(),
                        backend,
                        rx,
                    },
                    handle,
                ))
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
            // Fetch current command,
            // By default go back to the beginning when reached the end
            let current_cmd = {
                let mut internal = self.internal.lock().await;
                if internal.index >= internal.commands.len() {
                    internal.index = 0;
                }
                let Some(current_cmd) = internal.commands.get(internal.index).cloned() else {
                    log::error!("Got invalid command");
                    internal.index += 1;
                    continue;
                };
                current_cmd
            };

            // Process current command
            match current_cmd {
                Command::Default(props) => self.backend.update_default_props(props),
                Command::End => break,
                cmd => match self.exec_async(cmd).await {
                    LoopFlag::Nothing => (),
                    LoopFlag::Break => break,
                    LoopFlag::Continue => continue,
                },
            }
            self.next();
        }
        self.update_state(State::Exited);
    }

    /// Handles long-running tasks
    #[async_recursion]
    async fn exec_async(&mut self, cmd: Command) -> LoopFlag {
        let mut exec = Execution::begin(cmd, &self.backend, self.rx.clone());
        self.update_state(State::Running(exec.info()));
        let result = exec.result().await;
        let flag = match result {
            ExecResult::Elapsed => LoopFlag::Nothing,
            ExecResult::Error => LoopFlag::Nothing,
            ExecResult::Interrupted(action) => match action {
                Action::Next => LoopFlag::Nothing,
                Action::Prev => {
                    self.prev();
                    LoopFlag::Continue
                }
                Action::Goto(i) => {
                    self.goto(i);
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
                    self.update_state(State::Paused(exec.remaining()));
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
