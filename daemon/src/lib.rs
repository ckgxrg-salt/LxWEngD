mod wallpaper;

pub mod commands;
pub mod playlist;
pub mod resume;
pub mod runner;

pub use runner::{Runner, RuntimeError};

/// Accepted requests to the main thread.
pub enum DaemonRequest {
    /// Reports that runner is gracefully exiting.
    /// However, this does not kill the runner, so if the runner is still running, this just cuts
    /// the connection to it.
    Exit(u8),
    /// Reports that runner encountered an error, and is halting.
    Abort(u8, runner::RuntimeError),
}
