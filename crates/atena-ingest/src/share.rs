//! Sharing proposals between installations – without any third party.
//!
//! ATENA sends nothing about its users anywhere by itself (`atena_core::legality::CONNECTIONS`).
//! An installation keeps the addresses its users import – nothing else: no assessments, nothing
//! about the users – in its outbox. `atena-ingest share FILE` writes them into a list file, which
//! the user hands over as they choose; the editor reads such files with `atena-ingest proposals
//! FILE…`, assesses every address itself (a received assessment would prove nothing) and decides.
//! When the operator has a server of its own, which stores no IP addresses, it will take the lists
//! directly.
//!
//! A list: the line `ATENA-NAVRHY 1`, then one address or DOI per line, optionally followed by a
//! country code – the same lines a user types into an import.

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
