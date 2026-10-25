//! Counts for the overview: how many documents each concept of a scheme gathers, how many of
//! them are new, and how the documents split by region and language – all within a scope given
//! as filters (`region/sk`, `region/abroad` + `language/en` …).

use std::collections::HashSet;
use std::ops::Bound;

use atena_core::Taxonomy;
use jiff::Timestamp;
use serde::Serialize;
use tantivy::collector::{Count, FacetCollector};
use tantivy::query::{AllQuery, Query, RangeQuery};
use tantivy::schema::Facet;
use tantivy::{Searcher, Term};

use crate::index::{SearchIndex, date};
use crate::query::FacetCount;
use crate::schema::{LANGUAGE, REGION};
use crate::{Result, SearchError};

/// Documents of one concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConceptCount {
    /// Concept ID.
    pub concept: String,
    /// Documents in the scope.
    pub total: u64,
    /// Of them, first seen since the given time.
    pub new: u64,
}

/// The counts of a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tally {
    /// Documents in the scope.
    pub total: u64,
    /// Of them, first seen since the given time.
    pub new: u64,
    /// Every concept of the scheme, in the order of the taxonomy.
    pub concepts: Vec<ConceptCount>,
    /// Documents by region, regardless of the region and language chosen.
    pub regions: Vec<FacetCount>,
    /// Documents by language within the chosen region, regardless of the language chosen.
    pub languages: Vec<FacetCount>,
}

/// Filters without those of the given namespaces.
fn without(filters: &[String], namespaces: &[&str]) -> Vec<String> {
    filters
        .iter()
        .filter(|f| {
            !namespaces
                .iter()
                .any(|n| f.trim().split_once('/').is_some_and(|(ns, _)| ns == *n))
        })
        .cloned()
        .collect()
}

impl SearchIndex {
    fn facet_values(
        &self,
        searcher: &Searcher,
        query: &dyn Query,
        namespace: &str,
    ) -> Result<Vec<FacetCount>> {
        let mut collector = FacetCollector::for_field("labels");
        collector.add_facet(Facet::from_path([namespace]));
        let counts = searcher.search(query, &collector)?;
        let mut values: Vec<FacetCount> = counts
            .get(Facet::from_path([namespace]))
            .filter_map(|(facet, count)| {
                facet.to_path().last().map(|value| FacetCount {
                    namespace: namespace.to_string(),
                    value: value.to_string(),
                    count,
                })
            })
            .collect();
        values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
        Ok(values)
    }

    /// Documents per concept of `scheme` within the scope `filters`, and how many of them ATENA
    /// first saw at or after `since`.
    pub fn tally(
        &self,
        taxonomy: &Taxonomy,
        scheme: &str,
        filters: &[String],
        since: Timestamp,
    ) -> Result<Tally> {
        if !taxonomy.schemes.contains_key(scheme) {
            return Err(SearchError::Filter(scheme.to_string()));
        }
        let searcher = self.reader.searcher();
        let scope = self.filters(taxonomy, filters)?;
        let recent: Box<dyn Query> = Box::new(RangeQuery::new(
            Bound::Included(Term::from_field_date(self.fields.added, date(since))),
            Bound::Unbounded,
        ));
        let count = |extra: Vec<Box<dyn Query>>| -> Result<u64> {
            let mut conditions: Vec<Box<dyn Query>> = scope.iter().map(|q| q.box_clone()).collect();
            conditions.extend(extra);
            let query = Self::filtered(Box::new(AllQuery), &conditions);
            Ok(searcher.search(&*query, &Count)? as u64)
        };
        let recent_only = || vec![recent.box_clone()];

        let mut concepts = Vec::new();
        let mut done = HashSet::new();
        for concept in taxonomy.concepts_of(scheme) {
            if !done.insert(concept.id.clone()) {
                continue;
            }
            let members = self.facet_terms(&taxonomy.members(scheme, &concept.id));
            concepts.push(ConceptCount {
                concept: concept.id.clone(),
                total: count(vec![members.box_clone()])?,
                new: count(vec![members, recent.box_clone()])?,
            });
        }

        let all_regions = self.filters(taxonomy, &without(filters, &[REGION, LANGUAGE]))?;
        let all_languages = self.filters(taxonomy, &without(filters, &[LANGUAGE]))?;
        let regions = self.facet_values(
            &searcher,
            &*Self::filtered(Box::new(AllQuery), &all_regions),
            REGION,
        )?;
        let languages = self.facet_values(
            &searcher,
            &*Self::filtered(Box::new(AllQuery), &all_languages),
            LANGUAGE,
        )?;
        Ok(Tally {
            total: count(Vec::new())?,
            new: count(recent_only())?,
            concepts,
            regions,
            languages,
        })
    }
}
