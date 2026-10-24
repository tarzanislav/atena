//! ATENA ingestion: from the source register to documents, versions and classification.
//!
//! * [`register_entries`] turns `data/sources.yaml` into sources and their register endpoints.
//! * [`Pipeline::check`] handles one endpoint: fetch, then feed, sitemap or page processing.
//! * [`run`] is the worker loop on host leases (see `atena-store`).
//!
//! The `atena-ingest` program wraps these as commands (`migrate`, `sync`, `run`, `fetch`).

pub mod pipeline;
pub mod register;
pub mod urls;
pub mod worker;

pub use pipeline::{Outcome, Pipeline, PipelineConfig};
pub use register::register_entries;
pub use worker::{RunStats, WorkerConfig, run};
