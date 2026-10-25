//! ATENA ingestion: from the source register to documents, versions and classification.
//!
//! * [`register_entries`] turns `data/sources.yaml` into sources and their register endpoints.
//! * [`Pipeline::check`] handles one endpoint: fetch, then feed, sitemap or page processing.
//! * [`run`] is the worker loop on host leases (see `atena-store`).
//!
//! * [`import`] and [`share`] – proposals of users: imported sources, decisions, and the lists
//!   shared between installations.
//! * [`indexing`] keeps the search index in step with the database.
//!
//! The `atena-ingest` program and the desktop app (`atena-app`) wrap these.

pub mod classifying;
pub mod import;
pub mod indexing;
pub mod pipeline;
pub mod privacy;
pub mod register;
pub mod share;
pub mod urls;
pub mod worker;

pub use pipeline::{Outcome, Pipeline, PipelineConfig};
pub use register::register_entries;
pub use worker::{Progress, RunStats, WorkerConfig, run, run_observed};

use std::time::Duration;

use atena_extract::Parser;

/// Longest time for parsing one document.
pub const PARSE_TIME: Duration = Duration::from_secs(60);

/// Documents are parsed in a limited child process of the running program, which must answer
/// [`atena_extract::WORKER_ARG`] with [`atena_extract::worker_main`] and count its allocations.
pub fn parser() -> Parser {
    match std::env::current_exe() {
        Ok(program) => Parser::isolated(program, PARSE_TIME),
        Err(e) => {
            tracing::warn!("parsing in this process, without a memory limit: {e}");
            Parser::InProcess {
                time_limit: PARSE_TIME,
            }
        }
    }
}
