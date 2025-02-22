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
    properties: &[(String, String)],
) -> Exec {
    let mut engine = Exec::cmd("linux-wallpaperengine");
    engine = handle_properties(properties, engine);
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

fn handle_properties(properties: &[(String, String)], mut engine: Exec) -> Exec {
    for (key, value) in properties {
        match key.as_str() {
            "audio" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    engine = engine.arg("--no-audio-processing");
                }
            }
            "automute" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    engine = engine.arg("--no-automute");
                }
            }
            "fullscreen-pause" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    engine = engine.arg("--no-fullscreen-pause");
                }
            }
            "mouse" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    engine = engine.arg("--disable-mouse");
                }
            }
            "fps" => engine = engine.arg("--fps").arg(value),
            "volume" => engine = engine.arg("--volume").arg(value),
            "window" => engine = engine.arg("--window").arg(value),
            "scaling" => engine = engine.arg("--scaling").arg(value),
            "clamping" => engine = engine.arg("--clamping").arg(value),
            _ => engine = engine.arg("--set-property").arg(format!("{key}={value}")),
        }
    }
    engine
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

pub fn summon_forever(cmd: Exec) -> RuntimeError {
    let Ok(mut proc) = cmd.popen() else {
        return RuntimeError::EngineDied;
    };
    // This should block forever unless the child is killed externally
    let _ = proc.wait();
    RuntimeError::EngineDied
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn getting_cmd() {
        let cmd = get_cmd(
            114514,
            &PathBuf::from("/tmp/lxwengd-dev"),
            Some(&PathBuf::from("ng")),
            Some("Headless-1"),
            &vec![],
        );
        assert_eq!(
            cmd.to_cmdline_lossy(),
            String::from(
                "linux-wallpaperengine --assets-dir ng --screen-root Headless-1 --bg 114514"
            )
        );
    }

    #[test]
    fn applying_properties() {
        let mut engine = Exec::cmd("vmlinuz");
        let properties = vec![
            (String::from("fps"), String::from("15")),
            (String::from("scaling"), String::from("destruction")),
            (String::from("clamping"), String::from("boom")),
        ];
        engine = handle_properties(&properties, engine);
        assert_eq!(
            engine.to_cmdline_lossy(),
            "vmlinuz --fps 15 --scaling destruction --clamping boom"
        );

        let mut engine = Exec::cmd("./gradlew");
        let properties = vec![
            (String::from("mouse"), String::from("true")),
            (String::from("automute"), String::from("or")),
            (String::from("audio"), String::from("false")),
        ];
        engine = handle_properties(&properties, engine);
        assert_eq!(engine.to_cmdline_lossy(), "./gradlew --no-audio-processing");

        let mut engine = Exec::cmd("systemd");
        let properties = vec![
            (String::from("mujica"), String::from("ooo")),
            (String::from("whoknows"), String::from("idk")),
        ];
        engine = handle_properties(&properties, engine);
        assert_eq!(
            engine.to_cmdline_lossy(),
            "systemd --set-property 'mujica=ooo' --set-property 'whoknows=idk'"
        );
    }
}
