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
                "[{} {} {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                thread::current().name().unwrap(),
                record.level(),
                message
            ));
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

// Cli main entry
fn main() -> Result<(), RuntimeError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_location() {
        unsafe {
            env::set_var("XDG_CONFIG_HOME", ".");
            assert_eq!(sys_config_dir().unwrap(), PathBuf::from("./lxwengd"));
            env::remove_var("XDG_CONFIG_HOME");
            env::set_var("HOME", ".");
            assert_eq!(
                sys_config_dir().unwrap(),
                PathBuf::from("./.config/lxwengd")
            );
            env::remove_var("HOME");
            assert!(sys_config_dir().is_err());
        }
    }

    #[test]
    fn cache_location() {
        unsafe {
            env::set_var("XDG_CACHE_HOME", ".");
            assert_eq!(sys_cache_dir(), PathBuf::from("./lxwengd"));
            env::remove_var("XDG_CACHE_HOME");
            env::set_var("HOME", ".");
            assert_eq!(sys_cache_dir(), PathBuf::from("./.cache/lxwengd"));
            env::remove_var("HOME");
            assert_eq!(sys_cache_dir(), PathBuf::from("/tmp/lxwengd"));
        }
    }
}
