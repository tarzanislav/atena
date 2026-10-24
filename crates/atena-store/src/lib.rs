//! ATENA storage: the PostgreSQL schema and every query, plus the file store for originals.
//!
//! The rest of ATENA never writes SQL; it calls [`Store`]. The schema is in `migrations/`
//! and is applied with [`Store::migrate`]. Scheduling is built on host leases: a worker leases a
//! host, processes its due endpoints one after another and releases it, so several workers (also
//! in several processes) never send one host more than one request at a time.

mod blob;
mod documents;
mod erasure;
mod register;
mod schedule;
#[cfg(feature = "testing")]
pub mod testing;

use std::path::PathBuf;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

pub use blob::{BlobCompression, BlobStore, Hash, StoredBlob, hash, hex, parse_hex};
pub use documents::{
    Capture, CurrentDocument, DocumentLink, Label, LabelDecision, LabelSuggestion, LinkKind,
    NewVersion, Rights, SavedVersion, VersionSummary,
};
pub use erasure::ErasureReport;
pub use register::{RegisterSource, SourceRecord, SyncReport};
pub use schedule::{Check, Endpoint, EndpointKind, HostLease, NewEndpoint, Origin};

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
    /// A file store error.
    #[error("file store: {0}")]
    Io(#[from] std::io::Error),
    /// Data that the store refuses (an unknown value, a missing row…).
    #[error("{0}")]
    Invalid(String),
}

/// Shorthand for store results.
pub type Result<T> = std::result::Result<T, StoreError>;

/// The database and the file store.
#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
    blobs: BlobStore,
}

/// Seconds as a floating-point number for `make_interval(secs => …)`.
pub(crate) fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

impl Store {
    /// Connects to `database_url` (e.g. `postgres://atena@localhost/atena`) with files under `blob_root`.
    pub async fn connect(
        database_url: &str,
        blob_root: impl Into<PathBuf>,
        max_connections: u32,
    ) -> Result<Self> {
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
        Ok(Self::from_pool(pool, BlobStore::new(blob_root)))
    }

    /// A store from an existing pool.
    pub fn from_pool(pool: PgPool, blobs: BlobStore) -> Self {
        Self { pool, blobs }
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

    /// The file store.
    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    /// Stores an original file and records it; returns its hash.
    pub async fn put_blob(&self, bytes: Vec<u8>, content_type: Option<&str>) -> Result<StoredBlob> {
        let compression = BlobCompression::for_content_type(content_type);
        let blobs = self.blobs.clone();
        let stored = tokio::task::spawn_blocking(move || blobs.put(&bytes, compression))
            .await
            .map_err(|e| StoreError::Invalid(format!("file store task failed: {e}")))??;
        sqlx::query(
            "INSERT INTO blob (hash, size, stored_size, compression, content_type)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (hash) DO NOTHING",
        )
        .bind(&stored.hash[..])
        .bind(i64::try_from(stored.size).unwrap_or(i64::MAX))
        .bind(i64::try_from(stored.stored_size).unwrap_or(i64::MAX))
        .bind(stored.compression.as_str())
        .bind(content_type)
        .execute(&self.pool)
        .await?;
        Ok(stored)
    }

    /// Reads an original file back; `None` when it was never stored (e.g. TDM reservation).
    pub async fn get_blob(&self, hash: Hash) -> Result<Option<Vec<u8>>> {
        let compression: Option<String> =
            sqlx::query_scalar("SELECT compression FROM blob WHERE hash = $1")
                .bind(&hash[..])
                .fetch_optional(&self.pool)
                .await?;
        let Some(compression) = compression else {
            return Ok(None);
        };
        let compression = if compression == "gzip" {
            BlobCompression::Gzip
        } else {
            BlobCompression::None
        };
        let blobs = self.blobs.clone();
        Ok(
            tokio::task::spawn_blocking(move || blobs.get(&hash, compression))
                .await
                .map_err(|e| StoreError::Invalid(format!("file store task failed: {e}")))??,
        )
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
