//! # Wallpapers
//!
//! Provides utils for generating command and summoning linux-wallpaperengine.
#![warn(clippy::pedantic)]

use crate::runner::RuntimeError;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use subprocess::{Exec, NullFile};

pub fn get_cmd(
    id: u32,
    cache_path: &Path,
    assets_path: Option<&Path>,
    monitor: Option<&str>,
    properties: &HashMap<String, String>,
    defaults: &HashMap<String, String>,
) -> Exec {
    let mut engine = Exec::cmd("linux-wallpaperengine");
    if let Some(value) = assets_path {
        engine = engine.arg("--assets-dir").arg(value);
    }
    engine = handle_properties(&intersect(defaults, properties), engine);
    if let Some(value) = monitor {
        engine = engine.arg("--screen-root").arg(value).arg("--bg");
        // If invoked without --screen-root, linux-wallpaperengine rejects --bg
    }
    engine = engine.arg(id.to_string());
    engine.stdout(NullFile).stderr(NullFile).cwd(cache_path)
}

fn intersect<'a>(
    base: &'a HashMap<String, String>,
    overrides: &'a HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = base.to_owned();
    for (key, value) in overrides {
        let entry = result.entry(key.to_string()).or_default();
        *entry = value.to_string();
    }
    result
}

fn handle_properties(properties: &HashMap<String, String>, mut engine: Exec) -> Exec {
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
            &HashMap::new(),
            &HashMap::new(),
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
        let mut properties = HashMap::new();
        properties.insert(String::from("fps"), String::from("15"));
        properties.insert(String::from("scaling"), String::from("destruction"));
        properties.insert(String::from("clamping"), String::from("boom"));
        engine = handle_properties(&properties, engine);
        assert!(engine.to_cmdline_lossy().contains("--fps 15"));
        assert!(engine.to_cmdline_lossy().contains("--scaling destruction"));
        assert!(engine.to_cmdline_lossy().contains("--clamping boom"));

        let mut engine = Exec::cmd("./gradlew");
        let mut properties = HashMap::new();
        properties.insert(String::from("mouse"), String::from("true"));
        properties.insert(String::from("automute"), String::from("or"));
        properties.insert(String::from("audio"), String::from("false"));
        engine = handle_properties(&properties, engine);
        assert!(engine.to_cmdline_lossy().contains("--no-audio-processing"));
        assert!(!engine.to_cmdline_lossy().contains("automute"));
        assert!(!engine.to_cmdline_lossy().contains("mouse"));

        let mut engine = Exec::cmd("systemd");
        let mut properties = HashMap::new();
        properties.insert(String::from("mujica"), String::from("ooo"));
        properties.insert(String::from("whoknows"), String::from("idk"));
        engine = handle_properties(&properties, engine);
        assert!(engine
            .to_cmdline_lossy()
            .contains("--set-property 'mujica=ooo'"));
        assert!(engine
            .to_cmdline_lossy()
            .contains("--set-property 'whoknows=idk'"));
    }

    #[test]
    fn intersect_properties() {
        let mut base = HashMap::new();
        base.insert(String::from("unknown"), String::from("unknown"));
        base.insert(String::from("known"), String::from("still unknown"));
        base.insert(String::from("xixi"), String::from("noxixi"));

        let mut overrides = HashMap::new();
        overrides.insert(String::from("unknown"), String::from("got it!"));
        overrides.insert(String::from("known"), String::from("umm"));
        overrides.insert(String::from("woo"), String::from("hoo"));

        let mut expected = HashMap::new();
        expected.insert(String::from("unknown"), String::from("got it!"));
        expected.insert(String::from("known"), String::from("umm"));
        expected.insert(String::from("xixi"), String::from("noxixi"));
        expected.insert(String::from("woo"), String::from("hoo"));

        assert_eq!(intersect(&base, &overrides), expected);
    }
}
