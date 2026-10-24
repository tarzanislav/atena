//! Sources proposed by users, their rights assessments, and the lists of proposals exchanged with
//! other installations.
//!
//! An imported source is stored `proposed`: disabled, `copyrighted`, `unknown` – nothing is crawled
//! and nothing is kept. Its automatic assessment is stored beside it. Only [`Store::confirm_import`]
//! – a person's decision with a reason – gives it a licence, terms and, where the legality
//! principle allows, an enabled crawl.

use sqlx::Row;

use crate::schedule::{NewEndpoint, blocked, insert_hosts, wake_hosts};
use crate::{Result, Store, StoreError};

/// Where a proposal came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    /// A user of this installation.
    Local,
    /// A user of another installation, through the shared proposals (editor's installation).
    Shared,
}

impl Via {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Shared => "shared",
        }
    }
}

/// The state of an imported source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportState {
    /// Waiting for a person's decision; not crawled.
    Proposed,
    /// A person confirmed its licence and terms.
    Confirmed,
    /// A person rejected it; its content is removed.
    Rejected,
    /// The shared register took it over.
    Merged,
}

impl ImportState {
    /// The name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
            Self::Merged => "merged",
        }
    }

    pub(crate) fn parse(text: &str) -> Self {
        match text {
            "confirmed" => Self::Confirmed,
            "rejected" => Self::Rejected,
            "merged" => Self::Merged,
            _ => Self::Proposed,
        }
    }
}

/// A source to import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewImport {
    /// ID (`U-` and a fingerprint of the URL).
    pub id: String,
    /// Name (title of the page, or its host).
    pub name: String,
    /// Entry address.
    pub url: String,
    /// Country, if known.
    pub country: Option<String>,
    /// Connector kind.
    pub connector: String,
    /// Check interval in seconds.
    pub interval_s: i32,
    /// Where it came from.
    pub via: Via,
    /// Endpoints it will be crawled through once confirmed.
    pub endpoints: Vec<NewEndpoint>,
}

/// A rights assessment: automatic (`by_person: None`) or a person's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssessmentRecord {
    /// Who decided; `None` for the automatic assessment.
    pub by_person: Option<String>,
    /// `licence` value.
    pub licence: String,
    /// `terms` value.
    pub terms: String,
    /// Country.
    pub country: Option<String>,
    /// The page that was read.
    pub rights_page: String,
    /// What it says, or why the person decided so.
    pub finding: String,
    /// The automatic assessment as JSON.
    pub evidence: Option<String>,
}

/// What an import did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    /// A new proposed source.
    New,
    /// A proposed source got a new assessment.
    Reassessed,
    /// Already decided; the new assessment is kept for reference, nothing else changes.
    AlreadyDecided(ImportState),
    /// The register lists the address already (under this ID).
    InRegister(String),
    /// The address is blocked (erasure request).
    Blocked,
}

/// An imported source with its newest automatic assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSource {
    /// ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// Entry address.
    pub url: String,
    /// State.
    pub state: ImportState,
    /// Where it came from.
    pub via: Via,
    /// Country (empty when unknown).
    pub country: String,
    /// Connector.
    pub connector: String,
    /// Confirmed licence (`copyrighted` until confirmed).
    pub licence: String,
    /// Confirmed terms (`unknown` until confirmed).
    pub terms: String,
    /// Crawled.
    pub enabled: bool,
    /// The newest automatic proposal: licence, terms, country, page and finding.
    pub proposal: Option<AssessmentRecord>,
}

/// A list received from another installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxMessage {
    /// ID of the list.
    pub message_id: String,
    /// The lines.
    pub body: String,
}

/// A list waiting to be shared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxMessage {
    /// ID.
    pub id: i64,
    /// The lines.
    pub body: String,
    /// Attempts so far.
    pub attempts: i32,
}

async fn add_assessment(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    source_id: &str,
    a: &AssessmentRecord,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO assessment (source_id, by_person, licence, terms, country, rights_page, finding, evidence)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(source_id)
    .bind(&a.by_person)
    .bind(&a.licence)
    .bind(&a.terms)
    .bind(&a.country)
    .bind(&a.rights_page)
    .bind(&a.finding)
    .bind(&a.evidence)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl Store {
    /// Stores a proposed source with its automatic assessment. Existing decisions are never
    /// changed by an import.
    pub async fn import_source(
        &self,
        s: &NewImport,
        auto: &AssessmentRecord,
    ) -> Result<ImportOutcome> {
        if auto.by_person.is_some() {
            return Err(StoreError::Invalid(
                "an import carries the automatic assessment".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let is_blocked: bool = sqlx::query_scalar(&format!("SELECT {}", blocked("$1")))
            .bind(&s.url)
            .fetch_one(&mut *tx)
            .await?;
        if is_blocked {
            return Ok(ImportOutcome::Blocked);
        }
        let in_register: Option<String> = sqlx::query_scalar(
            "SELECT s.id FROM source s WHERE s.origin = 'register'
               AND (s.url = $1 OR EXISTS (SELECT 1 FROM endpoint e WHERE e.source_id = s.id AND e.url = $1))
             LIMIT 1",
        )
        .bind(&s.url)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = in_register {
            return Ok(ImportOutcome::InRegister(id));
        }
        let existing: Option<String> = sqlx::query_scalar("SELECT state FROM source WHERE id = $1")
            .bind(&s.id)
            .fetch_optional(&mut *tx)
            .await?;
        let outcome = match existing.as_deref().map(ImportState::parse) {
            None => {
                sqlx::query(
                    "INSERT INTO source (id, name, url, country, authority, priority, connector, interval_s,
                                         licence, terms, enabled, origin, state, via)
                     VALUES ($1, $2, $3, $4, 'D', 'P3', $5, $6, 'copyrighted', 'unknown', false, 'import', 'proposed', $7)",
                )
                .bind(&s.id)
                .bind(&s.name)
                .bind(&s.url)
                .bind(s.country.as_deref().unwrap_or(""))
                .bind(&s.connector)
                .bind(s.interval_s.max(60))
                .bind(s.via.as_str())
                .execute(&mut *tx)
                .await?;
                ImportOutcome::New
            }
            Some(ImportState::Proposed) => {
                sqlx::query(
                    "UPDATE source SET name = $2, country = COALESCE(NULLIF($3, ''), country), connector = $4,
                         interval_s = $5, synced_at = now() WHERE id = $1",
                )
                .bind(&s.id)
                .bind(&s.name)
                .bind(s.country.as_deref().unwrap_or(""))
                .bind(&s.connector)
                .bind(s.interval_s.max(60))
                .execute(&mut *tx)
                .await?;
                ImportOutcome::Reassessed
            }
            Some(state) => ImportOutcome::AlreadyDecided(state),
        };
        if matches!(outcome, ImportOutcome::New | ImportOutcome::Reassessed) {
            // the endpoints exist from the start, but the source is disabled until confirmed
            let hosts = insert_hosts(&mut tx, s.endpoints.iter().map(|e| e.host.as_str())).await?;
            for ep in &s.endpoints {
                sqlx::query(&format!(
                    "INSERT INTO endpoint (source_id, host_id, url, kind, origin, follow_links, interval_s)
                     SELECT $1, (SELECT id FROM host WHERE name = $2), $3, $4, $5, $6, $7
                     WHERE NOT {BLOCKED}
                     ON CONFLICT (url) DO NOTHING",
                    BLOCKED = blocked("$3")
                ))
                .bind(&s.id)
                .bind(&ep.host)
                .bind(&ep.url)
                .bind(ep.kind.as_str())
                .bind(ep.origin.as_str())
                .bind(ep.follow_links)
                .bind(ep.interval_s)
                .execute(&mut *tx)
                .await?;
            }
            wake_hosts(&mut tx, &hosts).await?;
        }
        add_assessment(&mut tx, &s.id, auto).await?;
        tx.commit().await?;
        Ok(outcome)
    }

    /// Imported sources, newest first; `state` narrows the list.
    pub async fn imported(&self, state: Option<ImportState>) -> Result<Vec<ImportedSource>> {
        let rows = sqlx::query(
            "SELECT s.id, s.name, s.url, s.state, s.via, s.country, s.connector, s.licence, s.terms, s.enabled,
                    a.licence AS p_licence, a.terms AS p_terms, a.country AS p_country,
                    a.rights_page AS p_page, a.finding AS p_finding, a.evidence AS p_evidence
             FROM source s
             LEFT JOIN LATERAL (SELECT * FROM assessment WHERE source_id = s.id AND by_person IS NULL
                                ORDER BY at DESC, id DESC LIMIT 1) a ON true
             WHERE s.origin = 'import' AND ($1::text IS NULL OR s.state = $1)
             ORDER BY s.synced_at DESC, s.id",
        )
        .bind(state.map(ImportState::as_str))
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                let proposal = match r.try_get::<Option<String>, _>("p_licence")? {
                    Some(licence) => Some(AssessmentRecord {
                        by_person: None,
                        licence,
                        terms: r.try_get("p_terms")?,
                        country: r.try_get("p_country")?,
                        rights_page: r.try_get("p_page")?,
                        finding: r.try_get("p_finding")?,
                        evidence: r.try_get("p_evidence")?,
                    }),
                    None => None,
                };
                Ok(ImportedSource {
                    id: r.try_get("id")?,
                    name: r.try_get("name")?,
                    url: r.try_get("url")?,
                    state: ImportState::parse(r.try_get("state")?),
                    via: if r.try_get::<String, _>("via")? == "shared" {
                        Via::Shared
                    } else {
                        Via::Local
                    },
                    country: r.try_get("country")?,
                    connector: r.try_get("connector")?,
                    licence: r.try_get("licence")?,
                    terms: r.try_get("terms")?,
                    enabled: r.try_get("enabled")?,
                    proposal,
                })
            })
            .collect()
    }

    /// Every assessment of a source, newest first.
    pub async fn assessments(&self, source_id: &str) -> Result<Vec<AssessmentRecord>> {
        let rows = sqlx::query(
            "SELECT by_person, licence, terms, country, rights_page, finding, evidence
             FROM assessment WHERE source_id = $1 ORDER BY at DESC, id DESC",
        )
        .bind(source_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(AssessmentRecord {
                    by_person: r.try_get("by_person")?,
                    licence: r.try_get("licence")?,
                    terms: r.try_get("terms")?,
                    country: r.try_get("country")?,
                    rights_page: r.try_get("rights_page")?,
                    finding: r.try_get("finding")?,
                    evidence: r.try_get("evidence")?,
                })
            })
            .collect()
    }

    /// A person's decision on an imported source: its licence, terms and country become the
    /// source's, and `enabled` (what the legality principle allows, decided by the caller) whether
    /// it is crawled. Returns `false` when there is no such imported source.
    pub async fn confirm_import(
        &self,
        source_id: &str,
        decision: &AssessmentRecord,
        enabled: bool,
    ) -> Result<bool> {
        let (Some(_), Some(country)) = (&decision.by_person, &decision.country) else {
            return Err(StoreError::Invalid(
                "a confirmation needs the person who decided and the country".into(),
            ));
        };
        if decision.finding.trim().is_empty() {
            return Err(StoreError::Invalid("a confirmation needs a reason".into()));
        }
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE source SET licence = $2, terms = $3, country = $4, enabled = $5, state = 'confirmed',
                 synced_at = now()
             WHERE id = $1 AND origin = 'import' AND state IN ('proposed', 'confirmed')",
        )
        .bind(source_id)
        .bind(&decision.licence)
        .bind(&decision.terms)
        .bind(country)
        .bind(enabled)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Ok(false);
        }
        add_assessment(&mut tx, source_id, decision).await?;
        let hosts: Vec<i64> =
            sqlx::query_scalar("SELECT DISTINCT host_id FROM endpoint WHERE source_id = $1")
                .bind(source_id)
                .fetch_all(&mut *tx)
                .await?;
        wake_hosts(&mut tx, &hosts).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// A person rejects an imported source: it is disabled and everything collected from it is
    /// removed. Returns `false` when there is no such imported source.
    pub async fn reject_import(
        &self,
        source_id: &str,
        by_person: &str,
        reason: &str,
    ) -> Result<bool> {
        if reason.trim().is_empty() {
            return Err(StoreError::Invalid("a rejection needs a reason".into()));
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, String)> = sqlx::query_as(
            "UPDATE source SET state = 'rejected', enabled = false, synced_at = now()
             WHERE id = $1 AND origin = 'import' RETURNING licence, url",
        )
        .bind(source_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((licence, url)) = row else {
            return Ok(false);
        };
        add_assessment(
            &mut tx,
            source_id,
            &AssessmentRecord {
                by_person: Some(by_person.to_string()),
                licence,
                terms: "forbidden".into(),
                country: None,
                rights_page: url,
                finding: reason.to_string(),
                evidence: None,
            },
        )
        .await?;
        tx.commit().await?;
        self.purge_source(source_id).await?;
        Ok(true)
    }

    /// Stores a list received from another installation; `false` when it was received before.
    pub async fn inbox_add(&self, message_id: &str, body: &str, items: i32) -> Result<bool> {
        let inserted = sqlx::query(
            "INSERT INTO inbox (message_id, body, items) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(message_id)
        .bind(body)
        .bind(items)
        .execute(&self.pool)
        .await?;
        Ok(inserted.rows_affected() == 1)
    }

    /// Message IDs already received.
    pub async fn inbox_known(&self, message_ids: &[String]) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT message_id FROM inbox WHERE message_id = ANY($1)")
                .bind(message_ids)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Queues a list for sharing.
    pub async fn outbox_add(&self, body: &str) -> Result<i64> {
        Ok(
            sqlx::query_scalar("INSERT INTO outbox (body) VALUES ($1) RETURNING id")
                .bind(body)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Lists waiting to be shared, oldest first.
    pub async fn outbox_pending(&self) -> Result<Vec<OutboxMessage>> {
        let rows: Vec<(i64, String, i32)> =
            sqlx::query_as("SELECT id, body, attempts FROM outbox ORDER BY id")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|(id, body, attempts)| OutboxMessage { id, body, attempts })
            .collect())
    }

    /// A list was shared: it is removed.
    pub async fn outbox_sent(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM outbox WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Sharing a list failed; it stays queued.
    pub async fn outbox_failed(&self, id: i64, error: &str) -> Result<()> {
        sqlx::query("UPDATE outbox SET attempts = attempts + 1, last_error = $2 WHERE id = $1")
            .bind(id)
            .bind(error)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
