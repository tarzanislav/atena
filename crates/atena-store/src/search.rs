//! What the search index needs: the queue of changed documents and the rows to index.
//!
//! Triggers (migration 0006) queue a document whenever something search shows may have changed.
//! The indexer takes entries, indexes or removes the documents, commits, and only then marks the
//! entries done – an entry queued again meanwhile stays.

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::Row;

use crate::{Result, Store};

/// A queued document and when it was queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueEntry {
    /// Document ID.
    pub document_id: i64,
    /// Time of the latest queuing.
    pub queued_at: Timestamp,
}

/// Everything the index needs about one document, with its source.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexRow {
    /// Document ID.
    pub document_id: i64,
    /// Source ID.
    pub source_id: String,
    /// Source name.
    pub source_name: String,
    /// The source is crawled.
    pub source_enabled: bool,
    /// Country of the source.
    pub country: String,
    /// Authority of the source (A1…E).
    pub authority: String,
    /// Licence of the source.
    pub licence: String,
    /// Terms of the source.
    pub terms: String,
    /// Final URL.
    pub url: String,
    /// `html`, `pdf`, `docx`, `odt`, `text`.
    pub format: String,
    /// `active` or `gone`.
    pub status: String,
    /// Current title.
    pub title: Option<String>,
    /// Current description, if it may be kept.
    pub description: Option<String>,
    /// Current text, if it may be kept.
    pub body: Option<String>,
    /// Language code.
    pub language: Option<String>,
    /// Publication time from the document's metadata.
    pub published_at: Option<Timestamp>,
    /// Modification time from the document's metadata.
    pub modified_at: Option<Timestamp>,
    /// Last change of the text.
    pub last_changed_at: Timestamp,
    /// When ATENA first saw the document.
    pub first_seen_at: Timestamp,
    /// The publisher reserved text and data mining.
    pub tdm_reserved: bool,
    /// Not to be shown in search results.
    pub noindex: bool,
    /// No cached copy may be shown.
    pub noarchive: bool,
    /// No snippet may be shown.
    pub nosnippet: bool,
    /// When the publisher stopped serving it (`status` is `gone`).
    pub gone_at: Option<Timestamp>,
    /// Labels that are not rejected: `(scheme, concept)`.
    pub labels: Vec<(String, String)>,
}

impl Store {
    /// The oldest queued documents, at most `limit`.
    pub async fn search_queue_take(&self, limit: i64) -> Result<Vec<QueueEntry>> {
        let rows = sqlx::query(
            "SELECT document_id, queued_at FROM search_queue ORDER BY queued_at, document_id LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(QueueEntry {
                    document_id: r.try_get("document_id")?,
                    queued_at: r.try_get::<jiff_sqlx::Timestamp, _>("queued_at")?.to_jiff(),
                })
            })
            .collect()
    }

    /// Marks entries done – unless their document was queued again after they were taken.
    pub async fn search_queue_done(&self, entries: &[QueueEntry]) -> Result<u64> {
        let ids: Vec<i64> = entries.iter().map(|e| e.document_id).collect();
        let times: Vec<jiff_sqlx::Timestamp> =
            entries.iter().map(|e| e.queued_at.to_sqlx()).collect();
        let done = sqlx::query(
            "DELETE FROM search_queue q USING unnest($1::bigint[], $2::timestamptz[]) AS t(id, at)
             WHERE q.document_id = t.id AND q.queued_at <= t.at",
        )
        .bind(&ids)
        .bind(&times)
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected())
    }

    /// Queues every document (to rebuild the index).
    pub async fn search_queue_all(&self) -> Result<u64> {
        let queued = sqlx::query(
            "INSERT INTO search_queue (document_id) SELECT id FROM document
             ON CONFLICT (document_id) DO UPDATE SET queued_at = clock_timestamp()",
        )
        .execute(&self.pool)
        .await?;
        Ok(queued.rows_affected())
    }

    /// Number of queued documents.
    pub async fn search_queue_len(&self) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM search_queue")
            .fetch_one(&self.pool)
            .await?)
    }

    /// The rows of existing documents among `ids` (erased documents are simply absent).
    pub async fn search_rows(&self, ids: &[i64]) -> Result<Vec<IndexRow>> {
        let rows = sqlx::query(
            "SELECT d.id, d.source_id, s.name AS source_name, s.enabled, s.country, s.authority, s.licence,
                    s.terms, d.url, d.format, d.status, d.title, v.description, v.body, d.language,
                    d.published_at, d.modified_at, d.last_changed_at, d.first_seen_at, d.tdm_reserved, d.noindex,
                    d.noarchive, d.nosnippet, d.gone_at,
                    COALESCE((SELECT array_agg(l.scheme || '/' || l.concept ORDER BY l.scheme, l.concept)
                              FROM document_label l
                              WHERE l.document_id = d.id AND l.status <> 'rejected'), '{}') AS labels
             FROM document d
             JOIN source s ON s.id = d.source_id
             LEFT JOIN document_version v ON v.id = d.current_version_id
             WHERE d.id = ANY($1)
             ORDER BY d.id",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                let labels: Vec<String> = r.try_get("labels")?;
                Ok(IndexRow {
                    document_id: r.try_get("id")?,
                    source_id: r.try_get("source_id")?,
                    source_name: r.try_get("source_name")?,
                    source_enabled: r.try_get("enabled")?,
                    country: r.try_get("country")?,
                    authority: r.try_get("authority")?,
                    licence: r.try_get("licence")?,
                    terms: r.try_get("terms")?,
                    url: r.try_get("url")?,
                    format: r.try_get("format")?,
                    status: r.try_get("status")?,
                    title: r.try_get("title")?,
                    description: r.try_get("description")?,
                    body: r.try_get("body")?,
                    language: r.try_get("language")?,
                    published_at: r
                        .try_get::<Option<jiff_sqlx::Timestamp>, _>("published_at")?
                        .map(|t| t.to_jiff()),
                    modified_at: r
                        .try_get::<Option<jiff_sqlx::Timestamp>, _>("modified_at")?
                        .map(|t| t.to_jiff()),
                    last_changed_at: r
                        .try_get::<jiff_sqlx::Timestamp, _>("last_changed_at")?
                        .to_jiff(),
                    first_seen_at: r
                        .try_get::<jiff_sqlx::Timestamp, _>("first_seen_at")?
                        .to_jiff(),
                    tdm_reserved: r.try_get("tdm_reserved")?,
                    noindex: r.try_get("noindex")?,
                    noarchive: r.try_get("noarchive")?,
                    nosnippet: r.try_get("nosnippet")?,
                    gone_at: r
                        .try_get::<Option<jiff_sqlx::Timestamp>, _>("gone_at")?
                        .map(|t| t.to_jiff()),
                    labels: labels
                        .into_iter()
                        .filter_map(|l| {
                            l.split_once('/')
                                .map(|(s, c)| (s.to_string(), c.to_string()))
                        })
                        .collect(),
                })
            })
            .collect()
    }
}
