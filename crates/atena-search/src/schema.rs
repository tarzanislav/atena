//! The fields of the index.
//!
//! Only what search needs is stored, and only what may be shown: the title, address and
//! description of the source for every hit, the text itself only for documents whose text ATENA
//! may show (field `shown`). Texts that may be analysed but not shown are indexed and not stored.

use tantivy::schema::{
    DateOptions, FacetOptions, Field, IndexRecordOption, NumericOptions, STORED, Schema,
    TextFieldIndexing, TextOptions,
};

use crate::analysis::GROUPS;

/// Facet namespaces that are not taxonomy schemes.
pub(crate) const LANGUAGE: &str = "language";
/// The source of a document.
pub(crate) const SOURCE: &str = "source";
/// The country of the source (also a taxonomy scheme).
pub(crate) const COUNTRY: &str = "country";
/// The authority of the source (also a taxonomy scheme).
pub(crate) const AUTHORITY: &str = "authority";

/// Handles of all fields.
#[derive(Debug, Clone)]
pub(crate) struct Fields {
    /// Document ID (indexed for updates and deletions).
    pub id: Field,
    pub url: Field,
    pub source: Field,
    pub source_name: Field,
    /// The title as shown.
    pub title: Field,
    pub language: Field,
    pub country: Field,
    pub authority: Field,
    pub licence: Field,
    pub published: Field,
    /// Date used for freshness in ranking.
    pub dated: Field,
    /// Authority weight used in ranking.
    pub weight: Field,
    /// The whole text may be shown in ATENA.
    pub full_text: Field,
    /// The text, stored only where snippets may be shown.
    pub shown: Field,
    /// Taxonomy labels and the namespaces above, as facets `/scheme/concept`.
    pub labels: Field,
    /// Title fields by language group.
    pub titles: Vec<Field>,
    /// Text fields by language group.
    pub texts: Vec<Field>,
}

fn analysed(analyzer: &str) -> TextOptions {
    TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer(analyzer)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    )
}

/// The schema and its fields; the same every time, so an existing index can be compared with it.
pub(crate) fn schema() -> (Schema, Fields) {
    let mut b = Schema::builder();
    let stored = TextOptions::default().set_stored();
    let id = b.add_u64_field(
        "id",
        NumericOptions::default()
            .set_indexed()
            .set_stored()
            .set_fast(),
    );
    let url = b.add_text_field("url", stored.clone());
    let source = b.add_text_field("source", stored.clone());
    let source_name = b.add_text_field("source_name", stored.clone());
    let title = b.add_text_field("title", stored.clone());
    let language = b.add_text_field("language", stored.clone());
    let country = b.add_text_field("country", stored.clone());
    let authority = b.add_text_field("authority", stored.clone());
    let licence = b.add_text_field("licence", stored.clone());
    let published = b.add_date_field("published", DateOptions::default().set_stored());
    let dated = b.add_date_field("dated", DateOptions::default().set_fast());
    let weight = b.add_f64_field("weight", NumericOptions::default().set_fast());
    let full_text = b.add_bool_field("full_text", STORED);
    let shown = b.add_text_field("shown", stored);
    let labels = b.add_facet_field("labels", FacetOptions::default().set_stored());
    let mut titles = Vec::with_capacity(GROUPS.len());
    let mut texts = Vec::with_capacity(GROUPS.len());
    for group in GROUPS {
        let analyzer = group.analyzer_name();
        titles.push(b.add_text_field(&format!("title_{}", group.id), analysed(&analyzer)));
        texts.push(b.add_text_field(&format!("text_{}", group.id), analysed(&analyzer)));
    }
    (
        b.build(),
        Fields {
            id,
            url,
            source,
            source_name,
            title,
            language,
            country,
            authority,
            licence,
            published,
            dated,
            weight,
            full_text,
            shown,
            labels,
            titles,
            texts,
        },
    )
}
