//! What of a document goes into the index – decided by the legality principle.
//!
//! * Not indexed at all: documents that are gone, documents of disabled sources, documents whose
//!   publisher asked not to appear in search results (`noindex`), and documents ATENA may not
//!   collect in the first place.
//! * Text searched only where ATENA may keep it (`Permission::store_text`); otherwise the title and
//!   metadata are searched.
//! * Text shown as snippets only where ATENA may show it (`Permission::show_full_text`: official
//!   texts and open licences) and the publisher did not forbid snippets (`nosnippet`). Only then is
//!   the text stored in the index at all.

use std::collections::BTreeSet;

use atena_core::legality::{SourceFacts, permission};
use atena_core::{Licence, Terms};
use atena_store::IndexRow;
use jiff::Timestamp;

use crate::analysis::group_of;
use crate::schema::{AUTHORITY, COUNTRY, LANGUAGE, SOURCE};

/// A document as the index takes it.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchDocument {
    /// Document ID.
    pub id: i64,
    /// Address of the document.
    pub url: String,
    /// Source ID.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
    /// Title (may be empty).
    pub title: String,
    /// Language code.
    pub language: Option<String>,
    /// Country of the source.
    pub country: String,
    /// Authority of the source.
    pub authority: String,
    /// Licence of the source.
    pub licence: String,
    /// Publication time.
    pub published: Option<Timestamp>,
    /// The date freshness is computed from: modification, publication or the last change seen.
    pub dated: Timestamp,
    /// Description and text, when ATENA may keep them.
    pub text: Option<String>,
    /// The text may be shown as snippets.
    pub snippets: bool,
    /// The whole text may be shown in ATENA.
    pub full_text: bool,
    /// Facets: `(namespace, value)` – taxonomy labels, country, authority, source, language.
    pub facets: BTreeSet<(String, String)>,
    /// Index of the language group.
    pub(crate) group: usize,
}

/// Ranking weight of a source's authority (A1 binding official … E media).
pub fn authority_weight(authority: &str) -> f64 {
    match authority {
        "A1" => 1.5,
        "A2" => 1.4,
        "B1" => 1.3,
        "B2" => 1.25,
        "C1" => 1.1,
        "C2" => 1.05,
        "E" => 0.8,
        _ => 1.0,
    }
}

/// The document as the index may take it, or `None` when it must not be found at all.
pub fn searchable(row: &IndexRow) -> Option<SearchDocument> {
    if row.status != "active" || !row.source_enabled || row.noindex {
        return None;
    }
    let licence = Licence::parse(&row.licence);
    let facts = SourceFacts {
        country: &row.country,
        licence: &licence,
        terms: Terms::parse(&row.terms),
    };
    let allowed = permission(facts, row.tdm_reserved);
    if !allowed.crawl {
        return None;
    }
    let text = allowed
        .store_text
        .then(|| {
            [row.description.as_deref(), row.body.as_deref()]
                .into_iter()
                .flatten()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .filter(|t| !t.is_empty());
    let has_text = text.is_some();
    let mut facets: BTreeSet<(String, String)> = row
        .labels
        .iter()
        .map(|(scheme, concept)| (scheme.clone(), concept.clone()))
        .collect();
    facets.insert((COUNTRY.into(), row.country.clone()));
    facets.insert((AUTHORITY.into(), row.authority.clone()));
    facets.insert((SOURCE.into(), row.source_id.clone()));
    if let Some(language) = &row.language {
        facets.insert((LANGUAGE.into(), language.clone()));
    }
    Some(SearchDocument {
        id: row.document_id,
        url: row.url.clone(),
        source_id: row.source_id.clone(),
        source_name: row.source_name.clone(),
        title: row.title.clone().unwrap_or_default(),
        language: row.language.clone(),
        country: row.country.clone(),
        authority: row.authority.clone(),
        licence: row.licence.clone(),
        published: row.published_at,
        dated: row
            .modified_at
            .or(row.published_at)
            .unwrap_or(row.last_changed_at),
        snippets: has_text && allowed.show_full_text && !row.nosnippet,
        full_text: has_text && allowed.show_full_text && !row.noarchive,
        text,
        facets,
        group: group_of(row.language.as_deref()),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn row(licence: &str, terms: &str) -> IndexRow {
        IndexRow {
            document_id: 7,
            source_id: "SK01".into(),
            source_name: "Ministerstvo".into(),
            source_enabled: true,
            country: "SK".into(),
            authority: "A1".into(),
            licence: licence.into(),
            terms: terms.into(),
            url: "https://www.minedu.sk/zakon".into(),
            format: "html".into(),
            status: "active".into(),
            title: Some("Školský zákon".into()),
            description: Some("Zákon o výchove a vzdelávaní".into()),
            body: Some("§ 1 Predmet úpravy".into()),
            language: Some("sk".into()),
            published_at: None,
            modified_at: None,
            last_changed_at: Timestamp::UNIX_EPOCH,
            tdm_reserved: false,
            noindex: false,
            noarchive: false,
            nosnippet: false,
            labels: vec![("topic".into(), "legislation".into())],
        }
    }

    #[test]
    fn official_texts_are_searched_and_shown() {
        let d = searchable(&row("official", "allowed")).unwrap();
        assert_eq!(
            d.text.as_deref(),
            Some("Zákon o výchove a vzdelávaní\n\n§ 1 Predmet úpravy")
        );
        assert!(d.snippets && d.full_text);
        assert!(d.facets.contains(&("topic".into(), "legislation".into())));
        assert!(d.facets.contains(&("country".into(), "SK".into())));
        assert!(d.facets.contains(&("language".into(), "sk".into())));
    }

    #[test]
    fn copyrighted_texts_are_searched_but_not_shown() {
        let d = searchable(&row("copyrighted", "allowed")).unwrap();
        assert!(d.text.is_some());
        assert!(!d.snippets && !d.full_text);
    }

    #[test]
    fn metadata_only_without_verified_terms_or_with_a_reservation() {
        let d = searchable(&row("copyrighted", "unknown")).unwrap();
        assert_eq!((d.text, d.snippets), (None, false));
        let d = searchable(&row("copyrighted", "reserved")).unwrap();
        assert_eq!(d.text, None);
        let mut reserved = row("CC-BY-4.0", "allowed");
        reserved.tdm_reserved = true;
        assert_eq!(searchable(&reserved).unwrap().text, None);
    }

    #[test]
    fn publishers_directives_are_obeyed() {
        let mut r = row("official", "allowed");
        r.nosnippet = true;
        let d = searchable(&r).unwrap();
        assert!(!d.snippets && d.full_text);
        r.noarchive = true;
        assert!(!searchable(&r).unwrap().full_text);
        r.noindex = true;
        assert!(searchable(&r).is_none());
    }

    #[test]
    fn some_documents_are_never_found() {
        assert!(searchable(&row("official", "forbidden")).is_none());
        let mut gone = row("official", "allowed");
        gone.status = "gone".into();
        assert!(searchable(&gone).is_none());
        let mut disabled = row("official", "allowed");
        disabled.source_enabled = false;
        assert!(searchable(&disabled).is_none());
        let mut nowhere = row("official", "allowed");
        nowhere.country = "XX".into();
        assert!(searchable(&nowhere).is_none());
    }
}
