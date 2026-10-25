//! Classifying stored documents again after the taxonomy changed – new concepts and keywords
//! then apply to what ATENA already has, not only to what it collects next.
//!
//! Classification sees exactly what the crawler saw and was allowed to keep: the title and the
//! kept text (nothing for documents of which only metadata may be kept). Only automatic
//! suggestions are replaced; an editor's decisions stay. A changed label queues the document for
//! the search index (migration 0006).

use atena_core::{Taxonomy, suggest};
use atena_store::{LabelSuggestion, Store, StoreError};

/// Documents read at a time.
const BATCH: i64 = 200;

/// What a run did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReclassifyReport {
    /// Documents looked at.
    pub documents: u64,
    /// Documents whose suggestions changed.
    pub changed: u64,
}

/// Classifies every stored document again with `taxonomy`; `threshold` is the lowest score of a
/// suggestion (as in `PipelineConfig::classify_threshold`).
pub async fn reclassify(
    store: &Store,
    taxonomy: &Taxonomy,
    threshold: f64,
) -> Result<ReclassifyReport, StoreError> {
    let mut report = ReclassifyReport::default();
    let mut after = 0;
    loop {
        let batch = store.classifiable(after, BATCH).await?;
        let Some(last) = batch.last() else {
            break;
        };
        after = last.document_id;
        for doc in batch {
            report.documents += 1;
            let mut suggestions: Vec<LabelSuggestion> = suggest(
                taxonomy,
                &doc.title,
                doc.body.as_deref().unwrap_or(""),
                doc.language.as_deref(),
                threshold,
            )
            .into_iter()
            .map(|s| LabelSuggestion {
                scheme: s.scheme,
                concept: s.concept,
                score: s.score as f32,
                evidence: s.evidence,
            })
            .collect();
            suggestions.sort_by(|a, b| (&a.scheme, &a.concept).cmp(&(&b.scheme, &b.concept)));
            let same = suggestions.len() == doc.suggested.len()
                && suggestions
                    .iter()
                    .zip(&doc.suggested)
                    .all(|(s, (scheme, concept))| s.scheme == *scheme && s.concept == *concept);
            if !same {
                store
                    .replace_suggestions(doc.document_id, doc.version_id, &suggestions)
                    .await?;
                report.changed += 1;
            }
        }
    }
    Ok(report)
}
