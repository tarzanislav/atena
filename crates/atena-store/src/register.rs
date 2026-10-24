//! Keeping the `source` table and the register endpoints in line with `data/sources.yaml`.

use crate::schedule::{NewEndpoint, blocked, insert_hosts, wake_hosts};
use crate::{Result, Store};

/// A source as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    /// Register ID (`SK01`).
    pub id: String,
    /// Publisher name.
    pub name: String,
    /// Entry address.
    pub url: String,
    /// Country concept.
    pub country: String,
    /// Content languages.
    pub languages: Vec<String>,
    /// Authority concept (A1…E).
    pub authority: String,
    /// Rollout priority.
    pub priority: String,
    /// Connector kind.
    pub connector: String,
    /// Check interval in seconds.
    pub interval_s: i32,
    /// `official`, `copyrighted` or an open licence (SPDX identifier).
    pub licence: String,
    /// `allowed`, `forbidden` or `unknown`.
    pub terms: String,
    /// `false` for sources that must not be crawled.
    pub enabled: bool,
}

/// A source with the endpoints the register defines for it (its page, its feed…).
#[derive(Debug, Clone)]
pub struct RegisterSource {
    /// The source.
    pub source: SourceRecord,
    /// Its register endpoints (origin `register`).
    pub endpoints: Vec<NewEndpoint>,
}

/// What a synchronisation changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Sources in the register.
    pub sources: u64,
    /// Sources disabled because they left the register.
    pub removed: u64,
    /// Register endpoints created.
    pub endpoints_added: u64,
    /// Register endpoints disabled because the register no longer lists them.
    pub endpoints_disabled: u64,
}

impl Store {
    /// Makes the database match the register: sources are inserted or updated, sources missing
    /// from the register are disabled, register endpoints are created, updated or disabled.
    /// Endpoints found later (feed items, links) are left alone.
    pub async fn sync_register(&self, register: &[RegisterSource]) -> Result<SyncReport> {
        let mut tx = self.pool.begin().await?;
        let mut report = SyncReport::default();
        for entry in register {
            let s = &entry.source;
            sqlx::query(
                "INSERT INTO source (id, name, url, country, languages, authority, priority, connector, interval_s,
                                     licence, terms, enabled)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 ON CONFLICT (id) DO UPDATE SET
                     name = EXCLUDED.name, url = EXCLUDED.url, country = EXCLUDED.country,
                     languages = EXCLUDED.languages, authority = EXCLUDED.authority,
                     priority = EXCLUDED.priority, connector = EXCLUDED.connector,
                     interval_s = EXCLUDED.interval_s, licence = EXCLUDED.licence, terms = EXCLUDED.terms,
                     enabled = EXCLUDED.enabled, synced_at = now()",
            )
            .bind(&s.id)
            .bind(&s.name)
            .bind(&s.url)
            .bind(&s.country)
            .bind(&s.languages)
            .bind(&s.authority)
            .bind(&s.priority)
            .bind(&s.connector)
            .bind(s.interval_s)
            .bind(&s.licence)
            .bind(&s.terms)
            .bind(s.enabled)
            .execute(&mut *tx)
            .await?;
            report.sources += 1;

            let hosts =
                insert_hosts(&mut tx, entry.endpoints.iter().map(|e| e.host.as_str())).await?;
            let urls: Vec<&str> = entry.endpoints.iter().map(|e| e.url.as_str()).collect();
            for ep in &entry.endpoints {
                // a URL found earlier as a feed item or link becomes a register endpoint;
                // a blocked URL (erasure request) is never added or re-enabled
                let inserted: Option<bool> = sqlx::query_scalar(&format!(
                    "INSERT INTO endpoint (source_id, host_id, url, kind, origin, follow_links, interval_s)
                     SELECT $1, (SELECT id FROM host WHERE name = $2), $3, $4, 'register', $5, $6
                     WHERE NOT {BLOCKED}
                     ON CONFLICT (url) DO UPDATE SET
                         source_id = EXCLUDED.source_id, kind = EXCLUDED.kind, origin = 'register',
                         follow_links = EXCLUDED.follow_links, interval_s = EXCLUDED.interval_s, enabled = true
                     RETURNING (xmax = 0)",
                    BLOCKED = blocked("$3")
                ))
                .bind(&s.id)
                .bind(&ep.host)
                .bind(&ep.url)
                .bind(ep.kind.as_str())
                .bind(ep.follow_links)
                .bind(ep.interval_s)
                .fetch_optional(&mut *tx)
                .await?;
                report.endpoints_added += u64::from(inserted.unwrap_or(false));
            }
            let disabled = sqlx::query(
                "UPDATE endpoint SET enabled = false
                 WHERE source_id = $1 AND origin = 'register' AND enabled AND NOT (url = ANY($2))",
            )
            .bind(&s.id)
            .bind(&urls)
            .execute(&mut *tx)
            .await?;
            report.endpoints_disabled += disabled.rows_affected();
            wake_hosts(&mut tx, &hosts).await?;
        }
        let ids: Vec<&str> = register.iter().map(|r| r.source.id.as_str()).collect();
        let removed =
            sqlx::query("UPDATE source SET enabled = false WHERE enabled AND NOT (id = ANY($1))")
                .bind(&ids)
                .execute(&mut *tx)
                .await?;
        report.removed = removed.rows_affected();
        tx.commit().await?;
        Ok(report)
    }
}
