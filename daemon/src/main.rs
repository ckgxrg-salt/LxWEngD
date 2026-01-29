use lxwengd::{DaemonError, LxWEngd};

fn main() -> Result<(), DaemonError> {
    let mut daemon = LxWEngd::init().inspect_err(|err| eprintln!("{err}"))?;
    daemon.start();
    Ok(())
}
