//! ATENA core: text normalisation, the multilingual taxonomy, classification suggestions,
//! document versions and the source register. Shared by the server and the desktop app.
//!
//! ATENA stands on four pillars, set by the project author ([`PILLARS`]); the rules that put them
//! into practice are written here: the legality principle and whom ATENA connects to
//! ([`legality`]).

pub mod classify;
pub mod legality;
pub mod pages;
pub mod safety;
pub mod source;
pub mod taxonomy;
pub mod text;
pub mod version;

pub use classify::{Suggestion, suggest};
pub use legality::{CONNECTIONS, PRINCIPLE, Permission, Restriction, SourceFacts, permission};
pub use source::{Licence, Source, Terms, load_sources};
pub use taxonomy::{Concept, MissingLabel, Taxonomy};
pub use version::{Change, ChangeKind, compare};

/// The four pillars of ATENA, in the author's order (the Slovak original is in `README.sk.md`).
pub const PILLARS: [&str; 4] = [
    "Seriousness, openness and moving human knowledge forward – the knowledge of every person.",
    "Anonymity and respect for privacy – ours and everyone else's.",
    "Security and protection against dangerous people, robots, code and processes.",
    "Respect for the law – no law may be broken, anywhere.",
];
