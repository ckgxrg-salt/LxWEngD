//! Do some preparations for integration tests

use env_logger;
use lazy_static::lazy_static;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

lazy_static! {
    pub static ref Captured: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
    pub static ref SearchPath: PathBuf = PathBuf::from("playlists");
    pub static ref CachePath: PathBuf = PathBuf::from("test_cache");
}

struct Capturer {
    content: Arc<RwLock<String>>,
}
impl std::io::Write for Capturer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut locked = self.content.write().unwrap();
        let got = std::str::from_utf8(buf).unwrap();
        locked.push_str(got);
        Ok(got.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn setup() {
    let cap = Capturer {
        content: Captured.clone(),
    };
    env_logger::builder()
        .is_test(true)
        .format(|buf, record| write!(buf, "{}\n", record.args()))
        .filter_level(log::LevelFilter::Trace)
        .target(env_logger::Target::Pipe(Box::new(cap)))
        .init();
    let _ = std::fs::create_dir(&*CachePath);
}

pub fn finalise() {
    std::fs::remove_dir(&*CachePath).expect("Cannot remove test cache directory");
}
