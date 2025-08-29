//! Utils for generating command and summoning linux-wallpaperengine.

use smol::process::{Command, Stdio};
use std::collections::HashMap;

use crate::entry::{CACHE_PATH, CFG};

/// Gets the [`Command`] to start `linux-wallpaperengine`.
pub fn get_cmd(
    id: u32,
    monitor: Option<&str>,
    properties: &HashMap<String, String>,
    defaults: &HashMap<String, String>,
) -> Command {
    let mut cmd = Command::new(CFG.binary.as_deref().unwrap_or("linux-wallpaperengine"));
    if let Some(value) = &CFG.assets_path {
        cmd.arg("--assets-dir").arg(value);
    }

    let properties = combine(defaults, properties);
    handle_properties(&properties, &mut cmd);

    if let Some(value) = monitor {
        cmd.arg("--screen-root").arg(value).arg("--bg");
    }
    cmd.arg(id.to_string());
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(CACHE_PATH.to_path_buf());
    cmd
}

/// Combine 2 [`HashMap`]s.
fn combine<'a>(
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

fn handle_properties(properties: &HashMap<String, String>, cmd: &mut Command) {
    for (key, value) in properties {
        match key.as_str() {
            "silent" => {
                if value.parse::<bool>().is_ok_and(|value| value) {
                    cmd.arg("--silent");
                }
            }
            "audio" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    cmd.arg("--no-audio-processing");
                }
            }
            "automute" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    cmd.arg("--no-automute");
                }
            }
            "fullscreen-pause" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    cmd.arg("--no-fullscreen-pause");
                }
            }
            "mouse" => {
                if value.parse::<bool>().is_ok_and(|value| !value) {
                    cmd.arg("--disable-mouse");
                }
            }
            "fps" => {
                cmd.arg("--fps").arg(value);
            }
            "volume" => {
                cmd.arg("--volume").arg(value);
            }
            "window" => {
                cmd.arg("--window").arg(value);
            }
            "scaling" => {
                cmd.arg("--scaling").arg(value);
            }
            "clamping" => {
                cmd.arg("--clamping").arg(value);
            }
            _ => {
                cmd.arg("--set-property").arg(format!("{key}={value}"));
            }
        }
    }
}

/// Output properties in form or key-value pairs for dry-run
pub fn pretty_print(properties: &HashMap<String, String>) -> String {
    let mut result = String::new();
    for (key, value) in properties {
        result.push_str(&format!("{key}={value} "));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_properties() {
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

        assert_eq!(combine(&base, &overrides), expected);
    }
}
