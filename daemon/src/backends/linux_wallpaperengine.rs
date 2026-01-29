//! Utils for generating command and summoning `linux-wallpaperengine`.

use smol::process::{Command, Stdio};
use std::collections::HashMap;

use crate::backends::Backend;
use crate::daemon::{CACHE_PATH, CFG};

pub struct LxWEng {
    monitor: Option<String>,
    default_props: HashMap<String, String>,
}

impl Backend for LxWEng {
    fn get_name() -> String {
        "linux-wallpaperengine".to_string()
    }

    /// Gets the [`Command`] to start `linux-wallpaperengine`.
    fn get_sys_command(&self, name: &str, properties: &HashMap<String, String>) -> Command {
        let mut sys_cmd = Command::new(CFG.binary.as_deref().unwrap_or("linux-wallpaperengine"));
        if let Some(value) = &CFG.assets_path {
            sys_cmd.arg("--assets-dir").arg(value);
        }

        let properties = combine(&self.default_props, properties);
        map_properties(&properties, &mut sys_cmd);

        if let Some(value) = &self.monitor {
            sys_cmd.arg("--screen-root").arg(value).arg("--bg");
        }
        sys_cmd.arg(name);
        sys_cmd
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(CACHE_PATH.to_path_buf());
        sys_cmd
    }
}

impl LxWEng {
    pub fn new(monitor: Option<String>) -> Self {
        Self {
            monitor,
            default_props: HashMap::new(),
        }
    }

    /// Updates the held default properties.
    pub fn update_default_props(&mut self, defaults: HashMap<String, String>) {
        self.default_props = defaults;
    }
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

fn map_properties(properties: &HashMap<String, String>, cmd: &mut Command) {
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
            "clamp" => {
                cmd.arg("--clamp").arg(value);
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
