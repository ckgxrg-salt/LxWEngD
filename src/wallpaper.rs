//! # Wallpapers
//!
//! Provides utils for generating command and summoning linux-wallpaperengine.
#![warn(clippy::pedantic)]

use crate::runner::RuntimeError;
use std::path::Path;
use std::time::Duration;
use subprocess::{Exec, NullFile};

pub fn get_cmd(
    id: u32,
    cache_path: &Path,
    assets_path: Option<&Path>,
    monitor: Option<&str>,
) -> Exec {
    let mut engine = Exec::cmd("linux-wallpaperengine");
    if let Some(value) = assets_path {
        engine = engine.arg("--assets-dir").arg(value);
    }
    if let Some(value) = monitor {
        engine = engine.arg("--screen-root").arg(value).arg("--bg");
        // If invoked without --screen-root, linux-wallpaperengine rejects --bg
    }
    engine = engine.arg(id.to_string());
    engine.stdout(NullFile).stderr(NullFile).cwd(cache_path)
}

pub fn summon(cmd: Exec, duration: Duration) -> Result<(), RuntimeError> {
    let Ok(mut proc) = cmd.popen() else {
        return Err(RuntimeError::EngineDied);
    };
    let result = proc.wait_timeout(duration);
    match result {
        // Duration has elapsed
        Ok(None) => {
            proc.terminate().unwrap();
            // Give it some time to finalise
            proc.wait_timeout(Duration::from_secs(5)).unwrap();
            proc.kill().unwrap();
            Ok(())
        }
        // Terminated abruptly
        Ok(Some(_)) | Err(_) => Err(RuntimeError::EngineDied),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cmd() {
        let cmd = get_cmd(
            114514,
            &Path::from("/tmp/lxwengd-dev"),
            Some(&Path::from("ng")),
            Some("Headless-1"),
        );
        assert_eq!(
            cmd.to_cmdline_lossy(),
            String::from(
                "linux-wallpaperengine --assets-dir ng --screen-root Headless-1 --bg 114514"
            )
        );
    }
}
