//! ATENA core: text normalisation, the multilingual taxonomy, classification suggestions,
//! document versions and the source register. Shared by the server and the desktop app.

pub mod classify;
pub mod source;
pub mod taxonomy;
pub mod text;
pub mod version;

pub use classify::{Suggestion, suggest};
pub use source::{Source, load_sources};
pub use taxonomy::{Concept, Taxonomy};
pub use version::{Change, ChangeKind, compare};
