//! From automatic assessments to imported sources, and from a person's decision to what the
//! legality principle lets ATENA do with them.

use atena_assess::{Assessment, Kind, Target};
use atena_core::legality::{SourceFacts, permission, profile};
use atena_core::source::{Licence, OPEN_LICENCES, Terms};
use atena_store::{AssessmentRecord, EndpointKind, NewEndpoint, NewImport, Origin, Via, hash, hex};
use url::Url;

use crate::urls::{host, normalize};

/// The ID of an imported source: `U-` and the first ten hex digits of its URL's fingerprint.
pub fn imported_id(url: &Url) -> String {
    let fingerprint = hex(&hash(normalize(url).as_str().as_bytes()));
    format!("U-{}", &fingerprint[..10])
}

/// How often an imported source is checked, by what it is.
pub fn interval_of(kind: Option<Kind>) -> i32 {
    match kind {
        Some(Kind::Feed) => 3_600,
        Some(Kind::Document) => 7 * 86_400,
        Some(Kind::Doi) => 30 * 86_400,
        _ => 86_400,
    }
}

/// The source and the automatic assessment to store for an assessment; `None` for pages about
/// people, which are not kept even as proposals.
pub fn to_import(a: &Assessment, via: Via) -> Option<(NewImport, AssessmentRecord)> {
    if a.personal_page {
        return None;
    }
    let entry = match &a.target {
        Target::Url(_) => normalize(&a.url),
        Target::Doi(_) => a.target.url(),
    };
    let interval_s = interval_of(a.kind);
    let kind = match a.kind {
        Some(Kind::Feed) => EndpointKind::Feed,
        Some(Kind::Sitemap) => EndpointKind::Sitemap,
        _ => EndpointKind::Page,
    };
    let endpoint = host(&entry).map(|h| NewEndpoint {
        url: entry.to_string(),
        host: h,
        kind,
        origin: Origin::Register,
        follow_links: a.kind == Some(Kind::Page),
        interval_s,
    });
    let source = NewImport {
        id: imported_id(&entry),
        name: a
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| host(&entry).unwrap_or_default())
            .chars()
            .take(200)
            .collect(),
        url: entry.to_string(),
        country: a.country.clone(),
        connector: a.kind.map_or("html", Kind::connector).to_string(),
        interval_s,
        via,
        endpoints: endpoint.into_iter().collect(),
    };
    let auto = AssessmentRecord {
        by_person: None,
        licence: a.proposal.licence.clone(),
        terms: a.proposal.terms.clone(),
        country: a.country.clone(),
        rights_page: a.proposal.rights.page.clone(),
        finding: a.proposal.rights.finding.clone(),
        evidence: serde_json::to_string(a).ok(),
    };
    Some((source, auto))
}

/// A person's decision, checked: the country must have a legal profile, the licence and terms
/// must be valid register values, and `official` only where the law frees official texts.
/// Returns the record to store and whether the source may be crawled.
pub fn decision(
    by_person: &str,
    country: &str,
    licence: &str,
    terms: &str,
    page: &str,
    reason: &str,
) -> Result<(AssessmentRecord, bool), String> {
    if by_person.trim().is_empty() || reason.trim().is_empty() {
        return Err("a decision needs the person who made it and a reason".into());
    }
    let profile =
        profile(country).ok_or_else(|| format!("country {country} has no legal profile"))?;
    let parsed_licence = Licence::try_from(licence.to_string())
        .map_err(|e| format!("{e}; open licences: {OPEN_LICENCES:?}"))?;
    let parsed_terms = match terms {
        "allowed" | "forbidden" | "reserved" | "unknown" => Terms::parse(terms),
        other => {
            return Err(format!(
                "unknown terms {other} (allowed, forbidden, reserved, unknown)"
            ));
        }
    };
    if parsed_licence == Licence::Official && !profile.official_texts_free {
        return Err(format!(
            "official texts of {country} are not free of copyright; use its open licence instead"
        ));
    }
    let crawl = permission(
        SourceFacts {
            country,
            licence: &parsed_licence,
            terms: parsed_terms,
        },
        false,
    )
    .crawl;
    Ok((
        AssessmentRecord {
            by_person: Some(by_person.trim().to_string()),
            licence: parsed_licence.as_str().to_string(),
            terms: parsed_terms.as_str().to_string(),
            country: Some(country.to_string()),
            rights_page: page.to_string(),
            finding: reason.trim().to_string(),
            evidence: None,
        },
        crawl,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_ignore_tracking() {
        let a = imported_id(&Url::parse("https://www.skola.sk/oznamy/?utm_source=x#top").unwrap());
        let b = imported_id(&Url::parse("https://www.skola.sk/oznamy/").unwrap());
        assert_eq!(a, b);
        assert_eq!(a.len(), 12);
        assert!(a.starts_with("U-"));
    }

    #[test]
    fn decisions_are_checked() {
        let ok = decision(
            "editor",
            "SK",
            "copyrighted",
            "allowed",
            "https://x.sk/podmienky",
            "read",
        );
        let (record, crawl) = ok.unwrap();
        assert!(crawl);
        assert_eq!(record.terms, "allowed");
        assert!(
            !decision("editor", "SK", "copyrighted", "forbidden", "p", "r")
                .unwrap()
                .1
        );
        assert!(decision("editor", "XX", "copyrighted", "allowed", "p", "r").is_err());
        assert!(decision("editor", "GB-ENG", "official", "allowed", "p", "r").is_err());
        assert!(decision("editor", "SK", "mine", "allowed", "p", "r").is_err());
        assert!(decision("editor", "SK", "copyrighted", "maybe", "p", "r").is_err());
        assert!(decision("", "SK", "copyrighted", "allowed", "p", "r").is_err());
        assert!(decision("editor", "SK", "copyrighted", "allowed", "p", " ").is_err());
    }
}
