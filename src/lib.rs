mod commands;
mod wallpaper;

use std::path::PathBuf;

pub mod playlist;
pub mod runner;

/// Accepted requests to the main thread   
/// - NewRunner(playlist file): Summons a new runner thread operating the given playlist.   
/// - Exit(id): Reports that runner is gracefully exiting.   
/// - Abort(id, error): Reports that runner encountered an error, and is halting.   
pub enum DaemonRequest {
    NewRunner(PathBuf),
    Exit(u8),
    Abort(u8, runner::RuntimeError),
}
