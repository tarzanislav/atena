//! The database inside the app. Needs the PostgreSQL programs: `ATENA_TEST_PG_BIN` names their
//! `bin` folder (e.g. `/usr/lib/postgresql/17/bin`); without it the test is skipped. PostgreSQL
//! does not run as root.

use std::path::PathBuf;

use atena_app::EmbeddedPostgres;
use sqlx::{Connection, PgConnection};

fn bin() -> Option<PathBuf> {
    let Ok(bin) = std::env::var("ATENA_TEST_PG_BIN") else {
        eprintln!("ATENA_TEST_PG_BIN is not set – the embedded database test is skipped");
        return None;
    };
    Some(bin.into())
}

#[tokio::test]
async fn the_database_runs_for_this_computer_only_and_keeps_its_data() {
    let Some(bin) = bin() else {
        return;
    };
    let folder = std::env::temp_dir().join(format!("atena-pg-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);

    let first = {
        let (bin, folder) = (bin.clone(), folder.clone());
        tokio::task::spawn_blocking(move || EmbeddedPostgres::start(&bin, &folder))
            .await
            .unwrap()
            .unwrap()
    };
    let url = first.url();
    assert!(url.starts_with("postgres://atena:") && url.contains("@127.0.0.1:"));
    let mut conn = PgConnection::connect(&url).await.unwrap();
    let (listen,): (String,) = sqlx::query_as("SHOW listen_addresses")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(listen, "127.0.0.1");
    let (statements,): (String,) = sqlx::query_as("SHOW log_min_error_statement")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(statements, "panic");
    sqlx::query("CREATE TABLE kept (id integer)")
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();

    // the password is the only way in
    let wrong = url.replacen("atena:", "atena:0", 1);
    assert!(PgConnection::connect(&wrong).await.is_err());
    let other_user = url.replacen("atena:", "postgres:", 1);
    assert!(PgConnection::connect(&other_user).await.is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(folder.join("pg-password"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // an app that crashed leaves its server running; the next start replaces it
    std::mem::forget(first);
    let second = {
        let (bin, folder) = (bin.clone(), folder.clone());
        tokio::task::spawn_blocking(move || EmbeddedPostgres::start(&bin, &folder))
            .await
            .unwrap()
            .unwrap()
    };
    assert!(PgConnection::connect(&url).await.is_err() || url == second.url());
    let mut conn = PgConnection::connect(&second.url()).await.unwrap();
    let (tables,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM pg_tables WHERE tablename = 'kept'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(tables, 1, "the data stays");
    conn.close().await.unwrap();
    let second_url = second.url();
    tokio::task::spawn_blocking(move || second.stop())
        .await
        .unwrap()
        .unwrap();
    assert!(PgConnection::connect(&second_url).await.is_err());
    std::fs::remove_dir_all(&folder).unwrap();
}

#[tokio::test]
async fn the_whole_backend_starts_once_per_folder_and_stops() {
    let Some(bin) = bin() else {
        return;
    };
    let folder = std::env::temp_dir().join(format!("atena-backend-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let start = atena_app::Start {
        folder: folder.clone(),
        postgres_bin: Some(bin),
        database_url: None,
    };
    let backend = atena_app::Backend::start(start.clone()).await.unwrap();
    let overview = backend.app().overview().await.unwrap();
    assert!(overview.sources > 50);
    // a second app on the same folder would stop the first one's database
    let second = atena_app::Backend::start(start.clone()).await;
    assert_eq!(second.err().map(|e| e.code), Some("already-running"));
    backend.close().await;
    // after a clean end the folder is free again and the data is there
    let again = atena_app::Backend::start(start).await.unwrap();
    assert_eq!(
        again.app().overview().await.unwrap().sources,
        overview.sources
    );
    again.close().await;
    std::fs::remove_dir_all(&folder).unwrap();
}
