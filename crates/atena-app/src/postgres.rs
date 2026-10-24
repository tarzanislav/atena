//! The database inside the app: a private PostgreSQL that starts and stops with ATENA.
//!
//! The user installs nothing. The PostgreSQL programs come with the app; the data lives in the
//! user's own folder. The server
//!
//! * listens only on `127.0.0.1`, on a port chosen at each start, and has no Unix socket;
//! * accepts one user, `atena`, only from this computer and only with a random password of 256
//!   bits kept next to the data, readable by its owner only (`pg_hba.conf` allows nothing else);
//! * writes no statement text into its log, so the log holds no collected text (pillar 2).
//!
//! Nothing about it leaves the computer. A server left running by an app that crashed is stopped
//! and started anew; the caller makes sure that only one app uses a folder at a time.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppError;

/// The PostgreSQL user and database ATENA uses.
const USER: &str = "atena";
const DATABASE: &str = "postgres";
/// The server log is renamed to `postgres.log.1` at a start when it is larger than this.
const LOG_LIMIT: u64 = 1024 * 1024;
/// Longest wait for the server to start or stop, in seconds.
const WAIT_S: &str = "60";

/// Settings added to `postgresql.conf` when the database is created.
const SETTINGS: &str = "
# ATENA: only this computer
listen_addresses = '127.0.0.1'
unix_socket_directories = ''
ssl = off
# ATENA: no statement text (it may hold collected text) in the log
log_min_error_statement = panic
timezone = 'UTC'
log_timezone = 'UTC'
";

/// The whole `pg_hba.conf`: the user `atena`, from this computer, with the password.
const HBA: &str = "\
# ATENA: one user, only from this computer, only with the password
host    postgres    atena    127.0.0.1/32    scram-sha-256
";

/// A running PostgreSQL; stopped by [`EmbeddedPostgres::stop`] or when dropped.
#[derive(Debug)]
pub struct EmbeddedPostgres {
    bin: PathBuf,
    data: PathBuf,
    port: u16,
    password: String,
    stopped: AtomicBool,
}

fn program(bin: &Path, name: &str) -> Command {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    #[allow(unused_mut)]
    let mut command = Command::new(bin.join(file));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // no console window flashing up from a window app
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn output(command: &mut Command, code: &'static str, what: &str) -> Result<Output, AppError> {
    command
        .output()
        .map_err(|e| AppError::new(code, format!("{what}: {e}")))
}

fn run(command: &mut Command, code: &'static str, what: &str) -> Result<(), AppError> {
    let out = output(command, code, what)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            code,
            format!("{what}: {}", String::from_utf8_lossy(&out.stderr).trim()),
        ))
    }
}

/// The last lines of the server log, for the detail of an error.
fn log_tail(log: &Path) -> String {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    let lines: Vec<&str> = text.lines().rev().take(8).collect();
    lines.into_iter().rev().collect::<Vec<_>>().join(" | ")
}

/// Starts the server. Its output goes to the log file only: the server inherits the standard
/// handles of `pg_ctl` (on Windows), so pipes to them would stay open as long as the server runs
/// and waiting for their end would never return.
fn start_server(bin: &Path, data: &Path, log: &Path, port: u16) -> Result<(), AppError> {
    let status = program(bin, "pg_ctl")
        .arg("start")
        .arg("-D")
        .arg(data)
        .arg("-l")
        .arg(log)
        .args(["-w", "-t", WAIT_S, "-o"])
        .arg(format!("-p {port}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| AppError::new("database-start", format!("starting the database: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "database-start",
            format!("starting the database ({status}): {}", log_tail(log)),
        ))
    }
}

/// A port nobody listens on right now.
fn free_port() -> Result<u16, AppError> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| AppError::new("database-start", format!("no free port: {e}")))?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| AppError::new("database-start", e.to_string()))
}

/// A new random password: 32 bytes, hexadecimal.
fn new_password() -> Result<String, AppError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|e| AppError::new("database-start", format!("no randomness: {e}")))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Writes a file only its owner may read. (On Windows the user's own application data folder
/// is already closed to other users.)
fn write_private(path: &Path, text: &str) -> Result<(), AppError> {
    std::fs::write(path, text).map_err(|e| AppError::io(path, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| AppError::io(path, e))?;
    }
    Ok(())
}

/// The major version in the first line of `postgres --version` or of `PG_VERSION`: `17` of
/// `postgres (PostgreSQL) 17.6` or of `17`.
fn major(text: &str) -> Option<&str> {
    let line = text.lines().next()?.trim();
    let version = match line.split_once("(PostgreSQL)") {
        Some((_, rest)) => rest.split_whitespace().next()?,
        None => line,
    };
    let end = version
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(version.len());
    (end > 0).then(|| &version[..end])
}

impl EmbeddedPostgres {
    /// Starts PostgreSQL from the programs in `bin` with its data in `folder`. The first start
    /// creates the database cluster.
    pub fn start(bin: &Path, folder: &Path) -> Result<Self, AppError> {
        let data = folder.join("pgdata");
        let secret = folder.join("pg-password");
        std::fs::create_dir_all(folder).map_err(|e| AppError::io(folder, e))?;
        let version = output(
            program(bin, "postgres").arg("--version"),
            "database-start",
            "PostgreSQL programs",
        )?;
        let programs = String::from_utf8_lossy(&version.stdout).to_string();
        let programs = major(&programs).ok_or_else(|| {
            AppError::new(
                "database-start",
                format!("PostgreSQL programs in {}: no version", bin.display()),
            )
        })?;
        if !data.join("PG_VERSION").exists() {
            create(bin, &data, &secret)?;
        }
        let made_by =
            std::fs::read_to_string(data.join("PG_VERSION")).map_err(|e| AppError::io(&data, e))?;
        if major(&made_by) != Some(programs) {
            return Err(AppError::new(
                "database-version",
                format!(
                    "the database was made by PostgreSQL {} and the app brings PostgreSQL {programs}",
                    made_by.trim()
                ),
            ));
        }
        let password = std::fs::read_to_string(&secret)
            .map_err(|e| AppError::io(&secret, e))?
            .trim()
            .to_string();
        // a server left by an app that crashed: stop it and start anew on a new port
        let status = output(
            program(bin, "pg_ctl").arg("status").arg("-D").arg(&data),
            "database-start",
            "checking the database",
        )?;
        if status.status.success() {
            tracing::warn!("stopping a database server left running");
            stop(bin, &data)?;
        }
        let log = folder.join("postgres.log");
        if std::fs::metadata(&log).is_ok_and(|m| m.len() > LOG_LIMIT) {
            std::fs::rename(&log, folder.join("postgres.log.1"))
                .map_err(|e| AppError::io(&log, e))?;
        }
        let port = free_port()?;
        start_server(bin, &data, &log, port)?;
        Ok(Self {
            bin: bin.to_path_buf(),
            data,
            port,
            password,
            stopped: AtomicBool::new(false),
        })
    }

    /// The connection address.
    pub fn url(&self) -> String {
        format!(
            "postgres://{USER}:{}@127.0.0.1:{}/{DATABASE}",
            self.password, self.port
        )
    }

    /// The port the server listens on (on `127.0.0.1`).
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stops PostgreSQL, letting it finish cleanly; once is enough.
    pub fn stop(&self) -> Result<(), AppError> {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        stop(&self.bin, &self.data)
    }
}

impl Drop for EmbeddedPostgres {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::warn!("{e}");
        }
    }
}

fn stop(bin: &Path, data: &Path) -> Result<(), AppError> {
    run(
        program(bin, "pg_ctl")
            .arg("stop")
            .arg("-D")
            .arg(data)
            .args(["-m", "fast", "-w", "-t", WAIT_S]),
        "database",
        "stopping the database",
    )
}

/// Creates the database cluster in `data` with a new password in `secret`, open to this computer
/// only.
fn create(bin: &Path, data: &Path, secret: &Path) -> Result<(), AppError> {
    // what an interrupted creation left behind
    if data.exists() {
        std::fs::remove_dir_all(data).map_err(|e| AppError::io(data, e))?;
    }
    write_private(secret, &new_password()?)?;
    run(
        program(bin, "initdb")
            .arg("-D")
            .arg(data)
            .args([
                "-U",
                USER,
                "-A",
                "scram-sha-256",
                "-E",
                "UTF8",
                "--no-locale",
            ])
            .arg(format!("--pwfile={}", secret.display())),
        "database-start",
        "creating the database",
    )?;
    let conf = data.join("postgresql.conf");
    let mut text = std::fs::read_to_string(&conf).map_err(|e| AppError::io(&conf, e))?;
    text.push_str(SETTINGS);
    std::fs::write(&conf, text).map_err(|e| AppError::io(&conf, e))?;
    let hba = data.join("pg_hba.conf");
    std::fs::write(&hba, HBA).map_err(|e| AppError::io(&hba, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read() {
        assert_eq!(major("postgres (PostgreSQL) 17.6\n"), Some("17"));
        assert_eq!(
            major("postgres (PostgreSQL) 16.10 (Ubuntu 16.10-0ubuntu0.24.04.1)"),
            Some("16")
        );
        assert_eq!(major("postgres (PostgreSQL) 18beta1"), Some("18"));
        assert_eq!(major("17\n"), Some("17"));
        assert_eq!(major(""), None);
        assert_eq!(major("postgres (PostgreSQL) beta"), None);
    }

    #[test]
    fn only_this_computer_is_let_in() {
        let rules: Vec<&str> = HBA
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(rules.len(), 1);
        let fields: Vec<&str> = rules[0].split_whitespace().collect();
        assert_eq!(
            fields,
            ["host", DATABASE, USER, "127.0.0.1/32", "scram-sha-256"]
        );
        assert!(SETTINGS.contains("listen_addresses = '127.0.0.1'"));
        assert!(SETTINGS.contains("unix_socket_directories = ''"));
    }

    #[test]
    fn passwords_are_long_and_new() {
        let (a, b) = (new_password().unwrap(), new_password().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
