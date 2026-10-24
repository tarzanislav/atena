//! ATENA storage: the PostgreSQL schema and every query. ATENA keeps no original files – only
//! texts, where the legality principle allows, and fingerprints (see [`hash`]).
//!
//! The rest of ATENA never writes SQL; it calls [`Store`]. The schema is in `migrations/`
//! and is applied with [`Store::migrate`]. Scheduling is built on host leases: a worker leases a
//! host, processes its due endpoints one after another and releases it, so several workers (also
//! in several processes) never send one host more than one request at a time.

mod documents;
mod erasure;
mod hash;
mod imports;
mod overview;
mod register;
mod schedule;
mod search;
#[cfg(feature = "testing")]
pub mod testing;

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

pub use documents::{
    Capture, CurrentDocument, DocumentLink, Label, LabelDecision, LabelSuggestion, LinkKind,
    NewVersion, Rights, SavedVersion, VersionSummary,
};
pub use erasure::ErasureReport;
/// The fingerprint of a byte string.
pub use hash::hash;
pub use hash::{Hash, hex, parse_hex};
pub use imports::{
    AssessmentRecord, ImportOutcome, ImportState, ImportedSource, InboxMessage, NewImport,
    OutboxMessage, Via,
};
pub use overview::{Overview, SourceSummary};
pub use register::{RegisterSource, SourceRecord, SyncReport};
pub use schedule::{
    Check, Endpoint, EndpointKind, HostLease, MAX_ENDPOINTS_PER_SOURCE, NewEndpoint, Origin,
};
pub use search::{IndexRow, QueueEntry};

/// The embedded migrations.
pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A storage failure.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A database error.
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    /// A migration failed.
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// Data that the store refuses (an unknown value, a missing row…).
    #[error("{0}")]
    Invalid(String),
}

/// Shorthand for store results.
pub type Result<T> = std::result::Result<T, StoreError>;

/// The database.
#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
}

/// Seconds as a floating-point number for `make_interval(secs => …)`.
pub(crate) fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

impl Store {
    /// Connects to `database_url` (e.g. `postgres://atena@localhost/atena`).
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self> {
        let options: PgConnectOptions = database_url.parse()?;
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(30))
            // server notices ("already exists, skipping") are not worth logging
            .connect_with(
                options
                    .application_name("atena")
                    .options([("client_min_messages", "warning")]),
            )
            .await?;
        Ok(Self::from_pool(pool))
    }

    /// A store from an existing pool.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Applies pending migrations.
    pub async fn migrate(&self) -> Result<()> {
        MIGRATIONS.run(&self.pool).await?;
        Ok(())
    }

    /// The connection pool (for read queries of other crates).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Deletes check records older than `age`; returns how many.
    pub async fn prune_fetch_log(&self, age: Duration) -> Result<u64> {
        let done =
            sqlx::query("DELETE FROM fetch_log WHERE at < now() - make_interval(secs => $1)")
                .bind(secs(age))
                .execute(&self.pool)
                .await?;
        Ok(done.rows_affected())
    }
}
