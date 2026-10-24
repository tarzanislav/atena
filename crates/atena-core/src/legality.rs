//! The legality principle – the rule above every other rule of ATENA.
//!
//! > ATENA draws only on open sources. Nothing it does may infringe the law, misuse data or breach
//! > data protection – in Slovakia, in the European Union or in any other country, America, China
//! > and Russia included. Both the law of the place where ATENA runs and the law of the country
//! > each source comes from apply, and the stricter of the two always wins. Where it is not certain
//! > that something is allowed, ATENA does not do it.
//!
//! Every decision about what may be collected, kept and shown goes through [`permission`]. Nothing
//! in ATENA may bypass it, and the register refuses a source whose country has no legal profile
//! here ([`check_source`]). A profile may be made stricter at any time; making one more permissive
//! requires a legal review.
//!
//! The profiles are conservative summaries of the law relevant to collecting public web content,
//! not legal advice:
//! * **Text and data mining** of copyrighted content is allowed by a general exception only in the
//!   EU (Directive 2019/790, Art. 4, with the publisher's opt-out), Japan (Copyright Act Art. 30-4)
//!   and Singapore (Copyright Act 2021, s. 244). The UK allows it only for non-commercial research
//!   (CDPA s. 29A); the USA relies on fair use, which is decided case by case; Australia has ruled
//!   an exception out; Canada, New Zealand, South Korea, China and Russia have none.
//! * **Official texts** (legal acts, official decisions) are free of copyright in most countries –
//!   but not under Crown copyright (UK, Canada, Australia), Irish state copyright, Singapore's
//!   government copyright, or for documents of international organisations.
//! * **Personal data**: China (PIPL) and Russia (152-FZ: processing of Russian citizens' data only
//!   in Russia) – and, to be safe, South Korea (PIPA) – make keeping texts that name people abroad
//!   unsafe, so from those countries only metadata is kept.

use crate::source::{Licence, Source, Terms};

/// The principle in English; the Slovak original is in `docs/sk/ZASADA-ZAKONNOSTI.md`.
pub const PRINCIPLE: &str = "ATENA draws only on open sources. Nothing it does may infringe the law, \
misuse data or breach data protection – in Slovakia, in the European Union or in any other country. \
Both the law of the place where ATENA runs and the law of each source's country apply; the stricter \
always wins. Where it is not certain that something is allowed, ATENA does not do it.";

/// Whom ATENA connects to – security first, beside legality (the author's rule of 24 Oct 2026).
///
/// Every server sees the address of whoever connects to it; what can be decided is whom ATENA
/// connects to and what it sends. The crawler is the anonymous visitor described here (it runs no
/// scripts, loads no images and keeps no cookies); the only other requests are made on a user's
/// explicit command, such as downloading the shared register.
pub const CONNECTIONS: &str = "ATENA sends nothing about its users to anyone. By itself it connects \
only to the sources it draws from – as an anonymous visitor that runs no scripts, loads no images \
and keeps no cookies, so no tracking or advertising service learns of it – and to the operator's \
own server, which stores no IP addresses. No third-party services stand in between: no relays, no \
analytics, no fonts or libraries from other servers. Anything else happens only on a user's \
explicit command.";

/// Country of the operator; its law applies to everything ATENA does (Slovakia: EU law).
pub const OPERATOR_COUNTRY: &str = "SK";

/// Whether copyrighted content may be mined (copied for automated analysis) under a general exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mining {
    /// A general exception that covers ATENA's purpose (subject to the publisher's opt-out).
    Allowed,
    /// Only for non-commercial research – not enough for ATENA.
    ResearchOnly,
    /// No exception (or only a case-by-case defence such as fair use).
    NotAllowed,
}

/// How strictly the source country's data protection law limits keeping texts that name people.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalData {
    /// GDPR or UK GDPR: ATENA's minimisation measures apply.
    Gdpr,
    /// Publicly available information may be processed; ATENA's minimisation measures apply.
    Moderate,
    /// Keeping such texts abroad is unsafe: only metadata is kept.
    Strict,
}

/// The legal profile of a country (or of `EU` institutions and `WORLD` organisations).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalProfile {
    /// Concept of the `country` scheme.
    pub country: &'static str,
    /// Text and data mining of copyrighted content.
    pub mining: Mining,
    /// Official texts (legal acts, official decisions) are free of copyright.
    pub official_texts_free: bool,
    /// Data protection regime.
    pub personal_data: PersonalData,
    /// Legal basis of the profile, for reviewers.
    pub basis: &'static str,
}

const fn eu(country: &'static str, official_texts_free: bool, basis: &'static str) -> LegalProfile {
    LegalProfile {
        country,
        mining: Mining::Allowed,
        official_texts_free,
        personal_data: PersonalData::Gdpr,
        basis,
    }
}

const fn other(
    country: &'static str,
    mining: Mining,
    official_texts_free: bool,
    personal_data: PersonalData,
    basis: &'static str,
) -> LegalProfile {
    LegalProfile {
        country,
        mining,
        official_texts_free,
        personal_data,
        basis,
    }
}

/// Every country of the register. A country without a profile cannot be used.
pub const PROFILES: [LegalProfile; 26] = [
    eu(
        "SK",
        true,
        "Directive 2019/790 Art. 4; Copyright Act 185/2015 § 5 (official works); GDPR",
    ),
    eu(
        "CZ",
        true,
        "Directive 2019/790 Art. 4; Copyright Act 121/2000 § 3 (official works); GDPR",
    ),
    eu(
        "PL",
        true,
        "Directive 2019/790 Art. 4; Copyright Act Art. 4 (official documents); GDPR",
    ),
    eu(
        "DE",
        true,
        "§ 44b UrhG (opt-out); § 5 UrhG (official works); GDPR",
    ),
    eu(
        "AT",
        true,
        "Directive 2019/790 Art. 4 (UrhG); § 7 UrhG (free works); GDPR",
    ),
    eu(
        "HU",
        true,
        "Directive 2019/790 Art. 4; Act LXXVI of 1999 § 1(4) (legal acts); GDPR",
    ),
    eu(
        "FI",
        true,
        "Directive 2019/790 Art. 4; Copyright Act § 9 (official documents); GDPR",
    ),
    eu(
        "EE",
        true,
        "Directive 2019/790 Art. 4; Copyright Act § 5 (official texts); GDPR",
    ),
    eu(
        "NL",
        true,
        "Directive 2019/790 Art. 4; Auteurswet Art. 11 (legal texts); GDPR",
    ),
    eu(
        "IE",
        false,
        "Directive 2019/790 Art. 4; state copyright (CRRA 2000 s. 191 ff.) – official texts need an open licence; GDPR",
    ),
    eu(
        "FR",
        true,
        "Directive 2019/790 Art. 4; official acts are not protected; GDPR",
    ),
    eu(
        "SE",
        true,
        "Directive 2019/790 Art. 4; Copyright Act § 9 (official documents); GDPR",
    ),
    eu(
        "IT",
        true,
        "Directive 2019/790 Art. 4; Copyright Act Art. 5 (official acts); GDPR",
    ),
    eu(
        "EU",
        true,
        "Directive 2019/790 Art. 4; EU legal acts free to reuse (Decision 2011/833/EU); GDPR",
    ),
    eu(
        "WORLD",
        false,
        "operator law (EU); documents of international organisations are copyrighted unless openly licensed; GDPR",
    ),
    other(
        "GB-ENG",
        Mining::ResearchOnly,
        false,
        PersonalData::Gdpr,
        "CDPA s. 29A (non-commercial research only); Crown copyright (OGL needed); UK GDPR",
    ),
    other(
        "GB-SCT",
        Mining::ResearchOnly,
        false,
        PersonalData::Gdpr,
        "CDPA s. 29A (non-commercial research only); Crown copyright (OGL needed); UK GDPR",
    ),
    other(
        "US",
        Mining::NotAllowed,
        true,
        PersonalData::Moderate,
        "fair use is case by case; 17 U.S.C. § 105 frees federal works only (mark only federal sources official); state privacy laws",
    ),
    other(
        "CA",
        Mining::NotAllowed,
        false,
        PersonalData::Moderate,
        "no TDM exception; Crown copyright (open government licence needed); PIPEDA",
    ),
    other(
        "AU",
        Mining::NotAllowed,
        false,
        PersonalData::Moderate,
        "no TDM exception (ruled out 2025); Crown copyright; Privacy Act 1988",
    ),
    other(
        "NZ",
        Mining::NotAllowed,
        true,
        PersonalData::Moderate,
        "no TDM exception; Copyright Act 1994 s. 27 (official texts); Privacy Act 2020",
    ),
    other(
        "JP",
        Mining::Allowed,
        true,
        PersonalData::Moderate,
        "Copyright Act Art. 30-4 (data analysis) and Art. 13 (official texts); APPI",
    ),
    other(
        "KR",
        Mining::NotAllowed,
        true,
        PersonalData::Strict,
        "no TDM exception; Copyright Act Art. 7 (official texts); PIPA – treated as strict",
    ),
    other(
        "SG",
        Mining::Allowed,
        false,
        PersonalData::Moderate,
        "Copyright Act 2021 s. 244 (computational data analysis, lawful access); government works copyrighted; PDPA",
    ),
    other(
        "CN",
        Mining::NotAllowed,
        true,
        PersonalData::Strict,
        "no TDM exception; Copyright Law Art. 5 (official documents); PIPL (incl. cross-border rules)",
    ),
    other(
        "RU",
        Mining::NotAllowed,
        true,
        PersonalData::Strict,
        "no TDM exception; Civil Code Art. 1259(6) (official documents); 152-FZ Art. 18(5) (processing only in Russia)",
    ),
];

/// The profile of a country, if ATENA has one.
pub fn profile(country: &str) -> Option<&'static LegalProfile> {
    PROFILES.iter().find(|p| p.country == country)
}

/// What a source is, as far as the law is concerned.
#[derive(Debug, Clone, Copy)]
pub struct SourceFacts<'a> {
    /// Concept of the `country` scheme.
    pub country: &'a str,
    /// Licence of the content.
    pub licence: &'a Licence,
    /// Terms of use on automated access.
    pub terms: Terms,
}

impl<'a> From<&'a Source> for SourceFacts<'a> {
    fn from(source: &'a Source) -> Self {
        Self {
            country: &source.country,
            licence: &source.licence,
            terms: source.terms,
        }
    }
}

/// What ATENA may do with the content of one source (and one document).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    /// Fetch pages and files at all (robots.txt is obeyed in addition).
    pub crawl: bool,
    /// Keep the text (for change detection and classification); otherwise metadata only.
    pub store_text: bool,
    /// Show the full text to users.
    pub show_full_text: bool,
    /// Offer a full translation.
    pub translate: bool,
    /// Why something is not allowed, for logs and reviewers.
    pub reasons: Vec<&'static str>,
}

impl Permission {
    fn nothing(reason: &'static str) -> Self {
        Self {
            crawl: false,
            store_text: false,
            show_full_text: false,
            translate: false,
            reasons: vec![reason],
        }
    }
}

/// Decides what ATENA may do. `tdm_reserved`: the publisher reserved text and data mining for this
/// document.
///
/// Original files are never kept, whatever the licence (the author's decision of 24 Oct 2026): a
/// file may carry harmful content or personal data that cannot be removed from it reliably. The
/// cleaned text and a fingerprint of the original are kept; the original stays with its publisher.
pub fn permission(source: SourceFacts<'_>, tdm_reserved: bool) -> Permission {
    let here = profile(OPERATOR_COUNTRY).expect("the operator's country has a profile");
    let Some(there) = profile(source.country) else {
        return Permission::nothing("no legal profile for the source's country");
    };
    if source.terms == Terms::Forbidden {
        return Permission::nothing("the terms of use forbid automated access");
    }
    let mut reasons = Vec::new();
    // a reservation in the terms of use counts like a machine-readable one
    let reserved = tdm_reserved || source.terms == Terms::Reserved;
    // content that may be copied and shown by anyone: openly licensed, or official texts free of
    // copyright under both laws
    let reusable = match source.licence {
        Licence::Open(_) => true,
        Licence::Official => here.official_texts_free && there.official_texts_free,
        Licence::Copyrighted => false,
    };
    // copyrighted content may be mined only where both laws have an exception that covers ATENA,
    // and only when the terms of use were verified not to reserve it
    let minable = here.mining == Mining::Allowed
        && there.mining == Mining::Allowed
        && source.terms == Terms::Allowed;
    if !reusable && !minable && !(reserved && there.mining == Mining::Allowed) {
        reasons.push(match there.mining {
            Mining::Allowed => "copyrighted content: the terms of use are not verified",
            Mining::ResearchOnly => "copyrighted content: the source country allows mining only for non-commercial research",
            Mining::NotAllowed => "copyrighted content: the source country has no text and data mining exception",
        });
    }
    if reserved {
        reasons.push("the publisher reserved text and data mining");
    }
    let strict_privacy = there.personal_data == PersonalData::Strict;
    if strict_privacy {
        reasons
            .push("the source country's data protection law does not allow keeping texts abroad");
    }
    let store_text = (reusable || minable) && !reserved && !strict_privacy;
    let show_full_text = store_text && reusable;
    Permission {
        crawl: true,
        store_text,
        show_full_text,
        translate: show_full_text,
        reasons,
    }
}

/// `YYYY-MM-DD` with a plausible month and day.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [y, m, d] => matches!(
            (number(y, 4), number(m, 2), number(d, 2)),
            (Some(2000..), Some(1..=12), Some(1..=31))
        ),
        _ => false,
    }
}

/// Checks a register entry against the principle: its country must have a profile, only countries
/// whose law frees official texts may have sources marked `official`, and any licence or terms
/// other than the defaults must be backed by a recorded rights review.
pub fn check_source(source: &Source) -> Result<(), String> {
    let Some(p) = profile(&source.country) else {
        return Err(format!(
            "source {}: country {} has no legal profile in atena-core (legality.rs)",
            source.id, source.country
        ));
    };
    if source.licence == Licence::Official && !p.official_texts_free {
        return Err(format!(
            "source {}: official texts of {} are not free of copyright; use its open licence instead",
            source.id, source.country
        ));
    }
    let defaults = source.licence == Licence::Copyrighted && source.terms == Terms::Unknown;
    match &source.rights {
        None if defaults => Ok(()),
        None => Err(format!(
            "source {}: licence {} and terms {} need a rights review (rights: checked, page, finding)",
            source.id,
            source.licence.as_str(),
            source.terms.as_str()
        )),
        Some(r) => {
            let web = |u: &str| u.starts_with("https://") || u.starts_with("http://");
            if !is_date(&r.checked) || !web(&r.page) || r.finding.trim().is_empty() {
                return Err(format!(
                    "source {}: the rights review needs a date (YYYY-MM-DD), the http(s) page \
                     that was read and a finding",
                    source.id
                ));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{BUNDLED_SOURCES, load_sources};
    use crate::taxonomy::Taxonomy;

    fn facts<'a>(country: &'a str, licence: &'a Licence, terms: Terms) -> SourceFacts<'a> {
        SourceFacts {
            country,
            licence,
            terms,
        }
    }

    const ALL_TERMS: [Terms; 4] = [
        Terms::Allowed,
        Terms::Forbidden,
        Terms::Reserved,
        Terms::Unknown,
    ];

    fn all_licences() -> [Licence; 3] {
        [
            Licence::Official,
            Licence::Copyrighted,
            Licence::Open("CC-BY-4.0".into()),
        ]
    }

    #[test]
    fn invariants_hold_for_every_country_and_combination() {
        for p in &PROFILES {
            for licence in all_licences() {
                for terms in ALL_TERMS {
                    for reserved in [false, true] {
                        let perm = permission(facts(p.country, &licence, terms), reserved);
                        let case =
                            format!("{} {licence:?} {terms:?} reserved={reserved}", p.country);
                        // forbidden terms: nothing at all
                        if terms == Terms::Forbidden {
                            assert!(!perm.crawl && !perm.store_text, "{case}");
                        }
                        // a reservation is always respected, wherever it is expressed
                        if reserved || terms == Terms::Reserved {
                            assert!(!perm.store_text && !perm.show_full_text, "{case}");
                        }
                        // strict data protection: never texts
                        if p.personal_data == PersonalData::Strict {
                            assert!(!perm.store_text, "{case}");
                        }
                        // copyrighted content is never shown in full
                        if licence == Licence::Copyrighted {
                            assert!(!perm.show_full_text, "{case}");
                            // and kept as text only with verified terms where both laws allow mining
                            if perm.store_text {
                                assert!(
                                    terms == Terms::Allowed && p.mining == Mining::Allowed,
                                    "{case}"
                                );
                            }
                        }
                        // showing requires keeping
                        assert!(!perm.show_full_text || perm.store_text, "{case}");
                        // whatever is not allowed is explained
                        assert!(perm.store_text || !perm.reasons.is_empty(), "{case}");
                    }
                }
            }
        }
    }

    #[test]
    fn typical_cases() {
        let copyrighted = Licence::Copyrighted;
        let official = Licence::Official;
        let open = Licence::Open("CC-BY-4.0".into());
        // a Slovak ministry page, terms not verified yet: metadata only
        let p = permission(facts("SK", &copyrighted, Terms::Unknown), false);
        assert!(p.crawl && !p.store_text);
        // the same with verified terms: kept for change detection, never shown in full
        let p = permission(facts("SK", &copyrighted, Terms::Allowed), false);
        assert!(p.store_text && !p.show_full_text);
        // Slov-Lex: official texts, shown in full
        let p = permission(facts("SK", &official, Terms::Unknown), false);
        assert!(p.store_text && p.show_full_text);
        // UK and US copyrighted content: metadata only even with verified terms
        assert!(!permission(facts("GB-ENG", &copyrighted, Terms::Allowed), false).store_text);
        assert!(!permission(facts("US", &copyrighted, Terms::Allowed), false).store_text);
        // UK government content needs its open licence (OGL), "official" is not enough
        assert!(!permission(facts("GB-ENG", &official, Terms::Allowed), false).store_text);
        assert!(
            permission(
                facts(
                    "GB-ENG",
                    &Licence::Open("OGL-UK-3.0".into()),
                    Terms::Unknown
                ),
                false
            )
            .show_full_text
        );
        // Japan has a mining exception
        assert!(permission(facts("JP", &copyrighted, Terms::Allowed), false).store_text);
        // China and Russia: metadata only, even for official and open texts
        for country in ["CN", "RU"] {
            for licence in [&official, &open, &copyrighted] {
                let p = permission(facts(country, licence, Terms::Allowed), false);
                assert!(
                    p.crawl && !p.store_text && !p.show_full_text,
                    "{country} {licence:?}"
                );
            }
        }
        // terms that reserve mining: crawled, metadata only – even for an open licence
        let p = permission(facts("SK", &open, Terms::Reserved), false);
        assert!(p.crawl && !p.store_text);
        // a country without a profile: nothing
        assert!(!permission(facts("XX", &open, Terms::Allowed), false).crawl);
    }

    #[test]
    fn non_default_rights_need_a_recorded_review() {
        let t = Taxonomy::bundled().unwrap();
        let entry = |extra: &str| {
            format!(
                "sources:\n- id: X1\n  name: X\n  url: https://x.example/\n  country: SK\n  \
                 languages: [sk]\n  authority: A2\n  priority: P0\n  connector: html\n  \
                 interval_s: 3600\n  review: new\n{extra}"
            )
        };
        // defaults need nothing
        assert!(load_sources(&entry(""), &t).is_ok());
        // a licence or terms without a review are refused
        assert!(load_sources(&entry("  terms: allowed\n"), &t).is_err());
        assert!(load_sources(&entry("  licence: CC-BY-4.0\n"), &t).is_err());
        let review = |date: &str, page: &str| {
            format!(
                "  terms: allowed\n  rights:\n    checked: {date}\n    page: {page}\n    \
                 finding: No terms restrict automated access.\n"
            )
        };
        let ok =
            load_sources(&entry(&review("2026-10-24", "https://x.example/terms")), &t).unwrap();
        assert_eq!(ok[0].rights.as_ref().unwrap().checked, "2026-10-24");
        assert!(
            load_sources(&entry(&review("24.10.2026", "https://x.example/terms")), &t).is_err()
        );
        assert!(
            load_sources(&entry(&review("2026-13-01", "https://x.example/terms")), &t).is_err()
        );
        assert!(load_sources(&entry(&review("2026-10-24", "x.example/terms")), &t).is_err());
        // "reserved" is a valid value
        let reserved =
            entry(&review("2026-10-24", "https://x.example/terms").replace("allowed", "reserved"));
        assert_eq!(
            load_sources(&reserved, &t).unwrap()[0].terms,
            Terms::Reserved
        );
    }

    #[test]
    fn every_register_country_has_a_profile_and_official_marks_are_valid() {
        let sources = load_sources(BUNDLED_SOURCES, &Taxonomy::bundled().unwrap()).unwrap();
        for s in &sources {
            check_source(s).unwrap();
        }
        let taxonomy = Taxonomy::bundled().unwrap();
        for c in taxonomy.concepts_of("country") {
            assert!(
                profile(&c.id).is_some(),
                "country {} has no legal profile",
                c.id
            );
        }
        assert!(profile(OPERATOR_COUNTRY).is_some());
    }
}
