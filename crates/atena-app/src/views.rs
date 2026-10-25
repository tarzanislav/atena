//! What the user interface receives and sends: plain data in camelCase JSON. Codes (outcomes,
//! reasons, states) are translated by the interface; texts that are data – titles, findings,
//! addresses – are passed as they are and shown as text, never as markup.

use std::collections::BTreeMap;

use atena_assess::{Kind, Problem, Reason, Rejected};
use atena_core::legality::{SourceFacts, permission};
use atena_core::scope::{region_of, trust_of};
use atena_core::{Licence, Restriction, Taxonomy, Terms};
use atena_ingest::import::Proposed;
use atena_search::{Hit, Results, Snippet, Tally};
use atena_store::{ImportOutcome, ImportState, ImportedSource, SourceSummary, Via};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

/// What the legality principle allows for a source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Allowed {
    /// Fetch it at all.
    pub crawl: bool,
    /// Keep its texts.
    pub store_text: bool,
    /// Show its texts in full.
    pub show_full_text: bool,
    /// Why not more.
    pub restrictions: Vec<Restriction>,
}

impl Allowed {
    /// What the principle allows for a source of `country` with `licence` and `terms`.
    pub fn of(country: &str, licence: &str, terms: &str) -> Self {
        let p = permission(
            SourceFacts {
                country,
                licence: &Licence::parse(licence),
                terms: Terms::parse(terms),
            },
            false,
        );
        Self {
            crawl: p.crawl,
            store_text: p.store_text,
            show_full_text: p.show_full_text,
            restrictions: p.reasons,
        }
    }
}

/// The overview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewView {
    /// Sources: the register and confirmed imports.
    pub sources: i64,
    /// Sources that are crawled.
    pub sources_crawled: i64,
    /// Proposals waiting for a decision.
    pub proposals: i64,
    /// Documents still published.
    pub documents: i64,
    /// Their versions.
    pub versions: i64,
    /// Documents that can be found.
    pub searchable: u64,
    /// Addresses that are checked.
    pub endpoints: i64,
    /// Addresses due now.
    pub endpoints_due: i64,
    /// The latest check.
    pub last_check: Option<Timestamp>,
    /// Checks in the last 24 hours.
    pub checks_last_day: i64,
    /// Lists of proposals waiting to be handed over.
    pub waiting_to_share: i64,
    /// The crawler is running.
    pub crawling: bool,
}

/// The state of an imported source, as a code.
pub fn state_code(state: ImportState) -> &'static str {
    state.as_str()
}

/// A source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    /// ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// Entry address.
    pub url: String,
    /// Country.
    pub country: String,
    /// Authority level.
    pub authority: String,
    /// Trust group of the authority: `official`, `expert` or `other`.
    pub trust: String,
    /// Licence.
    pub licence: String,
    /// Terms of use.
    pub terms: String,
    /// From the register (otherwise imported).
    pub register: bool,
    /// `proposed`, `confirmed`, `rejected` or `merged`.
    pub state: &'static str,
    /// Crawled.
    pub enabled: bool,
    /// Documents still published.
    pub documents: i64,
    /// The latest check.
    pub last_check: Option<Timestamp>,
    /// What the legality principle allows.
    pub allowed: Allowed,
}

impl From<SourceSummary> for SourceView {
    fn from(s: SourceSummary) -> Self {
        let allowed = Allowed::of(&s.country, &s.licence, &s.terms);
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            country: s.country,
            trust: trust_of(&s.authority).into(),
            authority: s.authority,
            licence: s.licence,
            terms: s.terms,
            register: s.register,
            state: state_code(s.state),
            enabled: s.enabled,
            documents: s.documents,
            last_check: s.last_check,
            allowed,
        }
    }
}

/// The parts of a stored automatic assessment the interface shows.
#[derive(Debug, Clone, Default, Deserialize)]
struct StoredEvidence {
    #[serde(default)]
    kind: Option<Kind>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    checked: String,
    #[serde(default)]
    problem: Option<Problem>,
    #[serde(default)]
    proposal: Option<StoredProposal>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StoredProposal {
    #[serde(default)]
    reasons: Vec<Reason>,
}

/// An automatic proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDetail {
    /// Proposed licence.
    pub licence: String,
    /// Proposed terms.
    pub terms: String,
    /// Country whose law applies, if known.
    pub country: Option<String>,
    /// The page the rights record refers to.
    pub rights_page: String,
    /// The rights record (what was found, with quotes) – data, in English.
    pub finding: String,
    /// Why the proposal is what it is.
    pub reasons: Vec<Reason>,
    /// Why the address could not be read.
    pub problem: Option<Problem>,
    /// What it turned out to be.
    pub kind: Option<Kind>,
    /// Its title.
    pub title: Option<String>,
    /// Day of the assessment.
    pub checked: String,
    /// What confirming the proposal would allow.
    pub allowed: Allowed,
}

/// An imported source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalView {
    /// ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// Address.
    pub url: String,
    /// `proposed`, `confirmed`, `rejected` or `merged`.
    pub state: &'static str,
    /// Proposed by a user of another installation (read from a list).
    pub shared: bool,
    /// Recorded country (empty when unknown).
    pub country: String,
    /// Recorded licence.
    pub licence: String,
    /// Recorded terms.
    pub terms: String,
    /// Crawled.
    pub enabled: bool,
    /// The newest automatic proposal.
    pub proposal: Option<ProposalDetail>,
}

impl From<ImportedSource> for ProposalView {
    fn from(s: ImportedSource) -> Self {
        let proposal = s.proposal.map(|p| {
            let evidence: StoredEvidence = p
                .evidence
                .as_deref()
                .and_then(|e| serde_json::from_str(e).ok())
                .unwrap_or_default();
            let country = p.country.clone().unwrap_or_default();
            ProposalDetail {
                allowed: Allowed::of(&country, &p.licence, &p.terms),
                licence: p.licence,
                terms: p.terms,
                country: p.country,
                rights_page: p.rights_page,
                finding: p.finding,
                reasons: evidence.proposal.map(|p| p.reasons).unwrap_or_default(),
                problem: evidence.problem,
                kind: evidence.kind,
                title: evidence.title,
                checked: evidence.checked,
            }
        });
        Self {
            id: s.id,
            name: s.name,
            url: s.url,
            state: state_code(s.state),
            shared: s.via == Via::Shared,
            country: s.country,
            licence: s.licence,
            terms: s.terms,
            enabled: s.enabled,
            proposal,
        }
    }
}

/// What became of one item of an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedView {
    /// Line of the input.
    pub line: usize,
    /// The address read.
    pub url: String,
    /// ID of the imported source; `None` for a page about people, which is not kept.
    pub id: Option<String>,
    /// `new`, `reassessed`, `already-proposed`, `already-confirmed`, `already-rejected`,
    /// `already-merged`, `in-register`, `blocked` or `personal-page`.
    pub outcome: &'static str,
    /// The register ID, for `in-register`.
    pub register_id: Option<String>,
    /// Its title.
    pub title: Option<String>,
    /// Country whose law applies, if known.
    pub country: Option<String>,
    /// Proposed licence.
    pub licence: String,
    /// Proposed terms.
    pub terms: String,
    /// Why.
    pub reasons: Vec<Reason>,
    /// Why it could not be read.
    pub problem: Option<Problem>,
}

fn outcome_code(outcome: &ImportOutcome) -> &'static str {
    match outcome {
        ImportOutcome::New => "new",
        ImportOutcome::Reassessed => "reassessed",
        ImportOutcome::AlreadyDecided(ImportState::Proposed) => "already-proposed",
        ImportOutcome::AlreadyDecided(ImportState::Confirmed) => "already-confirmed",
        ImportOutcome::AlreadyDecided(ImportState::Rejected) => "already-rejected",
        ImportOutcome::AlreadyDecided(ImportState::Merged) => "already-merged",
        ImportOutcome::InRegister(_) => "in-register",
        ImportOutcome::Blocked => "blocked",
    }
}

impl From<&Proposed> for ProposedView {
    fn from(p: &Proposed) -> Self {
        let a = &p.assessment;
        let (id, outcome, register_id) = match &p.stored {
            Some((id, outcome)) => (
                Some(id.clone()),
                outcome_code(outcome),
                match outcome {
                    ImportOutcome::InRegister(r) => Some(r.clone()),
                    _ => None,
                },
            ),
            None => (None, "personal-page", None),
        };
        Self {
            line: a.line,
            url: a.url.to_string(),
            id,
            outcome,
            register_id,
            title: a.title.clone(),
            country: a.country.clone(),
            licence: a.proposal.licence.clone(),
            terms: a.proposal.terms.clone(),
            reasons: a.proposal.reasons.clone(),
            problem: a.problem,
        }
    }
}

/// The result of an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportView {
    /// The items read.
    pub proposed: Vec<ProposedView>,
    /// Lines that were not accepted.
    pub rejected: Vec<Rejected>,
    /// Addresses kept for handing to the editor.
    pub kept_for_sharing: usize,
}

/// The result of reading a list from another installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedView {
    /// `read`, `read-before` or `not-a-list`.
    pub status: &'static str,
    /// ID of the list.
    pub list: Option<String>,
    /// The items read.
    pub proposed: Vec<ProposedView>,
    /// Lines that were not accepted.
    pub rejected: Vec<Rejected>,
}

/// The list waiting to be handed to the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingView {
    /// The list as text: addresses and country codes only.
    pub list: String,
    /// Number of addresses.
    pub addresses: usize,
    /// What to mark handed over afterwards.
    pub entries: Vec<i64>,
}

/// What a person changes in a proposal when confirming it, as the interface sends it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChoiceInput {
    /// Licence; `None` keeps the proposal.
    pub licence: Option<String>,
    /// Terms; `None` keeps the proposal.
    pub terms: Option<String>,
    /// Country; `None` keeps the proposal.
    pub country: Option<String>,
}

impl From<ChoiceInput> for atena_ingest::import::Choice {
    fn from(c: ChoiceInput) -> Self {
        let given = |v: Option<String>| v.filter(|v| !v.trim().is_empty());
        Self {
            licence: given(c.licence),
            terms: given(c.terms),
            country: given(c.country),
        }
    }
}

/// What a person may choose in a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionOptions {
    /// Licences: `copyrighted`, `official` and the accepted open licences.
    pub licences: Vec<&'static str>,
    /// Terms of use.
    pub terms: Vec<&'static str>,
    /// Countries with a legal profile.
    pub countries: Vec<&'static str>,
}

impl DecisionOptions {
    /// The options of the register.
    pub fn new() -> Self {
        let mut licences = vec!["copyrighted", "official"];
        licences.extend(atena_core::source::OPEN_LICENCES);
        Self {
            licences,
            terms: vec!["allowed", "unknown", "reserved", "forbidden"],
            countries: atena_core::legality::PROFILES
                .iter()
                .map(|p| p.country)
                .collect(),
        }
    }
}

impl Default for DecisionOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// A decision taken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmedView {
    /// Licence.
    pub licence: String,
    /// Terms.
    pub terms: String,
    /// Country.
    pub country: String,
    /// The source will be crawled.
    pub crawl: bool,
}

/// What a synchronisation with the shared register changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncView {
    /// Sources in the register.
    pub sources: u64,
    /// Sources that left it.
    pub removed: u64,
    /// Imports it took over.
    pub merged: u64,
    /// Addresses added.
    pub endpoints_added: u64,
    /// Addresses no longer checked.
    pub endpoints_disabled: u64,
    /// Documents removed because their source may no longer be crawled.
    pub purged: u64,
    /// Versions reduced to metadata.
    pub restricted: u64,
}

impl From<atena_ingest::register::SyncSummary> for SyncView {
    fn from(s: atena_ingest::register::SyncSummary) -> Self {
        Self {
            sources: s.report.sources,
            removed: s.report.removed,
            merged: s.report.merged,
            endpoints_added: s.report.endpoints_added,
            endpoints_disabled: s.report.endpoints_disabled,
            purged: s.purged,
            restricted: s.restricted,
        }
    }
}

/// What an erasure removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErasureView {
    /// Documents.
    pub documents: u64,
    /// Versions.
    pub versions: u64,
}

/// A search, as the interface sends it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchQuery {
    /// The words.
    pub text: String,
    /// Language of the words; `None`: any.
    pub language: Option<String>,
    /// Filters: `scheme/concept`, `language/xx`, `source/ID`, `country/CC`.
    pub filters: Vec<String>,
    /// Page, from 1.
    pub page: usize,
}

/// A piece of a snippet; `marked` pieces match the words searched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    /// The text.
    pub text: String,
    /// It matches.
    pub marked: bool,
}

/// A snippet as pieces, so the interface never needs to cut text (its strings count differently).
pub fn segments(snippet: &Snippet) -> Vec<Segment> {
    let text = &snippet.text;
    let mut out = Vec::new();
    let mut at = 0;
    let mut push = |piece: &str, marked: bool| {
        if !piece.is_empty() {
            out.push(Segment {
                text: piece.to_string(),
                marked,
            });
        }
    };
    for range in &snippet.highlighted {
        let valid = range.start >= at
            && range.end <= text.len()
            && text.is_char_boundary(range.start)
            && text.is_char_boundary(range.end);
        if !valid {
            continue;
        }
        push(&text[at..range.start], false);
        push(&text[range.clone()], true);
        at = range.end;
    }
    push(&text[at..], false);
    out
}

/// A document found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitView {
    /// Document ID.
    pub document_id: i64,
    /// Title (may be empty).
    pub title: String,
    /// Address.
    pub url: String,
    /// Source ID.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
    /// Country of the source.
    pub country: String,
    /// Authority level.
    pub authority: String,
    /// Trust group of the authority: `official`, `expert` or `other`.
    pub trust: String,
    /// `sk` (Slovak data) or `abroad`.
    pub region: String,
    /// Licence.
    pub licence: String,
    /// Language.
    pub language: Option<String>,
    /// Publication time.
    pub published: Option<Timestamp>,
    /// Taxonomy labels `scheme/concept`.
    pub labels: Vec<String>,
    /// The snippet, only where the text may be shown.
    pub snippet: Option<Vec<Segment>>,
    /// The whole text may be shown.
    pub full_text: bool,
}

impl From<Hit> for HitView {
    fn from(h: Hit) -> Self {
        Self {
            snippet: h.snippet.as_ref().map(segments),
            document_id: h.document_id,
            title: h.title,
            url: h.url,
            source_id: h.source_id,
            source_name: h.source_name,
            trust: trust_of(&h.authority).into(),
            region: region_of(&h.country, h.language.as_deref()).into(),
            country: h.country,
            authority: h.authority,
            licence: h.licence,
            language: h.language,
            published: h.published,
            labels: h.labels,
            full_text: h.full_text,
        }
    }
}

/// A version in a document's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    /// Version number, from 1.
    pub number: i32,
    /// When it was captured.
    pub fetched_at: Timestamp,
    /// `initial`, `minor`, `substantive` or `changed`.
    pub change: String,
    /// Paragraphs added.
    pub paragraphs_added: i32,
    /// Paragraphs removed.
    pub paragraphs_removed: i32,
    /// A number or a date changed.
    pub numbers_changed: bool,
}

impl From<atena_store::VersionSummary> for VersionView {
    fn from(v: atena_store::VersionSummary) -> Self {
        Self {
            number: v.version_no,
            fetched_at: v.fetched_at,
            change: v.change_kind,
            paragraphs_added: v.paragraphs_added,
            paragraphs_removed: v.paragraphs_removed,
            numbers_changed: v.numbers_changed,
        }
    }
}

/// A document card: what is known about it and, where the legality principle allows showing it
/// in full, its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentView {
    /// Document ID.
    pub id: i64,
    /// Title (may be empty).
    pub title: String,
    /// Address of the original.
    pub url: String,
    /// Source ID.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
    /// Country of the source.
    pub country: String,
    /// Authority level.
    pub authority: String,
    /// Trust group of the authority: `official`, `expert` or `other`.
    pub trust: String,
    /// Licence.
    pub licence: String,
    /// Language.
    pub language: Option<String>,
    /// Publication time.
    pub published: Option<Timestamp>,
    /// Last change of its text.
    pub last_changed: Timestamp,
    /// Taxonomy labels `scheme/concept`.
    pub labels: Vec<String>,
    /// The text, only where it may be shown in full.
    pub text: Option<String>,
    /// The versions, newest first.
    pub versions: Vec<VersionView>,
}

/// A value of a facet and how many documents found have it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetView {
    /// A scheme, `language`, `source`, `country` …
    pub namespace: String,
    /// The value.
    pub value: String,
    /// Documents.
    pub count: u64,
}

/// The result of a search.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchView {
    /// Documents found.
    pub total: usize,
    /// This page, from 1.
    pub page: usize,
    /// Pages that can be shown.
    pub pages: usize,
    /// All words matched (otherwise some of them).
    pub all_words: bool,
    /// Concepts recognised in the words (`scheme/concept`).
    pub concepts: Vec<String>,
    /// The hits of this page.
    pub hits: Vec<HitView>,
    /// Facets of all documents found.
    pub facets: Vec<FacetView>,
}

impl SearchView {
    /// The view of `results` as page `page` of pages of `per_page`.
    pub fn new(results: Results, page: usize, per_page: usize) -> Self {
        let reachable = atena_search::MAX_OFFSET / per_page + 1;
        Self {
            total: results.total,
            page,
            pages: results.total.div_ceil(per_page).min(reachable),
            all_words: results.all_words,
            concepts: results
                .concepts
                .iter()
                .map(|c| format!("{}/{}", c.scheme, c.concept))
                .collect(),
            hits: results.hits.into_iter().map(HitView::from).collect(),
            facets: results
                .facets
                .into_iter()
                .map(|f| FacetView {
                    namespace: f.namespace,
                    value: f.value,
                    count: f.count,
                })
                .collect(),
        }
    }
}

/// Documents of one concept: all in the scope, and the new ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptCountView {
    /// `scheme/concept`.
    pub key: String,
    /// Documents in the scope.
    pub total: u64,
    /// Of them, new.
    pub new: u64,
}

/// The counts of the overview within a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TallyView {
    /// Documents in the scope.
    pub total: u64,
    /// Of them, new.
    pub new: u64,
    /// "New" means first seen within this many days.
    pub new_days: u32,
    /// Documents per section.
    pub sections: Vec<ConceptCountView>,
    /// Documents per region (`sk`, `abroad`), whatever region is chosen.
    pub regions: BTreeMap<String, u64>,
    /// Documents per language within the region, most first.
    pub languages: Vec<FacetView>,
}

impl TallyView {
    /// The view of a tally of the scheme `scheme`.
    pub fn new(tally: Tally, scheme: &str, new_days: u32) -> Self {
        Self {
            total: tally.total,
            new: tally.new,
            new_days,
            sections: tally
                .concepts
                .into_iter()
                .map(|c| ConceptCountView {
                    key: format!("{scheme}/{}", c.concept),
                    total: c.total,
                    new: c.new,
                })
                .collect(),
            regions: tally
                .regions
                .into_iter()
                .map(|f| (f.value, f.count))
                .collect(),
            languages: tally
                .languages
                .into_iter()
                .map(|f| FacetView {
                    namespace: f.namespace,
                    value: f.value,
                    count: f.count,
                })
                .collect(),
        }
    }
}

/// The state of the crawler.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlView {
    /// It runs.
    pub running: bool,
    /// It was asked to stop and finishes the checks under way.
    pub stopping: bool,
    /// When it started.
    pub started: Option<Timestamp>,
    /// Checks so far.
    pub checks: u64,
    /// Hosts worked on.
    pub hosts: u64,
    /// Failures of the database.
    pub store_errors: u64,
    /// Checks by outcome.
    pub outcomes: BTreeMap<String, u64>,
}

/// A concept for filters and labels, in the interface's language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptView {
    /// `scheme/concept`.
    pub key: String,
    /// Concept ID.
    pub id: String,
    /// Label.
    pub label: String,
    /// Broader concept ID.
    pub broader: Option<String>,
}

/// A scheme of the taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeView {
    /// Scheme ID.
    pub id: String,
    /// Label.
    pub label: String,
    /// Its concepts in the order of the file.
    pub concepts: Vec<ConceptView>,
}

/// The taxonomy in one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxonomyView {
    /// The labels are machine drafts awaiting review by a native speaker.
    pub draft: bool,
    /// The schemes.
    pub schemes: Vec<SchemeView>,
}

impl TaxonomyView {
    /// The taxonomy with labels in `language` (English, then any, where one is missing).
    pub fn new(taxonomy: &Taxonomy, language: &str) -> Self {
        let label = |labels: &BTreeMap<String, String>, id: &str| {
            labels
                .get(language)
                .or_else(|| labels.get("en"))
                .cloned()
                .unwrap_or_else(|| id.to_string())
        };
        Self {
            draft: taxonomy.is_draft_language(language),
            schemes: taxonomy
                .schemes
                .values()
                .map(|s| SchemeView {
                    id: s.id.clone(),
                    label: label(&s.labels, &s.id),
                    concepts: taxonomy
                        .concepts_of(&s.id)
                        .map(|c| ConceptView {
                            key: format!("{}/{}", c.scheme, c.id),
                            id: c.id.clone(),
                            label: c.label(language).to_string(),
                            broader: c.broader.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets_become_pieces() {
        let snippet = Snippet {
            text: "Zápis do prvého ročníka".into(),
            highlighted: vec![0..6, 10..17],
        };
        let pieces = segments(&snippet);
        let shown: Vec<(&str, bool)> = pieces.iter().map(|s| (s.text.as_str(), s.marked)).collect();
        assert_eq!(
            shown,
            [
                ("Zápis", true),
                (" do ", false),
                ("prvého", true),
                (" ročníka", false)
            ]
        );
        // ranges that do not fit are left out, the text stays whole
        let broken = Snippet {
            text: "čaj".into(),
            highlighted: vec![1..2, 2..9],
        };
        let pieces = segments(&broken);
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].text, "čaj");
    }

    #[test]
    fn what_the_principle_allows_is_shown_with_reasons() {
        let open = Allowed::of("SK", "CC-BY-4.0", "unknown");
        assert!(open.crawl && open.store_text && open.show_full_text);
        let unknown = Allowed::of("SK", "copyrighted", "unknown");
        assert!(unknown.crawl && !unknown.store_text);
        assert_eq!(unknown.restrictions, [Restriction::TermsNotVerified]);
        let forbidden = Allowed::of("SK", "copyrighted", "forbidden");
        assert!(!forbidden.crawl);
        assert_eq!(
            serde_json::to_value(&forbidden).unwrap()["restrictions"],
            serde_json::json!(["terms-forbid"])
        );
    }

    #[test]
    fn the_taxonomy_speaks_the_interface_language() {
        let taxonomy = Taxonomy::bundled().unwrap();
        let sk = TaxonomyView::new(&taxonomy, "sk");
        assert!(!sk.draft);
        let section = sk.schemes.iter().find(|s| s.id == "section").unwrap();
        assert_eq!(section.label, "Sekcie");
        assert_eq!(section.concepts[0].label, "Prehľad dňa");
        assert_eq!(section.concepts[0].key, "section/today");
        assert!(TaxonomyView::new(&taxonomy, "zh").draft);
    }
}
