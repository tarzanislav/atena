//! Reading terms of use: sentences that ban automated access, reserve mining, limit use to
//! personal use, or merely state copyright.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::phrases::{Phrases, find, first, normalize};

/// Longest quote kept with a finding, in characters.
pub const MAX_QUOTE: usize = 300;
/// Findings of one kind kept per page.
const PER_KIND: usize = 3;

/// What a sentence of the terms says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// Robots, scrapers, crawling or automated access are not allowed.
    AutomationBan,
    /// The content may be accessed or used only for personal use.
    PersonalUseOnly,
    /// Text and data mining or AI use is reserved or not allowed.
    MiningReserved,
    /// A plain copyright notice ("all rights reserved").
    RightsReserved,
    /// Reuse only for non-commercial purposes.
    NonCommercial,
    /// An explicit permission to reuse.
    ReusePermitted,
}

impl FindingKind {
    /// `true` for findings that forbid ATENA to crawl at all.
    pub fn forbids_crawling(self) -> bool {
        matches!(self, Self::AutomationBan | Self::PersonalUseOnly)
    }
}

/// One sentence of a page that matters for the rights assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// What it says.
    pub kind: FindingKind,
    /// The sentence, as written (shortened to [`MAX_QUOTE`]).
    pub quote: String,
    /// The page it is on.
    pub page: Url,
}

/// Sentences of a text: split at line breaks and at `.` `!` `?` `;` and their CJK forms.
fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for block in text.split('\n') {
        let mut start = 0;
        let mut chars = block.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            let end_mark = matches!(c, '。' | '！' | '？' | '；')
                || (matches!(c, '.' | '!' | '?' | ';')
                    && chars.peek().is_none_or(|(_, n)| n.is_whitespace()));
            if end_mark {
                let end = i + c.len_utf8();
                out.push(block[start..end].trim());
                start = end;
            }
        }
        out.push(block[start..].trim());
    }
    out.retain(|s| !s.is_empty());
    out
}

fn quote(sentence: &str) -> String {
    let mut q: String = sentence.chars().take(MAX_QUOTE).collect();
    if sentence.chars().count() > MAX_QUOTE {
        q.push('…');
    }
    q
}

/// What one sentence says, if anything.
fn kind_of(normalised: &str, p: &Phrases) -> Option<FindingKind> {
    let paired = |pair: &crate::phrases::Pair| {
        first(normalised, &pair.subjects).is_some() && first(normalised, &pair.markers).is_some()
    };
    if paired(&p.automation) {
        Some(FindingKind::AutomationBan)
    } else if first(normalised, &p.personal_use).is_some() {
        Some(FindingKind::PersonalUseOnly)
    } else if paired(&p.mining) {
        Some(FindingKind::MiningReserved)
    } else if first(normalised, &p.rights_reserved).is_some() {
        Some(FindingKind::RightsReserved)
    } else if first(normalised, &p.non_commercial).is_some() {
        Some(FindingKind::NonCommercial)
    } else if first(normalised, &p.reuse_permitted).is_some() {
        Some(FindingKind::ReusePermitted)
    } else {
        None
    }
}

/// `true` when a block of an ordinary page reads like a legal statement (a footer line with ©,
/// a licence note …).
pub fn is_legal_block(block: &str, p: &Phrases) -> bool {
    let n = normalize(block);
    p.legal_context.iter().any(|w| find(&n, w).is_some())
}

/// The findings of a text from `page`. With `legal_only`, only blocks that read like legal
/// statements are considered – for ordinary pages, whose articles may discuss robots or AI.
pub fn analyse(text: &str, page: &Url, p: &Phrases, legal_only: bool) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    for block in text.split("\n\n") {
        if legal_only && !is_legal_block(block, p) {
            continue;
        }
        for sentence in sentences(block) {
            let Some(kind) = kind_of(&normalize(sentence), p) else {
                continue;
            };
            let quote = quote(sentence);
            let same_kind = findings.iter().filter(|f| f.kind == kind).count();
            if same_kind < PER_KIND && !findings.iter().any(|f| f.quote == quote) {
                findings.push(Finding {
                    kind,
                    quote,
                    page: page.clone(),
                });
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str, legal_only: bool) -> Vec<FindingKind> {
        let page = Url::parse("https://pub.example/terms").unwrap();
        analyse(text, &page, Phrases::bundled(), legal_only)
            .into_iter()
            .map(|f| f.kind)
            .collect()
    }

    #[test]
    fn bans_reservations_and_notices_in_many_languages() {
        use FindingKind::*;
        let cases: [(&str, FindingKind); 12] = [
            (
                "You may not use any robot, spider or other automatic device to access the site.",
                AutomationBan,
            ),
            (
                "Users must not scrape or download data from any web pages.",
                AutomationBan,
            ),
            (
                "Je zakázané používať robotov a automatizované prostriedky na sťahovanie obsahu.",
                AutomationBan,
            ),
            (
                "Es ist untersagt, Software, Robots oder Webcrawler einzusetzen.",
                AutomationBan,
            ),
            ("未经许可，禁止使用爬虫抓取本网站内容。", AutomationBan),
            (
                "You may access the content only for your own personal, non-commercial use.",
                PersonalUseOnly,
            ),
            (
                "This content is not licensed for text and data mining.",
                MiningReserved,
            ),
            (
                "Vydavateľ si vyhradzuje právo na vyťažovanie textov a dát podľa § 44b.",
                MiningReserved,
            ),
            (
                "Wir behalten uns die Nutzung für Text- und Data-Mining ausdrücklich vor.",
                MiningReserved,
            ),
            ("© 2026 Škola. Všetky práva vyhradené.", RightsReserved),
            (
                "Material may be used for non-commercial purposes with attribution.",
                NonCommercial,
            ),
            (
                "Reproduction is authorised, provided the source is acknowledged.",
                ReusePermitted,
            ),
        ];
        for (text, kind) in cases {
            assert_eq!(kinds(text, false), [kind], "{text}");
        }
    }

    #[test]
    fn ordinary_sentences_are_not_findings() {
        for text in [
            "Our pupils build robots in the robotics club.",
            "Robotika je obľúbený krúžok, prihlásiť sa môžete do piatku.",
            "Teacher training starts in September.",
            "Search engines may index the site according to robots.txt.",
        ] {
            assert!(kinds(text, false).is_empty(), "{text}");
        }
    }

    #[test]
    fn ordinary_pages_are_read_only_where_they_speak_about_rights() {
        let page = "AI in schools: teachers must not let artificial intelligence replace thinking.\n\n\
                    © 2026 Ministry. All rights reserved. No automated access by robots.";
        assert_eq!(
            kinds(page, true),
            [FindingKind::RightsReserved, FindingKind::AutomationBan]
        );
    }

    #[test]
    fn sentences_split_without_breaking_numbers_or_urls() {
        assert_eq!(
            sentences("Version 2.0 applies. See www.example.org! Next"),
            ["Version 2.0 applies.", "See www.example.org!", "Next"]
        );
    }
}
