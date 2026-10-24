//! The app's log: `logs/atena.log` in its folder, on this computer only. It records what the app
//! did and what failed – addresses of sources, never anything about the user, and never the
//! statements the database runs (they may carry collected text). The previous run's log is kept
//! as `atena.log.1`.

use std::fs::File;
use std::path::Path;
use std::sync::Mutex;

use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::prelude::*;

/// Starts writing the log; without a writable folder the app logs to the standard error.
pub fn init(folder: &Path) {
    let logs = folder.join("logs");
    let path = logs.join("atena.log");
    let file = std::fs::create_dir_all(&logs).and_then(|()| {
        if path.exists() {
            std::fs::rename(&path, logs.join("atena.log.1"))?;
        }
        File::create(&path)
    });
    let level = if cfg!(debug_assertions) {
        Level::DEBUG
    } else {
        Level::INFO
    };
    // the database library logs its statements below WARN
    let filter = Targets::new()
        .with_default(level)
        .with_target("sqlx", Level::WARN);
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(false);
    let registry = tracing_subscriber::registry().with(filter);
    let _ = match file {
        Ok(file) => registry
            .with(layer.with_writer(Mutex::new(file)))
            .try_init(),
        Err(_) => registry.with(layer.with_writer(std::io::stderr)).try_init(),
    };
}
