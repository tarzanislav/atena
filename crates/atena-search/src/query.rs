//! Searching: words in every language, concepts across languages, filters, ranking – and the
//! legal limits of what a hit shows.
//!
//! * **Words** are looked up in the title and text fields of every language group, each analysed
//!   by its own analyzer; titles weigh three times as much. All words must match; when no document
//!   has them all, any of them may.
//! * **Concepts.** The words are also matched against the keywords of the taxonomy
//!   (`atena_core::classify`). Documents labelled with a recognised concept – or a narrower one –
//!   are found in any language, so `čitateľská gramotnosť` finds English documents on reading
//!   literacy.
//! * **Filters** `scheme/concept` (narrower concepts included), `language/xx`, `source/ID`: values of
//!   one scheme are alternatives, different schemes must all hold.
//! * **Ranking** is the relevance multiplied by the authority of the source (A1 1.5 … E 0.8) and
//!   by freshness (up to a quarter more for the newest, halving every year).
//! * **What a hit shows**: title, address, source and labels always; a snippet of the text only
//!   where ATENA may show the text (see `document`).

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use atena_core::{Taxonomy, suggest};
use jiff::Timestamp;
use serde::Serialize;
use tantivy::collector::{Count, FacetCollector, FacetCounts, TopDocs};
use tantivy::query::{
    AllQuery, BooleanQuery, ConstScoreQuery, Occur, Query, QueryParser, TermQuery,
};
use tantivy::schema::{Facet, IndexRecordOption, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{DateTime, DocId, Score, Searcher, SegmentReader, TantivyDocument, Term};

use crate::analysis::group_of;
use crate::index::{SearchIndex, facet};
use crate::schema::{LANGUAGE, SOURCE};
use crate::{Result, SearchError};

/// Weight of the title fields.
const TITLE_BOOST: Score = 3.0;
/// Score of a document labelled with a recognised concept, times the confidence of recognition.
const CONCEPT_SCORE: Score = 1.5;
/// Least confidence of a recognised concept.
const CONCEPT_THRESHOLD: f64 = 0.45;
/// Most concepts recognised in one search.
const MAX_CONCEPTS: usize = 5;
/// Length of a snippet in characters.
const SNIPPET_CHARS: usize = 240;
/// The newest documents rank up to this much higher …
const FRESHNESS: f64 = 0.25;
/// … and the advantage halves in this many days.
const HALF_LIFE_DAYS: f64 = 365.0;
/// Most hits per page.
pub const MAX_LIMIT: usize = 100;
/// Most values counted per facet namespace.
const FACET_VALUES: usize = 20;
/// Longer searches are cut (in characters): a search is a few words, not a document.
const MAX_TEXT_CHARS: usize = 500;
/// Most filters in one search.
const MAX_FILTERS: usize = 50;

/// A search.
#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    /// The words. `"…"` searches a phrase, `-word` excludes a word.
    pub text: String,
    /// Language of the words, for recognising concepts; `None`: every language.
    pub language: Option<String>,
    /// Filters: `scheme/concept`, `language/xx`, `source/ID`.
    pub filters: Vec<String>,
    /// Hits per page (1 to [`MAX_LIMIT`]).
    pub limit: usize,
    /// Hits skipped (for further pages).
    pub offset: usize,
}

/// A concept recognised in the words.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecognisedConcept {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID.
    pub concept: String,
    /// Confidence between 0 and 1.
    pub score: f64,
    /// Keywords that matched.
    pub evidence: Vec<String>,
}

/// A piece of text with the matching words marked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snippet {
    /// The text.
    pub text: String,
    /// Byte ranges of the matching words in `text`.
    pub highlighted: Vec<Range<usize>>,
}

/// One document found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hit {
    /// Document ID.
    pub document_id: i64,
    /// Score after ranking.
    pub score: f32,
    /// Title (may be empty).
    pub title: String,
    /// Address of the document.
    pub url: String,
    /// Source ID.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
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
    /// Taxonomy labels `scheme/concept`.
    pub labels: Vec<String>,
    /// A snippet – only where the text may be shown.
    pub snippet: Option<Snippet>,
    /// The whole text may be shown in ATENA.
    pub full_text: bool,
}

/// How many hits have a value of a facet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FacetCount {
    /// Namespace: a scheme, `language` or `source`.
    pub namespace: String,
    /// Value: a concept, a language code or a source ID.
    pub value: String,
    /// Number of hits.
    pub count: u64,
}

/// The result of a search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Results {
    /// Number of documents found.
    pub total: usize,
    /// The requested page.
    pub hits: Vec<Hit>,
    /// Concepts recognised in the words.
    pub concepts: Vec<RecognisedConcept>,
    /// Counts of facet values among all documents found.
    pub facets: Vec<FacetCount>,
    /// All words had to match; `false` when no document had them all and any of them sufficed.
    pub all_words: bool,
}

/// Number of words outside quotes and operators – for deciding whether "any word" differs.
fn word_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

fn opening(text: &str) -> Snippet {
    let cut = text
        .char_indices()
        .nth(SNIPPET_CHARS)
        .map_or(text.len(), |(i, _)| {
            text[..i]
                .rfind(char::is_whitespace)
                .filter(|&s| s > 0)
                .unwrap_or(i)
        });
    Snippet {
        text: text[..cut].trim_end().to_string(),
        highlighted: Vec::new(),
    }
}

impl SearchIndex {
    fn text_query(&self, text: &str, all_words: bool) -> Option<Box<dyn Query>> {
        let f = &self.fields;
        let fields = f.titles.iter().chain(&f.texts).copied().collect();
        let mut parser = QueryParser::for_index(&self.index, fields);
        for &title in &f.titles {
            parser.set_field_boost(title, TITLE_BOOST);
        }
        if all_words {
            parser.set_conjunction_by_default();
        }
        let (query, _ignored) = parser.parse_query_lenient(text);
        let mut has_terms = false;
        query.query_terms(&mut |_, _| has_terms = true);
        has_terms.then_some(query)
    }

    fn facet_terms(&self, namespace: &str, values: &[String]) -> Box<dyn Query> {
        let clauses = values
            .iter()
            .map(|v| {
                let term = Term::from_facet(self.fields.labels, &facet(namespace, v));
                let q: Box<dyn Query> = Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                (Occur::Should, q)
            })
            .collect();
        Box::new(BooleanQuery::new(clauses))
    }

    /// Filters grouped by namespace, each with its narrower concepts.
    fn filters(&self, taxonomy: &Taxonomy, filters: &[String]) -> Result<Vec<Box<dyn Query>>> {
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for filter in filters {
            let bad = || SearchError::Filter(filter.clone());
            let (namespace, value) = filter.trim().split_once('/').ok_or_else(bad)?;
            let values = groups.entry(namespace.to_string()).or_default();
            match namespace {
                LANGUAGE => values.push(value.to_ascii_lowercase()),
                SOURCE => values.push(value.to_string()),
                scheme if taxonomy.get(scheme, value).is_some() => {
                    values.push(value.to_string());
                    values.extend(taxonomy.descendants(scheme, value));
                }
                _ => return Err(bad()),
            }
        }
        Ok(groups
            .iter()
            .map(|(namespace, values)| self.facet_terms(namespace, values))
            .collect())
    }

    fn concepts(
        &self,
        taxonomy: &Taxonomy,
        text: &str,
        language: Option<&str>,
    ) -> Vec<RecognisedConcept> {
        if text.trim().is_empty() {
            return Vec::new();
        }
        let language = language
            .and_then(|l| l.split(['-', '_']).next())
            .map(str::to_ascii_lowercase);
        let mut found = suggest(taxonomy, text, "", language.as_deref(), CONCEPT_THRESHOLD);
        found.sort_by(|a, b| b.score.total_cmp(&a.score));
        found.truncate(MAX_CONCEPTS);
        found
            .into_iter()
            .map(|s| RecognisedConcept {
                scheme: s.scheme,
                concept: s.concept,
                score: s.score,
                evidence: s.evidence,
            })
            .collect()
    }

    fn concept_query(&self, taxonomy: &Taxonomy, c: &RecognisedConcept) -> Box<dyn Query> {
        let mut values = vec![c.concept.clone()];
        values.extend(taxonomy.descendants(&c.scheme, &c.concept));
        let score = CONCEPT_SCORE * c.score as Score;
        Box::new(ConstScoreQuery::new(
            self.facet_terms(&c.scheme, &values),
            score,
        ))
    }

    /// The query with the filters as conditions that do not change the score.
    fn filtered(main: Box<dyn Query>, filters: &[Box<dyn Query>]) -> Box<dyn Query> {
        if filters.is_empty() {
            return main;
        }
        let mut clauses = vec![(Occur::Must, main)];
        for f in filters {
            let condition: Box<dyn Query> = Box::new(ConstScoreQuery::new(f.box_clone(), 0.0));
            clauses.push((Occur::Must, condition));
        }
        Box::new(BooleanQuery::new(clauses))
    }

    /// Searches. Taxonomy: for recognising concepts and expanding filters to narrower concepts.
    pub fn search(&self, taxonomy: &Taxonomy, request: &SearchRequest) -> Result<Results> {
        let searcher = self.reader.searcher();
        if request.filters.len() > MAX_FILTERS {
            return Err(SearchError::TooManyFilters(MAX_FILTERS));
        }
        let filters = self.filters(taxonomy, &request.filters)?;
        let text: String = request.text.chars().take(MAX_TEXT_CHARS).collect();
        let mut all_words = true;
        let mut words = self.text_query(&text, true);
        if let Some(q) = &words {
            let strict = Self::filtered(q.box_clone(), &filters);
            if word_count(&text) > 1 && searcher.search(&*strict, &Count)? == 0 {
                all_words = false;
                words = self.text_query(&text, false);
            }
        }
        let concepts = self.concepts(taxonomy, &text, request.language.as_deref());
        let mut alternatives: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        if let Some(q) = words {
            alternatives.push((Occur::Should, q));
        }
        for c in &concepts {
            alternatives.push((Occur::Should, self.concept_query(taxonomy, c)));
        }
        let main: Box<dyn Query> = match (alternatives.is_empty(), filters.is_empty()) {
            (false, _) => Box::new(BooleanQuery::new(alternatives)),
            (true, false) => Box::new(AllQuery),
            (true, true) => return Err(SearchError::EmptyQuery),
        };
        let query = Self::filtered(main, &filters);

        let mut namespaces: Vec<&str> = taxonomy.schemes.keys().map(String::as_str).collect();
        namespaces.extend([LANGUAGE, SOURCE]);
        let mut facets = FacetCollector::for_field("labels");
        for namespace in &namespaces {
            facets.add_facet(Facet::from_path([*namespace]));
        }
        let limit = request.limit.clamp(1, MAX_LIMIT);
        let top = TopDocs::with_limit(limit)
            .and_offset(request.offset)
            .tweak_score(rank(Timestamp::now().as_second()));
        let (top, total, counts) = searcher.search(&*query, &(top, Count, facets))?;

        let mut generators = HashMap::new();
        let hits = top
            .into_iter()
            .map(|(score, address)| {
                let doc: TantivyDocument = searcher.doc(address)?;
                self.hit(&searcher, &*query, &mut generators, score, &doc)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Results {
            total,
            hits,
            concepts,
            facets: facet_counts(&counts, &namespaces),
            all_words,
        })
    }

    fn hit(
        &self,
        searcher: &Searcher,
        query: &dyn Query,
        generators: &mut HashMap<usize, SnippetGenerator>,
        score: Score,
        doc: &TantivyDocument,
    ) -> Result<Hit> {
        let f = &self.fields;
        let text = |field| {
            doc.get_first(field)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let language = text(f.language);
        let snippet = match doc.get_first(f.shown).and_then(|v| v.as_str()) {
            Some(shown) => {
                let group = group_of(language.as_deref());
                let generator = match generators.entry(group) {
                    std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                    std::collections::hash_map::Entry::Vacant(e) => {
                        let mut g = SnippetGenerator::create(searcher, query, f.texts[group])?;
                        g.set_max_num_chars(SNIPPET_CHARS);
                        e.insert(g)
                    }
                };
                let s = generator.snippet(shown);
                Some(if s.is_empty() {
                    opening(shown)
                } else {
                    Snippet {
                        text: s.fragment().to_string(),
                        highlighted: s.highlighted().to_vec(),
                    }
                })
            }
            None => None,
        };
        let labels = doc
            .get_all(f.labels)
            .filter_map(|v| v.as_facet())
            .filter_map(|encoded| Facet::from_encoded(encoded.as_bytes().to_vec()).ok())
            .filter_map(|facet| match facet.to_path().as_slice() {
                [namespace, value] if *namespace != LANGUAGE && *namespace != SOURCE => {
                    Some(format!("{namespace}/{value}"))
                }
                _ => None,
            })
            .collect();
        Ok(Hit {
            document_id: doc
                .get_first(f.id)
                .and_then(|v| v.as_u64())
                .and_then(|id| i64::try_from(id).ok())
                .unwrap_or_default(),
            score,
            title: text(f.title).unwrap_or_default(),
            url: text(f.url).unwrap_or_default(),
            source_id: text(f.source).unwrap_or_default(),
            source_name: text(f.source_name).unwrap_or_default(),
            language,
            country: text(f.country).unwrap_or_default(),
            authority: text(f.authority).unwrap_or_default(),
            licence: text(f.licence).unwrap_or_default(),
            published: doc
                .get_first(f.published)
                .and_then(|v| v.as_datetime())
                .and_then(|d| Timestamp::from_second(d.into_timestamp_secs()).ok()),
            labels,
            snippet,
            full_text: doc
                .get_first(f.full_text)
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }
}

/// The ranking: relevance × authority × freshness.
fn rank(now: i64) -> impl Fn(&SegmentReader) -> Box<dyn Fn(DocId, Score) -> Score> + Send + Sync {
    move |segment: &SegmentReader| {
        let fast = segment.fast_fields();
        let weight = fast.f64("weight").ok().map(|c| c.first_or_default_col(1.0));
        let dated = fast
            .date("dated")
            .ok()
            .map(|c| c.first_or_default_col(DateTime::from_timestamp_secs(now)));
        Box::new(move |doc: DocId, score: Score| {
            let weight = weight.as_ref().map_or(1.0, |c| c.get_val(doc));
            let age_days = dated.as_ref().map_or(0.0, |c| {
                (now - c.get_val(doc).into_timestamp_secs()).max(0) as f64 / 86_400.0
            });
            let freshness = 1.0 + FRESHNESS * 0.5f64.powf(age_days / HALF_LIFE_DAYS);
            score * (weight * freshness) as Score
        })
    }
}

fn facet_counts(counts: &FacetCounts, namespaces: &[&str]) -> Vec<FacetCount> {
    let mut out = Vec::new();
    for namespace in namespaces {
        let mut values: Vec<FacetCount> = counts
            .get(Facet::from_path([*namespace]))
            .filter_map(|(facet, count)| {
                facet.to_path().last().map(|value| FacetCount {
                    namespace: namespace.to_string(),
                    value: value.to_string(),
                    count,
                })
            })
            .collect();
        values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
        values.truncate(FACET_VALUES);
        out.extend(values);
    }
    out
}
