//! From automatic assessments to imported sources, and from a person's decision to what the
//! legality principle lets ATENA do with them.
//!
//! * [`propose`] assesses items and stores them as proposals; nothing is crawled yet.
//! * [`confirm`] records a person's decision; only then may a source be crawled, and only as far
//!   as the legality principle allows.

use atena_assess::{Assessment, Assessor, Item, Kind, Target};
use atena_core::legality::{SourceFacts, permission, profile};
use atena_core::source::{Licence, OPEN_LICENCES, Terms};
use atena_store::{
    AssessmentRecord, EndpointKind, ImportOutcome, NewEndpoint, NewImport, Origin, Store,
    StoreError, Via, hash, hex,
};
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

/// Why a decision was not accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecisionError {
    /// The person or the reason is missing.
    #[error("a decision needs the person who made it and a reason")]
    Incomplete,
    /// The country has no legal profile, so nothing can be said about its law.
    #[error("country {0} has no legal profile")]
    Country(String),
    /// Not a licence of the register.
    #[error("{0}; open licences: {OPEN_LICENCES:?}")]
    Licence(String),
    /// Not a terms value of the register.
    #[error("unknown terms {0} (allowed, forbidden, reserved, unknown)")]
    Terms(String),
    /// `official` where the law does not free official texts.
    #[error("official texts of {0} are not free of copyright; use its open licence instead")]
    Official(String),
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
) -> Result<(AssessmentRecord, bool), DecisionError> {
    if by_person.trim().is_empty() || reason.trim().is_empty() {
        return Err(DecisionError::Incomplete);
    }
    let profile = profile(country).ok_or_else(|| DecisionError::Country(country.to_string()))?;
    let parsed_licence = Licence::try_from(licence.to_string())
        .map_err(|e| DecisionError::Licence(e.to_string()))?;
    let parsed_terms = match terms {
        "allowed" | "forbidden" | "reserved" | "unknown" => Terms::parse(terms),
        other => return Err(DecisionError::Terms(other.to_string())),
    };
    if parsed_licence == Licence::Official && !profile.official_texts_free {
        return Err(DecisionError::Official(country.to_string()));
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

/// One assessed item and what became of it.
#[derive(Debug, Clone)]
pub struct Proposed {
    /// The automatic assessment.
    pub assessment: Assessment,
    /// The ID of the imported source and what storing it did; `None` for a page about people,
    /// which is not kept even as a proposal.
    pub stored: Option<(String, ImportOutcome)>,
}

impl Proposed {
    /// The line to share with the editor – only for a new or newly assessed proposal: the
    /// address or DOI and the country, nothing else.
    pub fn share_line(&self) -> Option<String> {
        let (_, outcome) = self.stored.as_ref()?;
        if !matches!(outcome, ImportOutcome::New | ImportOutcome::Reassessed) {
            return None;
        }
        let a = &self.assessment;
        let target = match &a.target {
            Target::Url(u) => u.to_string(),
            Target::Doi(doi) => doi.clone(),
        };
        Some(match &a.country {
            Some(c) => format!("{target} {c}"),
            None => target,
        })
    }
}

/// Assesses `items` and stores each as a proposal (`via` says where they came from).
pub async fn propose(
    store: &Store,
    assessor: &Assessor,
    items: &[Item],
    via: Via,
    concurrency: usize,
) -> Result<Vec<Proposed>, StoreError> {
    let mut proposed = Vec::new();
    for assessment in assessor.assess_all(items, concurrency).await {
        let stored = match to_import(&assessment, via) {
            Some((source, auto)) => {
                let outcome = store.import_source(&source, &auto).await?;
                Some((source.id, outcome))
            }
            None => None,
        };
        proposed.push(Proposed { assessment, stored });
    }
    Ok(proposed)
}

/// What a person changes in a proposal when confirming it; `None` keeps the proposal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice {
    /// Licence.
    pub licence: Option<String>,
    /// Terms.
    pub terms: Option<String>,
    /// Country (upper case).
    pub country: Option<String>,
}

/// What a confirmation decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmed {
    /// Licence.
    pub licence: String,
    /// Terms.
    pub terms: String,
    /// Country.
    pub country: String,
    /// The source will be crawled.
    pub crawl: bool,
}

/// Why a confirmation failed.
#[derive(Debug, thiserror::Error)]
pub enum ConfirmError {
    /// No imported source has this ID.
    #[error("{0}: no such imported source")]
    NotFound(String),
    /// The decision is not valid.
    #[error("{0}: {1}")]
    Decision(String, DecisionError),
    /// The database failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Records a person's decision on the imported source `id`: what `choice` names, otherwise the
/// newest proposal, otherwise what is recorded now.
pub async fn confirm(
    store: &Store,
    id: &str,
    choice: &Choice,
    by_person: &str,
    reason: &str,
) -> Result<Confirmed, ConfirmError> {
    let source = store
        .imported(None)
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| ConfirmError::NotFound(id.to_string()))?;
    let proposal = source.proposal.as_ref();
    let pick = |given: &Option<String>, proposed: Option<String>, current: &str| {
        given
            .clone()
            .or(proposed)
            .unwrap_or_else(|| current.to_string())
    };
    let country = pick(
        &choice.country,
        proposal.and_then(|p| p.country.clone()),
        &source.country,
    );
    let licence = pick(
        &choice.licence,
        proposal.map(|p| p.licence.clone()),
        &source.licence,
    );
    let terms = pick(
        &choice.terms,
        proposal.map(|p| p.terms.clone()),
        &source.terms,
    );
    let page = proposal.map_or(source.url.clone(), |p| p.rights_page.clone());
    let (record, crawl) = decision(by_person, &country, &licence, &terms, &page, reason)
        .map_err(|e| ConfirmError::Decision(id.to_string(), e))?;
    store.confirm_import(id, &record, crawl).await?;
    Ok(Confirmed {
        licence: record.licence,
        terms: record.terms,
        country,
        crawl,
    })
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
