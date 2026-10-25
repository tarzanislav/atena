//! Documents, their versions, links and classification.
//!
//! A capture whose text equals the current version only touches the document (`last_seen_at`,
//! publisher rules). A capture with new text becomes the next version in one transaction, together
//! with the document's links and the new classification suggestions.

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::{Postgres, Row, Transaction};

use crate::{Hash, Result, Store, StoreError};

/// The publisher's rules for a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rights {
    /// Text and data mining is reserved: only metadata is kept.
    pub tdm_reserved: bool,
    /// Licensing policy of the reservation.
    pub tdm_policy: Option<String>,
    /// Not to be shown in search results.
    pub noindex: bool,
    /// No stored copy may be shown.
    pub noarchive: bool,
    /// No excerpts may be shown.
    pub nosnippet: bool,
    /// No translation may be offered.
    pub notranslate: bool,
}

/// One capture of an endpoint's content.
#[derive(Debug, Clone)]
pub struct Capture {
    /// Endpoint the content came from.
    pub endpoint_id: i64,
    /// Its source.
    pub source_id: String,
    /// Final URL after redirects.
    pub url: String,
    /// `html`, `pdf`, `docx`, `odt` or `text`.
    pub format: String,
    /// Hash of the original bytes.
    pub raw_hash: Hash,
    /// Fingerprint of the normalised text.
    pub fingerprint: Hash,
    /// When it was fetched.
    pub fetched_at: Timestamp,
    /// Publisher's rules.
    pub rights: Rights,
}

/// The content of a new version.
#[derive(Debug, Clone, Default)]
pub struct NewVersion {
    /// `initial`, `minor`, `substantive` or `changed`.
    pub change_kind: String,
    /// Paragraphs added since the previous version.
    pub paragraphs_added: i32,
    /// Paragraphs removed since the previous version.
    pub paragraphs_removed: i32,
    /// A number or date changed.
    pub numbers_changed: bool,
    /// `Content-Type` of the response.
    pub content_type: Option<String>,
    /// Encoding the text was decoded from.
    pub encoding: Option<String>,
    /// Title.
    pub title: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Text; `None` when only metadata may be kept.
    pub body: Option<String>,
    /// The text was cut at the size limit.
    pub body_truncated: bool,
    /// Language code.
    pub language: Option<String>,
    /// `detected` or `declared`.
    pub language_source: Option<String>,
    /// Detection confidence.
    pub language_confidence: Option<f32>,
    /// Authors.
    pub authors: Vec<String>,
    /// Publication time from metadata.
    pub published_at: Option<Timestamp>,
    /// Modification time from metadata.
    pub modified_at: Option<Timestamp>,
    /// Canonical URL declared by the page.
    pub canonical_url: Option<String>,
    /// The original file to archive with the version (official and openly licensed PDF and office
    /// documents only – the pipeline decides).
    pub file: Option<NewFile>,
}

/// An original file to archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    /// Media type of the recognised format.
    pub media_type: String,
    /// The bytes as the publisher served them.
    pub content: Vec<u8>,
}

/// An archived original file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedFile {
    /// The version it belongs to.
    pub version_id: i64,
    /// Media type.
    pub media_type: String,
    /// Size in bytes.
    pub byte_size: i64,
    /// BLAKE3 of the bytes.
    pub raw_hash: Hash,
    /// When it was archived.
    pub stored_at: Timestamp,
}

/// What decides whether an archived file may still be kept and shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveRights {
    /// Document ID.
    pub document_id: i64,
    /// Country of the source.
    pub country: String,
    /// Licence of the source.
    pub licence: String,
    /// Terms of the source.
    pub terms: String,
    /// The publisher reserved text and data mining.
    pub tdm_reserved: bool,
    /// The publisher asked for no stored copies.
    pub noarchive: bool,
}

/// What a link of a document points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// An ordinary link in the content.
    Link,
    /// A video on YouTube or Vimeo, linked or embedded – recorded, never fetched.
    Video,
}

impl LinkKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Video => "video",
        }
    }

    fn parse(text: &str) -> Self {
        if text == "video" {
            Self::Video
        } else {
            Self::Link
        }
    }
}

/// A link of a document: its address, its text (for videos the player's title) and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    /// Absolute URL.
    pub url: String,
    /// Link text or title, privacy-cleaned by the caller.
    pub text: String,
    /// Link or video.
    pub kind: LinkKind,
}

impl DocumentLink {
    /// An ordinary link.
    pub fn link(url: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            text: text.into(),
            kind: LinkKind::Link,
        }
    }

    /// A video.
    pub fn video(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            text: title.into(),
            kind: LinkKind::Video,
        }
    }
}

/// An automatic classification suggestion.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelSuggestion {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID.
    pub concept: String,
    /// Confidence 0–1.
    pub score: f32,
    /// Keywords that caused it.
    pub evidence: Vec<String>,
}

/// An editor's decision on a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelDecision {
    /// The label is right.
    Confirm,
    /// The label is wrong and must not be suggested again.
    Reject,
}

/// IDs of a saved version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedVersion {
    /// Document ID.
    pub document_id: i64,
    /// Version ID.
    pub version_id: i64,
    /// Version number (1 for the first capture).
    pub version_no: i32,
}

/// What the pipeline needs to know about an endpoint's document before processing a capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDocument {
    /// Document ID.
    pub id: i64,
    /// Number of versions.
    pub version_count: i32,
    /// Hash of the latest capture's bytes.
    pub last_raw_hash: Option<Hash>,
    /// Fingerprint of the latest capture's text.
    pub last_fingerprint: Option<Hash>,
    /// Text of the current version, if kept.
    pub body: Option<String>,
    /// `active` or `gone`.
    pub status: String,
    /// Text and data mining was reserved at the latest capture.
    pub tdm_reserved: bool,
    /// The current version.
    pub version_id: Option<i64>,
    /// The current version has an archived file.
    pub has_file: bool,
}

/// A version in a document's history.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionSummary {
    /// Version ID.
    pub id: i64,
    /// Version number.
    pub version_no: i32,
    /// Capture time.
    pub fetched_at: Timestamp,
    /// Kind of change.
    pub change_kind: String,
    /// Paragraphs added.
    pub paragraphs_added: i32,
    /// Paragraphs removed.
    pub paragraphs_removed: i32,
    /// A number or date changed.
    pub numbers_changed: bool,
    /// Title.
    pub title: Option<String>,
    /// The text is kept.
    pub has_body: bool,
}

/// A label of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// Scheme ID.
    pub scheme: String,
    /// Concept ID.
    pub concept: String,
    /// Confidence.
    pub score: f32,
    /// `auto` or `editor`.
    pub origin: String,
    /// `suggested`, `confirmed` or `rejected`.
    pub status: String,
}

fn hash_from(bytes: Option<Vec<u8>>) -> Result<Option<Hash>> {
    bytes
        .map(|b| {
            Hash::try_from(b.as_slice())
                .map_err(|_| StoreError::Invalid("stored hash is not 32 bytes".into()))
        })
        .transpose()
}

/// Removes everything a reservation forbids keeping: texts, descriptions, links and automatic
/// suggestions.
async fn forget_content(tx: &mut Transaction<'_, Postgres>, document_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE document_version SET body = NULL, description = NULL
         WHERE document_id = $1 AND (body IS NOT NULL OR description IS NOT NULL)",
    )
    .bind(document_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM document_link WHERE document_id = $1")
        .bind(document_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "DELETE FROM version_file WHERE version_id IN
             (SELECT id FROM document_version WHERE document_id = $1)",
    )
    .bind(document_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM document_label WHERE document_id = $1 AND origin = 'auto' AND status = 'suggested'")
        .bind(document_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Archives the original file of a version (once; a second one is ignored).
async fn insert_file(
    tx: &mut Transaction<'_, Postgres>,
    version_id: i64,
    file: &NewFile,
) -> Result<bool> {
    let size = i32::try_from(file.content.len()).unwrap_or(i32::MAX);
    if file.content.is_empty() || size == i32::MAX {
        return Ok(false);
    }
    let inserted = sqlx::query(
        "INSERT INTO version_file (version_id, media_type, byte_size, raw_hash, content)
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (version_id) DO NOTHING",
    )
    .bind(version_id)
    .bind(&file.media_type)
    .bind(size)
    .bind(&crate::hash::hash(&file.content)[..])
    .bind(&file.content)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}

/// New suggestions replace the old ones; confirmed and rejected labels stay as decided.
async fn write_suggestions(
    tx: &mut Transaction<'_, Postgres>,
    document_id: i64,
    version_id: Option<i64>,
    suggestions: &[LabelSuggestion],
) -> Result<()> {
    sqlx::query("DELETE FROM document_label WHERE document_id = $1 AND origin = 'auto' AND status = 'suggested'")
        .bind(document_id)
        .execute(&mut **tx)
        .await?;
    let evidence: Vec<String> = suggestions
        .iter()
        .map(|s| s.evidence.join("\u{1f}"))
        .collect();
    sqlx::query(
        "INSERT INTO document_label (document_id, scheme, concept, score, evidence, origin, status, version_id)
         SELECT $1, u.scheme, u.concept, u.score, string_to_array(u.evidence, E'\\x1f'), 'auto', 'suggested', $2
         FROM unnest($3::text[], $4::text[], $5::real[], $6::text[]) AS u(scheme, concept, score, evidence)
         ON CONFLICT (document_id, scheme, concept) DO NOTHING",
    )
    .bind(document_id)
    .bind(version_id)
    .bind(suggestions.iter().map(|s| s.scheme.as_str()).collect::<Vec<_>>())
    .bind(suggestions.iter().map(|s| s.concept.as_str()).collect::<Vec<_>>())
    .bind(suggestions.iter().map(|s| s.score.clamp(0.0, 1.0)).collect::<Vec<f32>>())
    .bind(&evidence)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// What classification looks at in a stored document: the same title and kept text the crawler
/// classified, and the automatic suggestions it made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classifiable {
    /// Document ID.
    pub document_id: i64,
    /// Its current version.
    pub version_id: Option<i64>,
    /// Title (may be empty).
    pub title: String,
    /// The kept text; `None` where only metadata may be kept.
    pub body: Option<String>,
    /// Language code.
    pub language: Option<String>,
    /// The automatic suggestions now: `(scheme, concept)` in order.
    pub suggested: Vec<(String, String)>,
}

impl Store {
    /// Documents after ID `after` in ID order, at most `limit`, with what classification needs.
    pub async fn classifiable(&self, after: i64, limit: i64) -> Result<Vec<Classifiable>> {
        let rows = sqlx::query(
            "SELECT d.id, d.current_version_id, COALESCE(v.title, d.title, '') AS title, v.body, d.language,
                    COALESCE((SELECT array_agg(l.scheme || '/' || l.concept ORDER BY l.scheme, l.concept)
                              FROM document_label l
                              WHERE l.document_id = d.id AND l.origin = 'auto' AND l.status = 'suggested'),
                             '{}') AS suggested
             FROM document d
             LEFT JOIN document_version v ON v.id = d.current_version_id
             WHERE d.id > $1
             ORDER BY d.id
             LIMIT $2",
        )
        .bind(after)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                let suggested: Vec<String> = r.try_get("suggested")?;
                Ok(Classifiable {
                    document_id: r.try_get("id")?,
                    version_id: r.try_get("current_version_id")?,
                    title: r.try_get("title")?,
                    body: r.try_get("body")?,
                    language: r.try_get("language")?,
                    suggested: suggested
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

    /// Replaces the automatic suggestions of a document (decided labels stay).
    pub async fn replace_suggestions(
        &self,
        document_id: i64,
        version_id: Option<i64>,
        suggestions: &[LabelSuggestion],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        write_suggestions(&mut tx, document_id, version_id, suggestions).await?;
        tx.commit().await?;
        Ok(())
    }

    /// The links of a document (videos first), as last saved.
    pub async fn links(&self, document_id: i64) -> Result<Vec<DocumentLink>> {
        let rows = sqlx::query(
            "SELECT url, text, kind FROM document_link WHERE document_id = $1
             ORDER BY kind = 'link', url",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(DocumentLink {
                    url: r.try_get("url")?,
                    text: r.try_get::<Option<String>, _>("text")?.unwrap_or_default(),
                    kind: LinkKind::parse(r.try_get("kind")?),
                })
            })
            .collect()
    }

    /// The document of an endpoint, if it has one.
    pub async fn current_document(&self, endpoint_id: i64) -> Result<Option<CurrentDocument>> {
        let row = sqlx::query(
            "SELECT d.id, d.version_count, d.last_raw_hash, d.last_fingerprint, d.status, d.tdm_reserved, v.body,
                    v.id AS version_id, EXISTS (SELECT 1 FROM version_file f WHERE f.version_id = v.id) AS has_file
             FROM document d LEFT JOIN document_version v ON v.id = d.current_version_id
             WHERE d.endpoint_id = $1",
        )
        .bind(endpoint_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            Ok(CurrentDocument {
                id: r.try_get("id")?,
                version_count: r.try_get("version_count")?,
                last_raw_hash: hash_from(r.try_get("last_raw_hash")?)?,
                last_fingerprint: hash_from(r.try_get("last_fingerprint")?)?,
                body: r.try_get("body")?,
                status: r.try_get("status")?,
                tdm_reserved: r.try_get("tdm_reserved")?,
                version_id: r.try_get("version_id")?,
                has_file: r.try_get("has_file")?,
            })
        })
        .transpose()
    }

    /// Records a capture whose text did not change. Returns `false` when the endpoint has no document.
    pub async fn touch_document(&self, capture: &Capture) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let r = &capture.rights;
        let row = sqlx::query(
            "UPDATE document d SET
                 url = $2, last_seen_at = $3, last_raw_hash = $4, last_fingerprint = $5, status = 'active', gone_at = NULL,
                 tdm_reserved = $6, tdm_policy = $7, noindex = $8, noarchive = $9, nosnippet = $10, notranslate = $11
             FROM document old
             WHERE d.endpoint_id = $1 AND old.id = d.id
             RETURNING d.id, old.tdm_reserved AS was_reserved",
        )
        .bind(capture.endpoint_id)
        .bind(&capture.url)
        .bind(capture.fetched_at.to_sqlx())
        .bind(&capture.raw_hash[..])
        .bind(&capture.fingerprint[..])
        .bind(r.tdm_reserved)
        .bind(&r.tdm_policy)
        .bind(r.noindex)
        .bind(r.noarchive)
        .bind(r.nosnippet)
        .bind(r.notranslate)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        if r.tdm_reserved && !row.try_get::<bool, _>("was_reserved")? {
            forget_content(&mut tx, row.try_get("id")?).await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Saves a capture with new text as the next version, with the document's links and the
    /// classification suggestions of this version. Editors' decisions are kept.
    pub async fn save_version(
        &self,
        capture: &Capture,
        version: &NewVersion,
        links: &[DocumentLink],
        suggestions: &[LabelSuggestion],
    ) -> Result<SavedVersion> {
        let mut tx = self.pool.begin().await?;
        let r = &capture.rights;
        let doc = sqlx::query(
            "INSERT INTO document (endpoint_id, source_id, url, format, first_seen_at, last_seen_at, last_changed_at,
                                   tdm_reserved, tdm_policy, noindex, noarchive, nosnippet, notranslate)
             VALUES ($1, $2, $3, $4, $5, $5, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (endpoint_id) DO UPDATE SET
                 url = EXCLUDED.url, format = EXCLUDED.format, status = 'active', gone_at = NULL,
                 last_seen_at = EXCLUDED.last_seen_at, last_changed_at = EXCLUDED.last_changed_at,
                 tdm_reserved = EXCLUDED.tdm_reserved, tdm_policy = EXCLUDED.tdm_policy,
                 noindex = EXCLUDED.noindex, noarchive = EXCLUDED.noarchive,
                 nosnippet = EXCLUDED.nosnippet, notranslate = EXCLUDED.notranslate
             RETURNING id, version_count",
        )
        .bind(capture.endpoint_id)
        .bind(&capture.source_id)
        .bind(&capture.url)
        .bind(&capture.format)
        .bind(capture.fetched_at.to_sqlx())
        .bind(r.tdm_reserved)
        .bind(&r.tdm_policy)
        .bind(r.noindex)
        .bind(r.noarchive)
        .bind(r.nosnippet)
        .bind(r.notranslate)
        .fetch_one(&mut *tx)
        .await?;
        let document_id: i64 = doc.try_get("id")?;
        let version_no = doc.try_get::<i32, _>("version_count")? + 1;
        if r.tdm_reserved {
            forget_content(&mut tx, document_id).await?;
        }
        let body = if r.tdm_reserved {
            None
        } else {
            version.body.as_deref()
        };

        let version_id: i64 = sqlx::query_scalar(
            "INSERT INTO document_version (
                 document_id, version_no, fetched_at, change_kind, paragraphs_added, paragraphs_removed,
                 numbers_changed, raw_hash, text_fingerprint, content_type, format, encoding, title, description,
                 body, body_truncated, language, language_source, language_confidence, authors,
                 published_at, modified_at, canonical_url)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
             RETURNING id",
        )
        .bind(document_id)
        .bind(version_no)
        .bind(capture.fetched_at.to_sqlx())
        .bind(&version.change_kind)
        .bind(version.paragraphs_added)
        .bind(version.paragraphs_removed)
        .bind(version.numbers_changed)
        .bind(&capture.raw_hash[..])
        .bind(&capture.fingerprint[..])
        .bind(&version.content_type)
        .bind(&capture.format)
        .bind(&version.encoding)
        .bind(&version.title)
        .bind(&version.description)
        .bind(body)
        .bind(version.body_truncated && body.is_some())
        .bind(&version.language)
        .bind(&version.language_source)
        .bind(version.language_confidence)
        .bind(&version.authors)
        .bind(version.published_at.map(ToSqlx::to_sqlx))
        .bind(version.modified_at.map(ToSqlx::to_sqlx))
        .bind(&version.canonical_url)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE document SET current_version_id = $2, version_count = $3, title = $4, language = $5,
                 published_at = $6, modified_at = $7, last_raw_hash = $8, last_fingerprint = $9
             WHERE id = $1",
        )
        .bind(document_id)
        .bind(version_id)
        .bind(version_no)
        .bind(&version.title)
        .bind(&version.language)
        .bind(version.published_at.map(ToSqlx::to_sqlx))
        .bind(version.modified_at.map(ToSqlx::to_sqlx))
        .bind(&capture.raw_hash[..])
        .bind(&capture.fingerprint[..])
        .execute(&mut *tx)
        .await?;

        if let Some(file) = version.file.as_ref().filter(|_| !r.tdm_reserved) {
            insert_file(&mut tx, version_id, file).await?;
        }

        if !r.tdm_reserved {
            sqlx::query("DELETE FROM document_link WHERE document_id = $1")
                .bind(document_id)
                .execute(&mut *tx)
                .await?;
            // a video that is also an ordinary link is recorded once, as a video
            let mut ordered: Vec<&DocumentLink> = links.iter().collect();
            ordered.sort_by_key(|l| l.kind != LinkKind::Video);
            sqlx::query(
                "INSERT INTO document_link (document_id, url, text, kind)
                 SELECT $1, u.url, u.text, u.kind
                 FROM unnest($2::text[], $3::text[], $4::text[]) WITH ORDINALITY AS u(url, text, kind, n)
                 ORDER BY u.n
                 ON CONFLICT DO NOTHING",
            )
            .bind(document_id)
            .bind(ordered.iter().map(|l| l.url.as_str()).collect::<Vec<_>>())
            .bind(ordered.iter().map(|l| l.text.as_str()).collect::<Vec<_>>())
            .bind(ordered.iter().map(|l| l.kind.as_str()).collect::<Vec<_>>())
            .execute(&mut *tx)
            .await?;
        }

        write_suggestions(&mut tx, document_id, Some(version_id), suggestions).await?;

        tx.commit().await?;
        Ok(SavedVersion {
            document_id,
            version_id,
            version_no,
        })
    }

    /// Records that the endpoint's document was confirmed unchanged by the server (`304`).
    pub async fn mark_seen(&self, endpoint_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE document SET last_seen_at = now(), status = 'active', gone_at = NULL WHERE endpoint_id = $1",
        )
        .bind(endpoint_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Archives the original file with the document's current version when it has none yet (a
    /// document captured before files were archived). Returns whether it was stored.
    pub async fn attach_file(&self, document_id: i64, file: &NewFile) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let version: Option<i64> = sqlx::query_scalar(
            "SELECT current_version_id FROM document WHERE id = $1 AND NOT tdm_reserved",
        )
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let stored = match version {
            Some(version_id) => insert_file(&mut tx, version_id, file).await?,
            None => false,
        };
        tx.commit().await?;
        Ok(stored)
    }

    /// The newest archived file of a document, without its bytes.
    pub async fn archived_file(&self, document_id: i64) -> Result<Option<ArchivedFile>> {
        let row = sqlx::query(
            "SELECT f.version_id, f.media_type, f.byte_size, f.raw_hash, f.stored_at
             FROM version_file f JOIN document_version v ON v.id = f.version_id
             WHERE v.document_id = $1 ORDER BY v.version_no DESC LIMIT 1",
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| archived_from(&r)).transpose()
    }

    /// The newest archived file of a document with its bytes.
    pub async fn archived_content(
        &self,
        document_id: i64,
    ) -> Result<Option<(ArchivedFile, Vec<u8>)>> {
        let row = sqlx::query(
            "SELECT f.version_id, f.media_type, f.byte_size, f.raw_hash, f.stored_at, f.content
             FROM version_file f JOIN document_version v ON v.id = f.version_id
             WHERE v.document_id = $1 ORDER BY v.version_no DESC LIMIT 1",
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| Ok((archived_from(&r)?, r.try_get("content")?)))
            .transpose()
    }

    /// The rights of every document with archived files, to check them again after the register
    /// changed.
    pub async fn archive_rights(&self) -> Result<Vec<ArchiveRights>> {
        let rows = sqlx::query(
            "SELECT DISTINCT d.id, s.country, s.licence, s.terms, d.tdm_reserved, d.noarchive
             FROM version_file f
             JOIN document_version v ON v.id = f.version_id
             JOIN document d ON d.id = v.document_id
             JOIN source s ON s.id = d.source_id
             ORDER BY d.id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(ArchiveRights {
                    document_id: r.try_get("id")?,
                    country: r.try_get("country")?,
                    licence: r.try_get("licence")?,
                    terms: r.try_get("terms")?,
                    tdm_reserved: r.try_get("tdm_reserved")?,
                    noarchive: r.try_get("noarchive")?,
                })
            })
            .collect()
    }

    /// Deletes the archived files of documents; returns how many files were deleted.
    pub async fn delete_files(&self, document_ids: &[i64]) -> Result<u64> {
        Ok(sqlx::query(
            "DELETE FROM version_file WHERE version_id IN
                 (SELECT id FROM document_version WHERE document_id = ANY($1))",
        )
        .bind(document_ids)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Marks the endpoint's document as no longer available (HTTP 404/410); its history stays.
    pub async fn mark_gone(&self, endpoint_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE document SET status = 'gone', gone_at = COALESCE(gone_at, now()) WHERE endpoint_id = $1",
        )
            .bind(endpoint_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Records an editor's decision on a label (also on one that was never suggested).
    pub async fn decide_label(
        &self,
        document_id: i64,
        scheme: &str,
        concept: &str,
        decision: LabelDecision,
        editor: &str,
    ) -> Result<()> {
        let status = match decision {
            LabelDecision::Confirm => "confirmed",
            LabelDecision::Reject => "rejected",
        };
        sqlx::query(
            "INSERT INTO document_label (document_id, scheme, concept, score, origin, status, decided_by, decided_at)
             VALUES ($1, $2, $3, 1, 'editor', $4, $5, now())
             ON CONFLICT (document_id, scheme, concept) DO UPDATE SET
                 status = EXCLUDED.status, decided_by = EXCLUDED.decided_by,
                 decided_at = EXCLUDED.decided_at, updated_at = now()",
        )
        .bind(document_id)
        .bind(scheme)
        .bind(concept)
        .bind(status)
        .bind(editor)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The labels of a document.
    pub async fn labels(&self, document_id: i64) -> Result<Vec<Label>> {
        let rows = sqlx::query(
            "SELECT scheme, concept, score, origin, status FROM document_label
             WHERE document_id = $1 ORDER BY scheme, concept",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(Label {
                    scheme: r.try_get("scheme")?,
                    concept: r.try_get("concept")?,
                    score: r.try_get("score")?,
                    origin: r.try_get("origin")?,
                    status: r.try_get("status")?,
                })
            })
            .collect()
    }

    /// The version history of a document, newest first.
    pub async fn versions(&self, document_id: i64) -> Result<Vec<VersionSummary>> {
        let rows = sqlx::query(
            "SELECT id, version_no, fetched_at, change_kind, paragraphs_added, paragraphs_removed,
                    numbers_changed, title, body IS NOT NULL AS has_body
             FROM document_version WHERE document_id = $1 ORDER BY version_no DESC",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(VersionSummary {
                    id: r.try_get("id")?,
                    version_no: r.try_get("version_no")?,
                    fetched_at: r
                        .try_get::<jiff_sqlx::Timestamp, _>("fetched_at")?
                        .to_jiff(),
                    change_kind: r.try_get("change_kind")?,
                    paragraphs_added: r.try_get("paragraphs_added")?,
                    paragraphs_removed: r.try_get("paragraphs_removed")?,
                    numbers_changed: r.try_get("numbers_changed")?,
                    title: r.try_get("title")?,
                    has_body: r.try_get("has_body")?,
                })
            })
            .collect()
    }

    /// The text of a version, if kept.
    pub async fn version_body(&self, version_id: i64) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT body FROM document_version WHERE id = $1")
                .bind(version_id)
                .fetch_optional(&self.pool)
                .await?
                .flatten(),
        )
    }
}

fn archived_from(r: &sqlx::postgres::PgRow) -> Result<ArchivedFile> {
    Ok(ArchivedFile {
        version_id: r.try_get("version_id")?,
        media_type: r.try_get("media_type")?,
        byte_size: i64::from(r.try_get::<i32, _>("byte_size")?),
        raw_hash: hash_from(r.try_get("raw_hash")?)?
            .ok_or_else(|| StoreError::Invalid("archived file without hash".into()))?,
        stored_at: r.try_get::<jiff_sqlx::Timestamp, _>("stored_at")?.to_jiff(),
    })
}
