//! The ATENA desktop app without its window: the database inside the app, the operations the user
//! interface calls and the errors it shows. The window (`apps/desktop`, Tauri) only passes calls
//! through; everything is decided and checked here, so it can be tested without a window.
//!
//! * [`Backend::start`] locks the app's folder, starts the bundled PostgreSQL
//!   ([`EmbeddedPostgres`]), and opens the [`App`].
//! * [`App`] holds the database, the search index and the crawler; its methods are the operations
//!   of the interface. Network access happens only on the user's command: the crawler, an import,
//!   reading the shared register (`atena_core::legality::CONNECTIONS`).
//! * Every error reaches the interface as an [`AppError`]: a code it translates and a detail.

mod app;
mod crawl;
mod error;
mod postgres;
mod settings;
pub mod views;

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use atena_store::Store;

pub use app::{App, MAX_INPUT_BYTES, MAX_REASON_CHARS, PER_PAGE};
pub use atena_ingest::import::Choice;
pub use error::{AppError, CODES};
pub use postgres::EmbeddedPostgres;
pub use settings::{LANGUAGES, MAX_CONCURRENCY, MAX_EDITOR_CHARS, Settings, Theme};

/// `true` for an address the interface may open in the user's own browser: `http` or `https`
/// with a host, without credentials, white space or hidden characters, of limited length.
pub fn openable(url: &str) -> bool {
    atena_core::safety::is_storable_url(url)
        && url::Url::parse(url).is_ok_and(|u| {
            matches!(u.scheme(), "http" | "https")
                && u.host().is_some()
                && u.username().is_empty()
                && u.password().is_none()
        })
}

/// Database connections of the app.
const MAX_CONNECTIONS: u32 = 24;

/// Where the app keeps its things and which database it uses.
#[derive(Debug, Clone)]
pub struct Start {
    /// The app's folder: the database, the search index, the settings.
    pub folder: PathBuf,
    /// The `bin` folder of the bundled PostgreSQL programs.
    pub postgres_bin: Option<PathBuf>,
    /// A database of one's own instead of the bundled one (for developers).
    pub database_url: Option<String>,
}

/// The running app: the [`App`] and the database it brought.
pub struct Backend {
    app: Arc<App>,
    postgres: Option<EmbeddedPostgres>,
    _lock: File,
}

/// Makes sure only one app uses `folder`: a second one would stop the first one's database.
fn lock(folder: &std::path::Path) -> Result<File, AppError> {
    std::fs::create_dir_all(folder).map_err(|e| AppError::io(folder, e))?;
    let path = folder.join("atena.lock");
    let file = File::create(&path).map_err(|e| AppError::io(&path, e))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(AppError::new(
            "already-running",
            format!("{} is used by another ATENA", folder.display()),
        )),
        Err(std::fs::TryLockError::Error(e)) => Err(AppError::io(&path, e)),
    }
}

impl Backend {
    /// Starts the app as `start` says.
    pub async fn start(start: Start) -> Result<Self, AppError> {
        let lock = lock(&start.folder)?;
        let (postgres, url) = match start.database_url {
            Some(url) => (None, url),
            None => {
                let bin = start.postgres_bin.ok_or_else(|| {
                    AppError::new("database-start", "the PostgreSQL programs were not found")
                })?;
                let folder = start.folder.join("postgres");
                let postgres =
                    tokio::task::spawn_blocking(move || EmbeddedPostgres::start(&bin, &folder))
                        .await
                        .map_err(|e| AppError::new("internal", e.to_string()))??;
                let url = postgres.url();
                (Some(postgres), url)
            }
        };
        let store = Store::connect(&url, MAX_CONNECTIONS).await?;
        let app = App::open(store, &start.folder).await?;
        Ok(Self {
            app: Arc::new(app),
            postgres,
            _lock: lock,
        })
    }

    /// The app.
    pub fn app(&self) -> Arc<App> {
        Arc::clone(&self.app)
    }

    /// Ends the app and stops its database.
    pub async fn close(self) {
        self.app.close().await;
        if let Some(postgres) = self.postgres {
            let stopped = tokio::task::spawn_blocking(move || postgres.stop()).await;
            if let Ok(Err(e)) = stopped {
                tracing::warn!("{e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::openable;

    #[test]
    fn only_web_addresses_are_opened() {
        assert!(openable("https://www.minedu.sk/oznamy/?a=1#b"));
        assert!(openable("http://example.sk/"));
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ms-settings:privacy",
            "https://user:secret@example.sk/",
            "https://example.sk/a b",
            "https://example.sk/\u{202e}",
            "",
            "example.sk",
        ] {
            assert!(!openable(bad), "{bad}");
        }
        assert!(!openable(&format!(
            "https://example.sk/{}",
            "a".repeat(3_000)
        )));
    }
}
