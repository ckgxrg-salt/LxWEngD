//! An action can be requested for the [`Runner`] to perform.
//!
//! These actions will interrupt the the current task.
//! When the runner is in paused state,
//! any of these actions will put it back to normal mode.

use crate::runner::Command;

pub enum Action {
    /// Jump to next [`Command`].
    Next,
    /// Jump to previous [`Command`].
    Prev,
    /// Jump to a certain [`Command`].
    Goto(usize),

    /// Execute a [`Command`] specified by the user manually.
    Exec(Command),

    /// Pause current [`Command`]. bool indicates whether to terminate the child.
    Pause(bool),

    /// Terminates the [`Runner`] because of user request.
    Exit,
}
