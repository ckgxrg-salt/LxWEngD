//! `LxWEngd` entry
//!
//! The daemon that operates `linux-wallpaperengine`.
//! Unless `--standby` is passed in the arguments, the programs attempts to find the default
//! playlist and runs it on all possible monitors.

// logging
fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                message
            ));
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

// Cli main entry
fn main() {
    // If cache directory does not exist, create it
    if !CACHE_PATH.is_dir() {
        if let Err(err) = std::fs::create_dir(CACHE_PATH.as_path()) {
            eprintln!("Failed to create the cache directory: {err}");
            return Err(RuntimeError::InitFailed);
        }
    }

    setup_logger().map_err(|_| RuntimeError::InitFailed)?;

    // Begin creating first runner
    let mut runners: HashMap<u8, thread::JoinHandle<()>> = HashMap::new();

    runners.insert(0, summon_runner(0, CFG.playlist.clone())?);

    // Listen to commands
    while !runners.is_empty() {}
    Ok(())
}
