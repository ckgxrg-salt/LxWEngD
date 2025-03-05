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
            "   ".to_string(),
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
