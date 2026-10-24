//! Document versions: deciding whether a new capture is a change and how important it is.
//!
//! Numbers and dates decide deadlines and obligations, so any change in them is substantive.

use similar::{ChangeTag, TextDiff};

use crate::text::{fingerprint, paragraphs};

/// Minimal number of changed characters for a text change to count as substantive.
const SUBSTANTIVE_CHARS: usize = 40;

/// How a new capture differs from the current version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// First capture of the document.
    Initial,
    /// Same content (spacing and typography ignored) – no new version.
    Unchanged,
    /// Small wording change.
    Minor,
    /// New or removed content, or any change of a number or date.
    Substantive,
}

/// The result of comparing two captures.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// Classification of the change.
    pub kind: ChangeKind,
    /// Paragraphs only in the new version.
    pub added: Vec<String>,
    /// Paragraphs only in the old version.
    pub removed: Vec<String>,
    /// `true` when the set of numbers or dates changed.
    pub numbers_changed: bool,
}

/// Numbers in a text (`20`, `3,5`, `2026`); dates contribute their parts.
fn numbers(paras: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for p in paras {
        let mut cur = String::new();
        let chars: Vec<char> = p.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            let separator = matches!(c, '.' | ',')
                && !cur.is_empty()
                && chars.get(i + 1).is_some_and(char::is_ascii_digit);
            if c.is_ascii_digit() || separator {
                cur.push(if *c == ',' { '.' } else { *c });
            } else if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out.sort();
    out
}

/// Characters inserted or deleted between two texts, so a one-word edit inside a paragraph stays small.
fn changed_chars(old: &str, new: &str) -> usize {
    TextDiff::from_chars(old, new)
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
        .count()
}

fn as_strs(paras: &[String]) -> Vec<&str> {
    paras.iter().map(String::as_str).collect()
}

/// Compares the current version (if any) with a new capture.
pub fn compare(old: Option<&str>, new: &str) -> Change {
    let empty = |kind| Change {
        kind,
        added: Vec::new(),
        removed: Vec::new(),
        numbers_changed: false,
    };
    let Some(old) = old else {
        return empty(ChangeKind::Initial);
    };
    if fingerprint(old) == fingerprint(new) {
        return empty(ChangeKind::Unchanged);
    }
    let (a, b) = (paragraphs(old), paragraphs(new));
    let (a, b) = (as_strs(&a), as_strs(&b));
    let diff = TextDiff::configure().diff_slices(&a, &b);
    let (mut added, mut removed) = (Vec::new(), Vec::new());
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added.push(change.value().to_string()),
            ChangeTag::Delete => removed.push(change.value().to_string()),
            ChangeTag::Equal => {}
        }
    }
    let numbers_changed = numbers(&added) != numbers(&removed);
    let changed_chars = changed_chars(&removed.join("\n"), &added.join("\n"));
    let kind = if numbers_changed || changed_chars >= SUBSTANTIVE_CHARS {
        ChangeKind::Substantive
    } else {
        ChangeKind::Minor
    };
    Change {
        kind,
        added,
        removed,
        numbers_changed,
    }
}

/// A line of a side-by-side or unified view of a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Unchanged context paragraph.
    Same(String),
    /// Added paragraph.
    Added(String),
    /// Removed paragraph.
    Removed(String),
    /// Skipped unchanged paragraphs.
    Gap,
}

/// Paragraph diff for display, with `context` unchanged paragraphs around each change.
pub fn diff_lines(old: &str, new: &str, context: usize) -> Vec<DiffLine> {
    let (a, b) = (paragraphs(old), paragraphs(new));
    let (a, b) = (as_strs(&a), as_strs(&b));
    let diff = TextDiff::configure().diff_slices(&a, &b);
    let mut out = Vec::new();
    for (i, group) in diff.grouped_ops(context).iter().enumerate() {
        if i > 0 {
            out.push(DiffLine::Gap);
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let text = change.value().to_string();
                out.push(match change.tag() {
                    ChangeTag::Equal => DiffLine::Same(text),
                    ChangeTag::Insert => DiffLine::Added(text),
                    ChangeTag::Delete => DiffLine::Removed(text),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "Termín podania prihlášok je 20. 3. 2026.\n\nŠkola zverejní kritériá prijatia.\n\nKontakt: riaditeľ.";

    #[test]
    fn spacing_is_not_a_change() {
        let spaced = OLD.replace(' ', "  ").replace("\n\n", "\n\n\n");
        assert_eq!(compare(Some(OLD), &spaced).kind, ChangeKind::Unchanged);
    }

    #[test]
    fn a_new_date_is_substantive() {
        let c = compare(Some(OLD), &OLD.replace("20. 3. 2026", "27. 3. 2026"));
        assert_eq!(c.kind, ChangeKind::Substantive);
        assert!(c.numbers_changed);
    }

    #[test]
    fn a_typo_is_minor() {
        assert_eq!(
            compare(Some(OLD), &OLD.replace("riaditeľ.", "riaditeľ školy.")).kind,
            ChangeKind::Minor
        );
    }

    #[test]
    fn a_new_paragraph_is_substantive() {
        let new = format!(
            "{OLD}\n\nOd septembra platí nová povinnosť oznámiť zmenu zákonnému zástupcovi."
        );
        let c = compare(Some(OLD), &new);
        assert_eq!(c.kind, ChangeKind::Substantive);
        assert_eq!(c.added.len(), 1);
    }

    #[test]
    fn first_capture_and_display_lines() {
        assert_eq!(compare(None, OLD).kind, ChangeKind::Initial);
        let lines = diff_lines(OLD, &OLD.replace("20. 3.", "27. 3."), 1);
        assert!(lines.contains(&DiffLine::Removed(
            "Termín podania prihlášok je 20. 3. 2026.".into()
        )));
        assert!(lines.contains(&DiffLine::Added(
            "Termín podania prihlášok je 27. 3. 2026.".into()
        )));
    }
}
