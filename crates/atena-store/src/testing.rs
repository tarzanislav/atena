//! Test support: a fresh, migrated database per test. Only built with the `testing` feature.
//!
//! `ATENA_TEST_DATABASE_URL` must point to a server where tests may create databases, e.g.
//! `postgres://postgres@localhost/postgres`. Without it [`TestDb::new`] returns `None` and the
//! calling test is expected to return early (the test is skipped).

use std::sync::atomic::{AtomicU32, Ordering};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgConnection};

use crate::Store;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A temporary database; remove it with [`TestDb::drop`].
pub struct TestDb {
    admin: String,
    name: String,
    /// The store on the temporary database.
    pub store: Store,
}

impl TestDb {
    /// Creates and migrates a new database, or `None` when no test server is configured.
    pub async fn new() -> Option<Self> {
        let Ok(admin) = std::env::var("ATENA_TEST_DATABASE_URL") else {
            eprintln!("ATENA_TEST_DATABASE_URL is not set – database tests skipped");
            return None;
        };
        let name = format!(
            "atena_t_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let mut conn = PgConnection::connect(&admin)
            .await
            .expect("admin connection");
        conn.execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .expect("create database");
        conn.close().await.ok();
        let options: PgConnectOptions = admin.parse().expect("database URL");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options.database(&name))
            .await
            .expect("test database");
        let store = Store::from_pool(pool);
        store.migrate().await.expect("migrations");
        Some(Self { admin, name, store })
    }

    /// Drops the database.
    pub async fn drop(self) {
        self.store.pool().close().await;
        let mut conn = PgConnection::connect(&self.admin)
            .await
            .expect("admin connection");
        conn.execute(format!("DROP DATABASE {} WITH (FORCE)", self.name).as_str())
            .await
            .ok();
    }
}
