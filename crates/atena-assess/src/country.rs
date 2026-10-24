//! The country whose law applies to a source, guessed from its host name. Generic domains
//! (`.org`, `.com`, `.net` …) say nothing about the publisher: there is no guess, and a person
//! must name the country before the source can be used (no legal profile, no permission).

/// Country codes of the `country` scheme for host name endings, most specific first.
const ENDINGS: [(&str, &str); 36] = [
    ("europa.eu", "EU"),
    ("gov.uk", "GB-ENG"),
    ("gov.scot", "GB-SCT"),
    ("scot", "GB-SCT"),
    ("uk", "GB-ENG"),
    ("eu", "EU"),
    ("int", "WORLD"),
    ("gov", "US"),
    ("mil", "US"),
    ("edu", "US"),
    ("us", "US"),
    ("sk", "SK"),
    ("cz", "CZ"),
    ("pl", "PL"),
    ("de", "DE"),
    ("at", "AT"),
    ("hu", "HU"),
    ("fi", "FI"),
    ("ee", "EE"),
    ("nl", "NL"),
    ("fr", "FR"),
    ("se", "SE"),
    ("it", "IT"),
    ("ie", "IE"),
    ("ca", "CA"),
    ("au", "AU"),
    ("nz", "NZ"),
    ("jp", "JP"),
    ("kr", "KR"),
    ("sg", "SG"),
    ("cn", "CN"),
    ("ru", "RU"),
    ("su", "RU"),
    // .рф and .中国 in their ASCII form
    ("xn--p1ai", "RU"),
    ("xn--fiqs8s", "CN"),
    ("xn--fiqz9s", "CN"),
];

/// The country of a host name, if its ending tells.
pub fn country_of_host(host: &str) -> Option<&'static str> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    ENDINGS
        .iter()
        .find(|(ending, _)| host == *ending || host.ends_with(&format!(".{ending}")))
        .map(|(_, country)| *country)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endings() {
        for (host, country) in [
            ("www.minedu.sk", Some("SK")),
            ("eurydice.eacea.ec.europa.eu", Some("EU")),
            ("epale.ec.europa.eu", Some("EU")),
            ("www.gov.uk", Some("GB-ENG")),
            ("www.ed.gov", Some("US")),
            ("eric.ed.gov", Some("US")),
            ("www.who.int", Some("WORLD")),
            ("edu.gov.ru", Some("RU")),
            ("www.oecd.org", None),
            ("www.edutopia.org", None),
            ("127.0.0.1", None),
        ] {
            assert_eq!(country_of_host(host), country, "{host}");
        }
        // every code exists in the taxonomy and has a legal profile
        let taxonomy = atena_core::Taxonomy::bundled().unwrap();
        for (_, code) in ENDINGS {
            assert!(taxonomy.get("country", code).is_some(), "{code}");
            assert!(atena_core::legality::profile(code).is_some(), "{code}");
        }
    }
}
