//! ATENA search: a multilingual full-text index (Tantivy) of what the legality principle allows.
//!
//! * Every language group has its own analyzer: ATENA's light stemmers for Slovak, Czech and
//!   Polish, Snowball stemmers for 17 other languages, pairs of characters for Chinese, Japanese
//!   and Korean; diacritics are folded everywhere.
//! * Concepts of the taxonomy carry a search across languages.
//! * Nothing is found that ATENA may not collect or that the publisher marked `noindex`; text is
//!   searched only where ATENA may keep it and shown only where it may be shown.
//! * The database queues every change (migration 0006); [`update`] brings the index up to date.

mod analysis;
mod document;
mod index;
mod query;
mod schema;
mod stem;
#[cfg(test)]
mod tests;
mod update;
mod words;

use std::path::PathBuf;

pub use document::{SearchDocument, authority_weight, searchable};
pub use index::SearchIndex;
pub use query::{
    FacetCount, Hit, MAX_LIMIT, MAX_OFFSET, RecognisedConcept, Results, SearchRequest, Snippet,
};
pub use stem::Slavic;
pub use update::{DEFAULT_BATCH, UpdateReport, rebuild, update};

/// Errors of the search.
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// The index itself failed.
    #[error("search index: {0}")]
    Index(#[from] tantivy::TantivyError),
    /// The database failed.
    #[error("database: {0}")]
    Store(#[from] atena_store::StoreError),
    /// A file of the index could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// The error.
        source: std::io::Error,
    },
    /// The directory holds something else; ATENA neither opens nor deletes it.
    #[error("{}: not an ATENA search index – choose a new or empty directory", .0.display())]
    NotAnIndex(PathBuf),
    /// The index was made by another version of ATENA.
    #[error("{}: the index was made by another version of ATENA – rebuild it (atena-ingest index --rebuild)", .0.display())]
    Outdated(PathBuf),
    /// Neither words nor filters.
    #[error("nothing to search for: give words or at least one filter")]
    EmptyQuery,
    /// A filter that is not `scheme/concept`, `language/xx` or `source/ID`.
    #[error("unknown filter {0} (scheme/concept, language/xx or source/ID)")]
    Filter(String),
    /// More filters than one search may have.
    #[error("too many filters (at most {0})")]
    TooManyFilters(usize),
}

/// Result of the search crate.
pub type Result<T> = std::result::Result<T, SearchError>;
