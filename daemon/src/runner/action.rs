//! An action can be requested for the [`Runner`] to perform.
//! These actions will interrupt the the current task.

use crate::utils::command::Command;

pub enum Action {
    /// Jump to next [`Command`].
    Next,
    /// Jump to previous [`Command`].
    Prev,
    /// Jump to a certain [`Command`].
    Goto(usize),

    /// Execute a [`Command`] specified by the user manually.
    Exec(Command),

    /// Pause current [`Command`]. bool indicates whether to SIGHUP the child instead
    /// of terminating.
    Pause(bool),
    /// Resumes normal operation.
    Resume,

    /// Terminates the [`Runner`] because of user request.
    Exit,
}
