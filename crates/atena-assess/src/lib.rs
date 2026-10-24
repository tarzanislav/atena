//! ATENA rights assessment: what users propose, line by line, and what the law and the
//! publisher's terms allow ATENA to do with it.
//!
//! 1. [`parse_input`] reads one item per line – a web address, a bare domain or a DOI, optionally
//!    followed by a country code.
//! 2. [`Assessor`] reads each item the way a careful person would: robots.txt, TDMRep and the
//!    refusal of AI crawlers, the licence the page names, and the terms of use it links to,
//!    in any of the languages of `data/legal-phrases.yaml`.
//! 3. [`propose`] turns the evidence into a licence and terms for the register, always the
//!    strictest the evidence calls for, with a rights record quoting what was found.
//!
//! The proposal is only a proposal: the legality principle allows nothing that is uncertain, and a
//! machine may misread terms of use, so a person confirms it before ATENA crawls the source.

pub mod assess;
pub mod country;
pub mod input;
pub mod licence;
pub mod phrases;
pub mod terms;

pub use assess::{Assessment, Assessor, Kind, MAX_LEGAL_PAGES, Problem, Proposal, Reason, propose};
pub use country::country_of_host;
pub use input::{Item, MAX_ITEMS, Parsed, Rejected, Rejection, Target, parse_input};
pub use licence::{LicenceHint, licence_of_url, licences_in_text};
pub use phrases::Phrases;
pub use terms::{Finding, FindingKind};
