//! Provides utils for summoning linux-wallpaperengine

use crate::runner::RuntimeError;
use std::ops::Deref;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub fn get_cmd(id: u32, assets_path: Option<&Path>, monitor: Option<&str>) -> Command {
    let mut engine = Command::new("linux-wallpaperengine");
    if let Some(value) = assets_path {
        engine.arg("--assets-dir").arg(value);
    }
    if let Some(value) = monitor {
        engine.arg("--screen-root").arg(value);
        // If invoked without --screen-root, linux-wallpaperengine rejects --bg
        engine.arg("--bg");
    }
    engine.arg(id.to_string());
    engine
}

pub fn summon(mut cmd: Command, duration: Duration) -> Result<(), RuntimeError> {
    // exitcode, condvar
    let pair = Arc::new((Mutex::new(None), Condvar::new()));
    let pair2 = Arc::clone(&pair);
    let monitor = thread::spawn(move || {
        let (lock, cond) = &*pair2;
        let mut status = lock.lock().unwrap();
        *status = Some(cmd.status().unwrap());
        cond.notify_one();
    });

    let (lock, cond) = &*pair;
    let status = lock.lock().unwrap();
    let result = cond.wait_timeout(status, duration).unwrap();
    (*monitor.thread()).id();
    if result.1.timed_out() {
        Ok(())
    } else {
        Err(RuntimeError::EngineDied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn test_summon_failure() {
        let mut cmd = Command::new("linux-wallpaperengine");
        cmd.stderr(Stdio::null());
        let result = summon(cmd, Duration::from_secs(15));
        assert_eq!(result, Err(RuntimeError::EngineDied));
    }
}
