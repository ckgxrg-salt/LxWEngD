//! Tests general functionality of runners

use std::path::PathBuf;
use std::sync::mpsc;

use lxwengd::{DaemonRequest, Runner};

mod common;

#[test]
fn default_playlist() {
    common::setup();
    let (tx, rx) = mpsc::channel();
    let mut runner = Runner::new(
        0,
        PathBuf::from("default.playlist"),
        &common::SearchPath,
        &common::CachePath,
        tx,
        true,
    );
    runner.run();

    let result = rx.recv().expect("Failed to receive message");
    if let DaemonRequest::Exit(0) = result {
        common::finalise();
        assert_eq!(
            "default.playlist line 1: Display wallpaper ID: 1 for 15min
Run: linux-wallpaperengine 1
default.playlist line 2: Display wallpaper ID: 2 for 1h
Run: linux-wallpaperengine 2
default.playlist line 3: Display wallpaper ID: 3 for 6min
Run: linux-wallpaperengine 3
default.playlist line 4: Sleep for 5min
default.playlist line 5: Reached the end
"
            .to_string(),
            common::Captured
                .clone()
                .read()
                .expect("Cannot read captured log")
                .to_string()
        );
    } else {
        common::finalise();
        panic!("Runner didn't exit gracefully");
    }
}
