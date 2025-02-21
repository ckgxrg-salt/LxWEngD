//! # Wallpapers
//!
//! Provides utils for generating command and summoning linux-wallpaperengine.   

use crate::runner::RuntimeError;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use subprocess::{Exec, NullFile};

pub fn cache_dir() -> PathBuf {
    // linux-wallpaperengine generates some cache
    if let Ok(mut value) = env::var("XDG_CACHE_HOME") {
        value.push_str("/lxwengd");
        return PathBuf::from(value);
    };
    if let Ok(mut value) = env::var("HOME") {
        value.push_str("/.cache/lxwengd");
        return PathBuf::from(value);
    };
    // This is not persistent anyhow
    PathBuf::from("/tmp/lxwengd")
}

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
    engine
        .stdout(NullFile)
        .stderr(NullFile)
        .cwd(crate::CachePath.as_path())
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
        Ok(Some(_)) => Err(RuntimeError::EngineDied),
        Err(_) => Err(RuntimeError::EngineDied),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_cmd() {
        let cmd = get_cmd(114514, Some(&PathBuf::from("ng")), Some("Headless-1"));
        assert_eq!(
            cmd.to_cmdline_lossy(),
            String::from(
                "linux-wallpaperengine --assets-dir ng --screen-root Headless-1 --bg 114514"
            )
        );
    }
}
