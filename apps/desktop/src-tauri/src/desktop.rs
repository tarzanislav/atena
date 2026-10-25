//! The window: starts the backend in the background, passes the interface's calls to it and ends
//! it cleanly when the window closes.

use std::path::PathBuf;
use std::sync::Arc;

use atena_app::{App, AppError, Backend, Start};
use tauri::Manager;
use tokio::sync::RwLock;

use crate::commands;

/// Where the backend is.
pub enum Phase {
    /// The database is starting.
    Starting,
    /// Ready.
    Ready(Backend),
    /// It could not start.
    Failed(AppError),
    /// The window closed.
    Closing,
}

/// What the commands share.
pub struct Shared {
    /// The backend.
    pub phase: RwLock<Phase>,
    /// The app's folder (for the settings before the backend is ready).
    pub folder: PathBuf,
}

impl Shared {
    /// The app, when it is ready.
    pub async fn app(&self) -> Result<Arc<App>, AppError> {
        match &*self.phase.read().await {
            Phase::Ready(backend) => Ok(backend.app()),
            Phase::Failed(e) => Err(e.clone()),
            Phase::Starting | Phase::Closing => {
                Err(AppError::new("not-ready", "ATENA is starting or closing"))
            }
        }
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

/// Where the app keeps its data and which database it uses. Developers may name their own
/// folder (`ATENA_DATA_DIR`), database (`ATENA_DATABASE_URL`) or PostgreSQL programs
/// (`ATENA_PG_BIN`); otherwise the user's application folder and the bundled PostgreSQL are used.
fn start_of(app: &tauri::App) -> Result<Start, Box<dyn std::error::Error>> {
    let folder = match env("ATENA_DATA_DIR") {
        Some(folder) => PathBuf::from(folder),
        None => app.path().app_local_data_dir()?,
    };
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|r| r.join("postgres").join("bin"))
        .filter(|bin| bin.exists());
    Ok(Start {
        folder,
        postgres_bin: env("ATENA_PG_BIN").map(PathBuf::from).or(bundled),
        database_url: env("ATENA_DATABASE_URL"),
    })
}

/// Runs the window until it is closed.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let start = start_of(app)?;
            crate::logging::init(&start.folder);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "ATENA starts");
            app.manage(Shared {
                phase: RwLock::new(Phase::Starting),
                folder: start.folder.clone(),
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let started = Backend::start(start).await;
                let shared = handle.state::<Shared>();
                let mut phase = shared.phase.write().await;
                match started {
                    Ok(backend) if matches!(*phase, Phase::Starting) => {
                        tracing::info!("ATENA is ready");
                        *phase = Phase::Ready(backend);
                    }
                    // the window closed meanwhile
                    Ok(backend) => backend.close().await,
                    Err(e) => {
                        tracing::error!("ATENA could not start: {e}");
                        *phase = Phase::Failed(e);
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup,
            commands::overview,
            commands::sources,
            commands::taxonomy,
            commands::search,
            commands::newest,
            commands::tally,
            commands::document,
            commands::import_addresses,
            commands::proposals,
            commands::decision_options,
            commands::confirm_proposal,
            commands::reject_proposal,
            commands::pending_share,
            commands::handed_over,
            commands::receive_list,
            commands::sync_shared_register,
            commands::forget,
            commands::crawl_start,
            commands::crawl_stop,
            commands::crawl_status,
            commands::get_settings,
            commands::set_settings,
            commands::open_link,
        ])
        .build(tauri::generate_context!())?
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                let shared = handle.state::<Shared>();
                tauri::async_runtime::block_on(async {
                    let phase = std::mem::replace(&mut *shared.phase.write().await, Phase::Closing);
                    if let Phase::Ready(backend) = phase {
                        backend.close().await;
                        tracing::info!("ATENA ended");
                    }
                });
            }
        });
    Ok(())
}
