mod linux_wallpaperengine;

pub use linux_wallpaperengine::LxWEng;

use smol::process::Command;
use std::collections::HashMap;

/// General trait of a backend.
pub trait Backend {
    fn get_sys_command(&self, name: &str, properties: &HashMap<String, String>) -> Command;
}
