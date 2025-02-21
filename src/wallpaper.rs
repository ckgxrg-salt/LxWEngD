//! # Wallpapers
//!
//! Provides utils for generating command and summoning linux-wallpaperengine.   

use crate::runner::RuntimeError;
use std::path::Path;
use std::time::Duration;
use subprocess::{Exec, NullFile};

pub fn get_cmd(id: u32, assets_path: Option<&Path>, monitor: Option<&str>) -> Exec {
    let mut engine = Exec::cmd("linux-wallpaperengine");
    if let Some(value) = assets_path {
        engine = engine.arg("--assets-dir").arg(value);
    }
    if let Some(value) = monitor {
        engine = engine.arg("--screen-root").arg(value).arg("--bg");
        // If invoked without --screen-root, linux-wallpaperengine rejects --bg
    }
    engine = engine.arg(id.to_string());
    engine.stdout(NullFile).stderr(NullFile)
}

pub fn summon(cmd: Exec, duration: Duration) -> Result<(), RuntimeError> {
    let mut proc = cmd.popen().unwrap();
    let result = proc.wait_timeout(duration);
    match result {
        Ok(None) => {
            proc.terminate().unwrap();
            // Give it some time to finalise
            proc.wait_timeout(Duration::from_secs(5)).unwrap();
            proc.kill().unwrap();
            Ok(())
        }
        Ok(Some(_)) => Err(RuntimeError::EngineDied),
        Err(_) => Err(RuntimeError::EngineDied),
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
