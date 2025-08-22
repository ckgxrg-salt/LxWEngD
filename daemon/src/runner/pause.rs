//! Handles listening when the runner is in [`State::Paused`].

use crate::runner::{Action, Runner, State};

impl Runner {
    /// Loop handling actions until a [`RunnerAction::Resume`] is received.
    async fn paused(&self) {
        loop {
            todo!()
        }
    }
}
