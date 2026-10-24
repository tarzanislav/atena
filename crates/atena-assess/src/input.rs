//! Lines a user types or pastes: one item per line – a web address, a bare domain or a DOI,
//! optionally followed by a country code. Empty lines and lines starting with `#` are skipped.

use serde::{Deserialize, Serialize};
use url::Url;

/// Most items read from one input.
pub const MAX_ITEMS: usize = 500;
/// Longest line considered.
pub const MAX_LINE: usize = 2_000;

/// What a line points to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// A web address: a page, a feed, a sitemap, a document or an OAI-PMH interface.
    Url(Url),
    /// A DOI (lower-cased, without a prefix): `10.1234/abc`.
    Doi(String),
}

impl Target {
    /// The address the target is reached at.
    pub fn url(&self) -> Url {
        match self {
            Self::Url(u) => u.clone(),
            Self::Doi(doi) => {
                Url::parse(&format!("https://doi.org/{doi}")).expect("a DOI is a valid URL path")
            }
        }
    }
}

/// One accepted line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Line number, from 1.
    pub line: usize,
    /// What it points to.
    pub target: Target,
    /// The country the user gave, if any (upper case).
    pub country: Option<String>,
}

/// Why a line was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rejection {
    /// Neither a web address nor a DOI.
    NotAnAddress,
    /// Not `http`/`https` (`ftp:`, `file:`, `javascript:` …).
    UnsupportedScheme,
    /// The address contains a user name or password.
    Credentials,
    /// The same item appeared earlier.
    Duplicate,
    /// More than [`MAX_ITEMS`] items.
    TooMany,
    /// Longer than [`MAX_LINE`].
    TooLong,
}

/// A line that was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejected {
    /// Line number, from 1.
    pub line: usize,
    /// The line as written (shortened).
    pub text: String,
    /// Why.
    pub reason: Rejection,
}

/// The result of reading an input.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parsed {
    /// Accepted lines.
    pub items: Vec<Item>,
    /// Lines that were not accepted.
    pub rejected: Vec<Rejected>,
}

/// A DOI in any common form: `10.1234/x`, `doi:10.1234/x`, `https://doi.org/10.1234/x`.
fn doi_of(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let rest = lower
        .strip_prefix("doi:")
        .or_else(|| {
            [
                "https://doi.org/",
                "http://doi.org/",
                "https://dx.doi.org/",
                "http://dx.doi.org/",
            ]
            .iter()
            .find_map(|p| lower.strip_prefix(p))
        })
        .unwrap_or(&lower)
        .trim();
    let (prefix, suffix) = rest.split_once('/')?;
    let registrant = prefix.strip_prefix("10.")?;
    let valid = !registrant.is_empty()
        && registrant
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && !suffix.is_empty()
        && !suffix.chars().any(char::is_whitespace);
    valid.then(|| format!("{prefix}/{suffix}"))
}

/// A country code as the register writes them: `SK`, `GB-ENG`, `WORLD`.
fn is_country(word: &str) -> bool {
    (2..=6).contains(&word.len())
        && word.bytes().all(|b| b.is_ascii_uppercase() || b == b'-')
        && !word.starts_with('-')
}

fn target_of(text: &str) -> Result<Target, Rejection> {
    if let Some(doi) = doi_of(text) {
        return Ok(Target::Doi(doi));
    }
    let candidate = match text.split_once("://") {
        Some((scheme, _)) if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") => {
            return Err(Rejection::UnsupportedScheme);
        }
        Some(_) => text.to_string(),
        None if text.contains(':') && !text.contains('/') && !text.contains('.') => {
            return Err(Rejection::UnsupportedScheme);
        }
        None => format!("https://{text}"),
    };
    let url = Url::parse(&candidate).map_err(|_| Rejection::NotAnAddress)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Rejection::Credentials);
    }
    let host = url.host_str().ok_or(Rejection::NotAnAddress)?;
    if !host.contains('.') && !host.contains(':') {
        return Err(Rejection::NotAnAddress);
    }
    let mut url = url;
    url.set_fragment(None);
    Ok(Target::Url(url))
}

/// Reads an input of one item per line.
pub fn parse_input(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let shown: String = trimmed.chars().take(200).collect();
        let reject = |parsed: &mut Parsed, reason| {
            parsed.rejected.push(Rejected {
                line,
                text: shown.clone(),
                reason,
            })
        };
        if trimmed.len() > MAX_LINE {
            reject(&mut parsed, Rejection::TooLong);
            continue;
        }
        let mut words = trimmed.split_whitespace();
        let first = words
            .next()
            .unwrap_or_default()
            .trim_matches(|c| matches!(c, '<' | '>' | '"' | '\'' | '(' | ')' | ',' | ';'));
        let country = words.next().filter(|w| is_country(w)).map(str::to_string);
        match target_of(first) {
            Err(reason) => reject(&mut parsed, reason),
            Ok(target) if parsed.items.iter().any(|i| i.target == target) => {
                reject(&mut parsed, Rejection::Duplicate);
            }
            Ok(_) if parsed.items.len() >= MAX_ITEMS => reject(&mut parsed, Rejection::TooMany),
            Ok(target) => parsed.items.push(Item {
                line,
                target,
                country,
            }),
        }
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_line() {
        let input = "# návrhy\n\
            https://www.minedu.sk/  SK\n\
            www.npi.cz/aktuality\n\
            <https://eduscol.education.gouv.fr/feed>,\n\
            doi:10.1002/CL2.1180\n\
            https://doi.org/10.1002/cl2.1180\n\
            10.5281/zenodo.123 WORLD\n\
            ftp://files.example.org/x\n\
            https://user:secret@example.org/\n\
            nonsense\n\
            \n\
            www.npi.cz/aktuality#top\n";
        let p = parse_input(input);
        let items: Vec<(usize, String, Option<&str>)> = p
            .items
            .iter()
            .map(|i| {
                let t = match &i.target {
                    Target::Url(u) => u.to_string(),
                    Target::Doi(d) => format!("doi {d}"),
                };
                (i.line, t, i.country.as_deref())
            })
            .collect();
        assert_eq!(
            items,
            [
                (2, "https://www.minedu.sk/".to_string(), Some("SK")),
                (3, "https://www.npi.cz/aktuality".to_string(), None),
                (
                    4,
                    "https://eduscol.education.gouv.fr/feed".to_string(),
                    None
                ),
                (5, "doi 10.1002/cl2.1180".to_string(), None),
                (7, "doi 10.5281/zenodo.123".to_string(), Some("WORLD")),
            ]
        );
        let rejected: Vec<(usize, Rejection)> =
            p.rejected.iter().map(|r| (r.line, r.reason)).collect();
        assert_eq!(
            rejected,
            [
                (6, Rejection::Duplicate),
                (8, Rejection::UnsupportedScheme),
                (9, Rejection::Credentials),
                (10, Rejection::NotAnAddress),
                (12, Rejection::Duplicate),
            ]
        );
        assert_eq!(
            p.items[3].target.url().as_str(),
            "https://doi.org/10.1002/cl2.1180"
        );
    }

    #[test]
    fn limits() {
        let many: String = (0..MAX_ITEMS + 2)
            .map(|i| format!("https://s{i}.example.org/\n"))
            .collect();
        let p = parse_input(&many);
        assert_eq!(p.items.len(), MAX_ITEMS);
        assert_eq!(p.rejected.len(), 2);
        assert!(p.rejected.iter().all(|r| r.reason == Rejection::TooMany));
        assert_eq!(
            parse_input(&"x".repeat(MAX_LINE + 1)).rejected[0].reason,
            Rejection::TooLong
        );
        assert_eq!(
            parse_input("javascript:alert(1)").rejected[0].reason,
            Rejection::UnsupportedScheme
        );
    }
}
