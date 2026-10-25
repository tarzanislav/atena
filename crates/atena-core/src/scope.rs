//! Where a document belongs for the reader: Slovak or foreign data, and how much its source is
//! to be trusted.
//!
//! * **Region.** Slovak data are documents of Slovak sources and documents in Slovak, wherever
//!   they come from; everything else is foreign data, narrowed by language.
//! * **Trust.** The authority of the source (`A1`…`E`) in three groups a reader sees at a glance:
//!   official sources of the state, expert sources (international organisations, research,
//!   professional bodies) and other sources (associations describing themselves, individual
//!   authors, communities, media) – to be checked against a primary source.

/// Facet namespace and values of the region.
pub const REGION: &str = "region";
/// Slovak data.
pub const REGION_SK: &str = "sk";
/// Foreign data.
pub const REGION_ABROAD: &str = "abroad";
/// All regions in display order.
pub const REGIONS: [&str; 2] = [REGION_SK, REGION_ABROAD];

/// Facet namespace and values of the trust group.
pub const TRUST: &str = "trust";
/// Binding and other official sources of the state (A1, A2).
pub const TRUST_OFFICIAL: &str = "official";
/// International organisations, research and professional bodies (B1, B2, C1).
pub const TRUST_EXPERT: &str = "expert";
/// Everything else: self-described associations, authors, communities, media (C2, D, E).
pub const TRUST_OTHER: &str = "other";
/// All trust groups from the most trusted.
pub const TRUST_GROUPS: [&str; 3] = [TRUST_OFFICIAL, TRUST_EXPERT, TRUST_OTHER];

/// The region of a document from the country of its source and its language.
pub fn region_of(country: &str, language: Option<&str>) -> &'static str {
    let slovak_language = language.is_some_and(|l| {
        l.split(['-', '_'])
            .next()
            .is_some_and(|primary| primary.eq_ignore_ascii_case("sk"))
    });
    if country.eq_ignore_ascii_case("SK") || slovak_language {
        REGION_SK
    } else {
        REGION_ABROAD
    }
}

/// The trust group of a source's authority; an unknown authority is never trusted more than
/// "other".
pub fn trust_of(authority: &str) -> &'static str {
    match authority {
        "A1" | "A2" => TRUST_OFFICIAL,
        "B1" | "B2" | "C1" => TRUST_EXPERT,
        _ => TRUST_OTHER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slovak_data_are_slovak_sources_and_slovak_texts() {
        assert_eq!(region_of("SK", Some("en")), REGION_SK);
        assert_eq!(region_of("SK", None), REGION_SK);
        assert_eq!(region_of("CZ", Some("sk")), REGION_SK);
        assert_eq!(region_of("INT", Some("sk-SK")), REGION_SK);
        assert_eq!(region_of("CZ", Some("cs")), REGION_ABROAD);
        assert_eq!(region_of("INT", None), REGION_ABROAD);
        assert_eq!(region_of("US", Some("skr")), REGION_ABROAD);
    }

    #[test]
    fn authorities_fall_into_three_groups() {
        assert_eq!(trust_of("A1"), TRUST_OFFICIAL);
        assert_eq!(trust_of("A2"), TRUST_OFFICIAL);
        for a in ["B1", "B2", "C1"] {
            assert_eq!(trust_of(a), TRUST_EXPERT, "{a}");
        }
        for a in ["C2", "D", "E", "", "X"] {
            assert_eq!(trust_of(a), TRUST_OTHER, "{a}");
        }
    }
}
