//! Tests general functionality of runners

use std::path::PathBuf;
use std::sync::mpsc;

use lxwengd::{DaemonRequest, Runner};

mod common;

#[test]
fn test() {
    common::setup();
    let (tx, rx) = mpsc::channel();

    // A playlist with `goto`s
    let mut runner = Runner::new(0, &common::SearchPath, &common::CachePath, tx.clone());
    runner.init(PathBuf::from("goto.playlist"));
    runner.dry_run();

    let result = rx.recv().expect("Failed to receive message");
    if let DaemonRequest::Exit(0) = result {
        common::finalise();
        assert_eq!(
            "goto.playlist line 1: Display wallpaper ID: 1 for 1s\nRun: linux-wallpaperengine 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nRemaining times for this goto: 2\ngoto.playlist line 1: Display wallpaper ID: 1 for 1s\nRun: linux-wallpaperengine 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nRemaining times for this goto: 1\ngoto.playlist line 1: Display wallpaper ID: 1 for 1s\nRun: linux-wallpaperengine 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nThis goto is no longer effective\ngoto.playlist line 4: Display wallpaper ID: 3 for 1s\nRun: linux-wallpaperengine 3\ngoto.playlist line 5: Goto line 2\nRemaining times for this goto: 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nRemaining times for this goto: 2\ngoto.playlist line 1: Display wallpaper ID: 1 for 1s\nRun: linux-wallpaperengine 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nRemaining times for this goto: 1\ngoto.playlist line 1: Display wallpaper ID: 1 for 1s\nRun: linux-wallpaperengine 1\ngoto.playlist line 2: Display wallpaper ID: 2 for 1s\nRun: linux-wallpaperengine 2\ngoto.playlist line 3: Goto line 1\nThis goto is no longer effective\ngoto.playlist line 4: Display wallpaper ID: 3 for 1s\nRun: linux-wallpaperengine 3\ngoto.playlist line 5: Goto line 2\nThis goto is no longer effective\ngoto.playlist line 6: Display wallpaper ID: 4 for 1s\nRun: linux-wallpaperengine 4\ngoto.playlist line 7: Display wallpaper ID: 5 for 1s\nRun: linux-wallpaperengine 5\ngoto.playlist line 8: Display wallpaper ID: 6 for 1s\nRun: linux-wallpaperengine 6\ngoto.playlist line 9: Goto line 7\nRemaining times for this goto: 2\ngoto.playlist line 7: Display wallpaper ID: 5 for 1s\nRun: linux-wallpaperengine 5\ngoto.playlist line 8: Display wallpaper ID: 6 for 1s\nRun: linux-wallpaperengine 6\ngoto.playlist line 9: Goto line 7\nRemaining times for this goto: 1\ngoto.playlist line 7: Display wallpaper ID: 5 for 1s\nRun: linux-wallpaperengine 5\ngoto.playlist line 8: Display wallpaper ID: 6 for 1s\nRun: linux-wallpaperengine 6\ngoto.playlist line 9: Goto line 7\nThis goto is no longer effective\ngoto.playlist line 10: Reached the end\n"
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
