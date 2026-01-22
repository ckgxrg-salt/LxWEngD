//! General trait of a backend

use smol::process::Command;
use std::collections::HashMap;

pub trait Backend {
    fn get_sys_command(&self, name: &str, properties: &HashMap<String, String>) -> Command;
}
