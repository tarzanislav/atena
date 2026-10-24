//! Recognising licences from their official addresses and from how pages name them.

use atena_core::source::OPEN_LICENCES;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::phrases::{find, normalize};

/// A licence a page points to or names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceHint {
    /// The licence's identifier: an SPDX id when ATENA knows it, otherwise a short description
    /// (`CC-BY-NC-4.0`, `CC-BY-3.0` …).
    pub id: String,
    /// `true` when the licence is one of the open licences ATENA accepts ([`OPEN_LICENCES`]).
    pub open: bool,
    /// Where it was found: the licence address or the phrase on the page.
    pub evidence: String,
}

impl LicenceHint {
    fn new(id: String, evidence: String) -> Self {
        Self {
            open: OPEN_LICENCES.contains(&id.as_str()),
            id,
            evidence,
        }
    }
}

/// The licence an address identifies: Creative Commons, Open Government Licence, Licence Ouverte
/// and the Open Data Commons licences.
pub fn licence_of_url(url: &Url) -> Option<LicenceHint> {
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let path = url.path().to_ascii_lowercase();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let id = match (host, segments.as_slice()) {
        ("creativecommons.org", ["publicdomain", "zero", "1.0", ..]) => "CC0-1.0".to_string(),
        ("creativecommons.org", ["licenses", code, version, rest @ ..]) => {
            let igo = rest.first() == Some(&"igo");
            format!(
                "CC-{}-{version}{}",
                code.to_ascii_uppercase(),
                if igo { "-IGO" } else { "" }
            )
        }
        ("nationalarchives.gov.uk", _) if path.contains("open-government-licence/version/3") => {
            "OGL-UK-3.0".to_string()
        }
        ("etalab.gouv.fr", _) if path.contains("licence-ouverte") => "etalab-2.0".to_string(),
        ("github.com", ["etalab", "licence-ouverte", ..]) => "etalab-2.0".to_string(),
        ("opendatacommons.org", ["licenses", "by", ..]) => "ODC-By-1.0".to_string(),
        ("opendatacommons.org", ["licenses", "odbl", ..]) => "ODbL-1.0".to_string(),
        ("opendatacommons.org", ["licenses", "pddl", ..]) => "PDDL-1.0".to_string(),
        _ => return None,
    };
    Some(LicenceHint::new(id, url.to_string()))
}

/// How pages name licences in text, normalised, with the identifier they stand for.
const NAMED: [(&str, &str); 16] = [
    ("cc by-nc", "CC-BY-NC"),
    ("cc-by-nc", "CC-BY-NC"),
    ("creative commons attribution-noncommercial", "CC-BY-NC"),
    ("cc by-nd", "CC-BY-ND"),
    ("creative commons attribution-noderivatives", "CC-BY-ND"),
    ("cc by-sa 4.0", "CC-BY-SA-4.0"),
    ("cc-by-sa 4.0", "CC-BY-SA-4.0"),
    (
        "creative commons attribution-sharealike 4.0",
        "CC-BY-SA-4.0",
    ),
    ("cc by 4.0", "CC-BY-4.0"),
    ("cc-by 4.0", "CC-BY-4.0"),
    ("cc-by-4.0", "CC-BY-4.0"),
    ("creative commons attribution 4.0", "CC-BY-4.0"),
    ("cc0", "CC0-1.0"),
    ("open government licence", "OGL-UK-3.0"),
    ("licence ouverte", "etalab-2.0"),
    ("etalab-2.0", "etalab-2.0"),
];

/// Licences a text names. A non-commercial or no-derivatives variant hides the open licence it
/// contains ("CC BY-NC 4.0" is not "CC BY 4.0").
pub fn licences_in_text(text: &str) -> Vec<LicenceHint> {
    let n = normalize(text);
    let mut hints: Vec<LicenceHint> = Vec::new();
    let mut taken: Vec<(usize, usize)> = Vec::new();
    for (phrase, id) in NAMED {
        let mut from = 0;
        while let Some(at) = find(&n[from..], phrase).map(|i| i + from) {
            let end = at + phrase.len();
            from = end;
            // a longer, more specific name already covers this place
            if taken.iter().any(|&(s, e)| at >= s && at < e) {
                continue;
            }
            taken.push((at, end));
            if !hints.iter().any(|h| h.id == id) {
                hints.push(LicenceHint::new(id.to_string(), phrase.to_string()));
            }
        }
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(url: &str) -> Option<(String, bool)> {
        licence_of_url(&Url::parse(url).unwrap()).map(|h| (h.id, h.open))
    }

    #[test]
    fn licence_addresses() {
        assert_eq!(
            id("https://creativecommons.org/licenses/by/4.0/deed.sk"),
            Some(("CC-BY-4.0".into(), true))
        );
        assert_eq!(
            id("http://creativecommons.org/licenses/by-sa/4.0/"),
            Some(("CC-BY-SA-4.0".into(), true))
        );
        assert_eq!(
            id("https://creativecommons.org/licenses/by-sa/3.0/igo/"),
            Some(("CC-BY-SA-3.0-IGO".into(), true))
        );
        assert_eq!(
            id("https://creativecommons.org/publicdomain/zero/1.0/"),
            Some(("CC0-1.0".into(), true))
        );
        assert_eq!(
            id("https://creativecommons.org/licenses/by-nc-nd/4.0/"),
            Some(("CC-BY-NC-ND-4.0".into(), false))
        );
        assert_eq!(
            id("https://creativecommons.org/licenses/by/3.0/"),
            Some(("CC-BY-3.0".into(), false))
        );
        assert_eq!(
            id("https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/"),
            Some(("OGL-UK-3.0".into(), true))
        );
        assert_eq!(
            id("https://github.com/etalab/licence-ouverte/blob/master/LO.md"),
            Some(("etalab-2.0".into(), true))
        );
        assert_eq!(id("https://creativecommons.org/about/"), None);
        assert_eq!(id("https://example.org/licenses/by/4.0/"), None);
    }

    #[test]
    fn licences_named_in_text() {
        let ids = |t: &str| {
            licences_in_text(t)
                .into_iter()
                .map(|h| (h.id, h.open))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids("Obsah je dostupný pod licenciou CC BY 4.0."),
            [("CC-BY-4.0".into(), true)]
        );
        assert_eq!(
            ids("Licensed under CC BY-NC 4.0"),
            [("CC-BY-NC".into(), false)]
        );
        assert_eq!(
            ids("Texts: CC BY-SA 4.0, audiovisual material: CC BY-NC-ND 4.0"),
            [("CC-BY-NC".into(), false), ("CC-BY-SA-4.0".into(), true)]
        );
        assert!(ids("Accessibility statement").is_empty());
    }
}
