//! The index on disk (or in memory) and how documents are written into it.
//!
//! An index directory carries a marker file with the format version. ATENA opens only directories
//! with its marker (or empty ones), and deletes only those when rebuilding – never a directory
//! that happens to be named in `ATENA_INDEX` by mistake.

use std::fs;
use std::path::Path;

use tantivy::schema::Facet;
use tantivy::store::Compressor;
use tantivy::{
    DateTime, Index, IndexReader, IndexSettings, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};

use crate::analysis;
use crate::document::{SearchDocument, authority_weight};
use crate::schema::{Fields, schema};
use crate::{Result, SearchError};

/// Name of the marker file.
const MARKER: &str = "atena-search.format";
/// Format of the index; raised whenever fields or analyzers change, which requires a rebuild.
const FORMAT: &str = "3";
/// Memory for the writer, shared by its threads (each needs at least 15 MB).
const WRITER_MEMORY: usize = 64 * 1024 * 1024;
/// Most writer threads.
const WRITER_THREADS: usize = 4;

/// The search index.
pub struct SearchIndex {
    pub(crate) index: Index,
    pub(crate) reader: IndexReader,
    pub(crate) fields: Fields,
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> SearchError + '_ {
    move |source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    Ok(fs::read_dir(path).map_err(io(path))?.next().is_none())
}

fn marker(path: &Path) -> Option<String> {
    fs::read_to_string(path.join(MARKER))
        .ok()
        .map(|m| m.trim().to_string())
}

impl SearchIndex {
    fn new(index: Index) -> Result<Self> {
        analysis::register(index.tokenizers());
        let (_, fields) = schema();
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        Ok(Self {
            index,
            reader,
            fields,
        })
    }

    fn settings() -> IndexSettings {
        IndexSettings {
            docstore_compression: Compressor::Lz4,
            ..IndexSettings::default()
        }
    }

    fn create(path: &Path) -> Result<Self> {
        fs::create_dir_all(path).map_err(io(path))?;
        fs::write(path.join(MARKER), FORMAT).map_err(io(path))?;
        let index = Index::builder()
            .schema(schema().0)
            .settings(Self::settings())
            .create_in_dir(path)?;
        Self::new(index)
    }

    /// Opens the index in `path`, creating it when the directory is missing or empty.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() || is_empty_dir(path)? {
            return Self::create(path);
        }
        match marker(path) {
            None => Err(SearchError::NotAnIndex(path.to_path_buf())),
            Some(format) if format != FORMAT => Err(SearchError::Outdated(path.to_path_buf())),
            // the marker was written, the index not (an interrupted creation)
            Some(_) if !path.join("meta.json").exists() => Self::create(path),
            Some(_) => {
                let index = Index::open_in_dir(path)?;
                if index.schema() != schema().0 {
                    return Err(SearchError::Outdated(path.to_path_buf()));
                }
                Self::new(index)
            }
        }
    }

    /// Deletes the index in `path` (only an ATENA index) and creates an empty one.
    pub fn recreate(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() && !is_empty_dir(path)? {
            if marker(path).is_none() {
                return Err(SearchError::NotAnIndex(path.to_path_buf()));
            }
            fs::remove_dir_all(path).map_err(io(path))?;
        }
        Self::create(path)
    }

    /// An index in memory (tests, previews).
    pub fn in_memory() -> Result<Self> {
        let index = Index::builder()
            .schema(schema().0)
            .settings(Self::settings())
            .create_in_ram()?;
        Self::new(index)
    }

    /// Number of searchable documents (as of the last update).
    pub fn len(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    /// `true` when nothing can be found.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Writes documents, and removes those given without one, in one commit.
    pub fn apply(
        &self,
        changes: impl IntoIterator<Item = (i64, Option<SearchDocument>)>,
    ) -> Result<()> {
        let mut writer = self.writer()?;
        for (id, document) in changes {
            self.put(&writer, id, document.as_ref())?;
        }
        self.commit(&mut writer)?;
        writer.wait_merging_threads()?;
        Ok(())
    }

    pub(crate) fn writer(&self) -> Result<IndexWriter> {
        let threads = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(WRITER_THREADS);
        Ok(self.index.writer_with_num_threads(threads, WRITER_MEMORY)?)
    }

    pub(crate) fn id_term(&self, id: i64) -> Term {
        Term::from_field_u64(self.fields.id, id_value(id))
    }

    /// Replaces the document `id` in the writer's next commit (`None`: removes it).
    pub(crate) fn put(
        &self,
        writer: &IndexWriter,
        id: i64,
        document: Option<&SearchDocument>,
    ) -> Result<()> {
        writer.delete_term(self.id_term(id));
        if let Some(d) = document {
            writer.add_document(self.tantivy_document(d))?;
        }
        Ok(())
    }

    /// Commits and makes the changes visible to searches.
    pub(crate) fn commit(&self, writer: &mut IndexWriter) -> Result<()> {
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    fn tantivy_document(&self, d: &SearchDocument) -> TantivyDocument {
        let f = &self.fields;
        let mut doc = TantivyDocument::new();
        doc.add_u64(f.id, id_value(d.id));
        doc.add_text(f.url, &d.url);
        doc.add_text(f.source, &d.source_id);
        doc.add_text(f.source_name, &d.source_name);
        doc.add_text(f.title, &d.title);
        doc.add_text(f.titles[d.group], &d.title);
        if let Some(language) = &d.language {
            doc.add_text(f.language, language);
        }
        doc.add_text(f.country, &d.country);
        doc.add_text(f.authority, &d.authority);
        doc.add_text(f.licence, &d.licence);
        if let Some(published) = d.published {
            doc.add_date(f.published, date(published));
        }
        doc.add_date(f.dated, date(d.dated));
        doc.add_date(f.added, date(d.added));
        doc.add_f64(f.weight, authority_weight(&d.authority));
        doc.add_bool(f.full_text, d.full_text);
        if let Some(text) = &d.text {
            doc.add_text(f.texts[d.group], text);
            if d.snippets {
                doc.add_text(f.shown, text);
            }
        }
        for (namespace, value) in &d.facets {
            doc.add_facet(f.labels, facet(namespace, value));
        }
        doc
    }
}

/// Document IDs are positive; the index keeps them as `u64`.
fn id_value(id: i64) -> u64 {
    u64::try_from(id).unwrap_or_default()
}

pub(crate) fn date(time: jiff::Timestamp) -> DateTime {
    DateTime::from_timestamp_secs(time.as_second())
}

pub(crate) fn facet(namespace: &str, value: &str) -> Facet {
    Facet::from_path([namespace, value])
}
