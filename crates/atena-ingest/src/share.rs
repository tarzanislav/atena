//! Sharing proposals between installations – without any third party.
//!
//! ATENA sends nothing about its users anywhere by itself (`atena_core::legality::CONNECTIONS`).
//! An installation keeps the addresses its users import – nothing else: no assessments, nothing
//! about the users – in its outbox. [`pending`] gathers them into one list – `atena-ingest share
//! FILE` writes it into a file, the desktop app shows it for copying – which the user hands over
//! as they choose; the editor reads such lists with [`receive`] (`atena-ingest proposals FILE…`),
//! assesses every address itself (a received assessment would prove nothing) and decides.
//! When the operator has a server of its own, which stores no IP addresses, it will take the lists
//! directly.
//!
//! A list: the line `ATENA-NAVRHY 1`, then one address or DOI per line, optionally followed by a
//! country code – the same lines a user types into an import.

use atena_assess::{Assessor, MAX_ITEMS, Rejected, parse_input};
use atena_store::{Store, StoreError, Via};

use crate::import::{Proposed, propose};

/// First line of every list.
pub const HEADER: &str = "ATENA-NAVRHY 1";
/// Most lines read from one list file.
pub const MAX_LINES: usize = 5_000;

/// The item lines of a list, or `None` when the text is not an ATENA list. At most
/// [`MAX_LINES`] lines are kept; anything else is for the import to judge.
pub fn read_list(text: &str) -> Option<Vec<String>> {
    let mut lines = text
        .lines()
        .map(|l| l.trim_start_matches('\u{feff}').trim())
        .filter(|l| !l.is_empty());
    if lines.next()? != HEADER {
        return None;
    }
    let items: Vec<String> = lines.take(MAX_LINES).map(str::to_string).collect();
    (!items.is_empty()).then_some(items)
}

/// One list of the given lines (each once, in their order).
pub fn write_list<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::from(HEADER);
    for line in lines.into_iter().map(str::trim) {
        if !line.is_empty() && seen.insert(line) {
            out.push('\n');
            out.push_str(line);
        }
    }
    out.push('\n');
    out
}

/// A stable ID of a list (its lines), so that a list received twice is assessed once.
pub fn list_id(lines: &[String]) -> String {
    let hash = atena_store::hex(&atena_store::hash(lines.join("\n").as_bytes()));
    format!("list-{}", &hash[..16])
}

/// The proposals waiting to be shared, as one list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The list (see [`write_list`]).
    pub list: String,
    /// Number of addresses in it.
    pub addresses: usize,
    /// The outbox entries it holds, to mark handed over with [`handed_over`].
    pub entries: Vec<i64>,
}

/// Everything waiting to be shared as one list, or `None` when nothing waits.
pub async fn pending(store: &Store) -> Result<Option<Pending>, StoreError> {
    let waiting = store.outbox_pending().await?;
    let lines: Vec<String> = waiting
        .iter()
        .filter_map(|m| read_list(&m.body))
        .flatten()
        .collect();
    if lines.is_empty() {
        return Ok(None);
    }
    let list = write_list(lines.iter().map(String::as_str));
    Ok(Some(Pending {
        addresses: list.lines().count() - 1,
        list,
        entries: waiting.iter().map(|m| m.id).collect(),
    }))
}

/// Marks outbox entries as handed over; what was added since stays waiting.
pub async fn handed_over(store: &Store, entries: &[i64]) -> Result<(), StoreError> {
    for id in entries {
        store.outbox_sent(*id).await?;
    }
    Ok(())
}

/// What reading a received list did.
#[derive(Debug, Clone)]
pub enum Received {
    /// The text is not an ATENA list.
    NotAList,
    /// The same list was read before (its ID).
    ReadBefore(String),
    /// The list was read; every address was assessed here and stored as a proposal.
    Read {
        /// ID of the list.
        id: String,
        /// The proposals.
        proposed: Vec<Proposed>,
        /// Lines that are not addresses or DOIs.
        rejected: Vec<Rejected>,
    },
}

/// Editor: reads a list from another installation, assesses every address itself (a received
/// assessment would prove nothing) and stores them as proposals. A list read before is skipped.
pub async fn receive(
    store: &Store,
    assessor: &Assessor,
    text: &str,
    concurrency: usize,
) -> Result<Received, StoreError> {
    let Some(lines) = read_list(text) else {
        return Ok(Received::NotAList);
    };
    let id = list_id(&lines);
    if !store
        .inbox_known(std::slice::from_ref(&id))
        .await?
        .is_empty()
    {
        return Ok(Received::ReadBefore(id));
    }
    let (mut proposed, mut rejected) = (Vec::new(), Vec::new());
    // an import takes at most MAX_ITEMS lines at a time
    for chunk in lines.chunks(MAX_ITEMS) {
        let parsed = parse_input(&chunk.join("\n"));
        rejected.extend(parsed.rejected);
        proposed.extend(propose(store, assessor, &parsed.items, Via::Shared, concurrency).await?);
    }
    let count = i32::try_from(proposed.len()).unwrap_or(i32::MAX);
    store.inbox_add(&id, &lines.join("\n"), count).await?;
    Ok(Received::Read {
        id,
        proposed,
        rejected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_written_and_read_back() {
        let lists = [
            format!("{HEADER}\nhttps://a.example.sk/ SK\nhttps://b.example.cz/"),
            format!("{HEADER}\nhttps://a.example.sk/ SK\n10.1002/cl2.1180"),
        ];
        let lines: Vec<String> = lists.iter().flat_map(|l| read_list(l).unwrap()).collect();
        let file = write_list(lines.iter().map(String::as_str));
        assert_eq!(
            file,
            format!(
                "{HEADER}\nhttps://a.example.sk/ SK\nhttps://b.example.cz/\n10.1002/cl2.1180\n"
            )
        );
        let back = read_list(&file).unwrap();
        assert_eq!(back.len(), 3);
        assert_eq!(list_id(&back), list_id(&back.clone()));
        assert_ne!(list_id(&back), list_id(&back[..2]));
    }

    #[test]
    fn other_texts_are_not_lists() {
        assert_eq!(read_list("hello\nhttps://a.example.sk/"), None);
        assert_eq!(read_list(HEADER), None);
        assert_eq!(read_list(""), None);
        // a byte order mark from an editor does not matter
        assert!(read_list(&format!("\u{feff}{HEADER}\r\nhttps://a.example.sk/\r\n")).is_some());
    }
}
