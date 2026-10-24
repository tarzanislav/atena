//! Endpoints, hosts and leases: what is checked when, and by whom.

use std::time::Duration;

use sqlx::{Postgres, Row, Transaction};

use crate::{Result, Store, StoreError, secs};

/// What an endpoint is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointKind {
    /// RSS or Atom feed; its items become pages.
    Feed,
    /// Sitemap or sitemap index; its URLs become pages.
    Sitemap,
    /// A page or file; it becomes a document.
    Page,
}

impl EndpointKind {
    /// The name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feed => "feed",
            Self::Sitemap => "sitemap",
            Self::Page => "page",
        }
    }

    fn parse(text: &str) -> Result<Self> {
        match text {
            "feed" => Ok(Self::Feed),
            "sitemap" => Ok(Self::Sitemap),
            "page" => Ok(Self::Page),
            other => Err(StoreError::Invalid(format!(
                "unknown endpoint kind {other}"
            ))),
        }
    }
}

/// How ATENA learned about an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Listed in the source register.
    Register,
    /// An item of a feed.
    Feed,
    /// An entry of a sitemap.
    Sitemap,
    /// A link on a followed page.
    Link,
}

impl Origin {
    /// The name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Feed => "feed",
            Self::Sitemap => "sitemap",
            Self::Link => "link",
        }
    }

    fn parse(text: &str) -> Result<Self> {
        match text {
            "register" => Ok(Self::Register),
            "feed" => Ok(Self::Feed),
            "sitemap" => Ok(Self::Sitemap),
            "link" => Ok(Self::Link),
            other => Err(StoreError::Invalid(format!(
                "unknown endpoint origin {other}"
            ))),
        }
    }
}

/// An endpoint to create; the URL must already be normalised and `host` lower-case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEndpoint {
    /// Normalised URL.
    pub url: String,
    /// Host name of the URL.
    pub host: String,
    /// What it is.
    pub kind: EndpointKind,
    /// How it was found.
    pub origin: Origin,
    /// Follow attachments and subpages.
    pub follow_links: bool,
    /// Check interval in seconds.
    pub interval_s: i32,
}

/// An endpoint due for a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// ID.
    pub id: i64,
    /// Source ID.
    pub source_id: String,
    /// Host ID.
    pub host_id: i64,
    /// URL.
    pub url: String,
    /// What it is.
    pub kind: EndpointKind,
    /// How it was found.
    pub origin: Origin,
    /// Follow attachments and subpages.
    pub follow_links: bool,
    /// Check interval in seconds.
    pub interval_s: i32,
    /// Check interval of the source.
    pub source_interval_s: i32,
    /// Authority level of the source (`A1`…`D`).
    pub source_authority: String,
    /// Licence of the source's content.
    pub source_licence: String,
    /// Country of the source.
    pub source_country: String,
    /// Terms of use of the source on automated access.
    pub source_terms: String,
    /// Failed checks in a row.
    pub fail_count: i32,
    /// `ETag` of the last full response.
    pub etag: Option<String>,
    /// `Last-Modified` of the last full response.
    pub last_modified: Option<String>,
}

/// A leased host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLease {
    /// Host ID.
    pub id: i64,
    /// Host name.
    pub name: String,
}

/// The result of one check, recorded with [`Store::record_check`].
#[derive(Debug, Clone, Default)]
pub struct Check {
    /// Short outcome (`new`, `changed`, `unchanged`, `not_modified`, `robots`, `http_error`…).
    pub outcome: String,
    /// HTTP status, if a response arrived.
    pub status: Option<u16>,
    /// Size of the body.
    pub bytes: Option<u64>,
    /// Time the check took.
    pub elapsed: Duration,
    /// Error message or other detail.
    pub detail: Option<String>,
    /// When to check again, from now.
    pub next_in: Duration,
    /// `true` when the check failed (counts towards back-off).
    pub failed: bool,
    /// New validators (`ETag`, `Last-Modified`) when a full response arrived.
    pub validators: Option<(Option<String>, Option<String>)>,
    /// Stop checking this endpoint.
    pub disable: bool,
}

const MAX_DETAIL: usize = 2_000;

/// SQL condition: the URL expression is on the block list (exactly, or below a blocked prefix).
pub(crate) fn blocked(url: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM blocked_url b WHERE b.url = {url} OR (b.prefix AND starts_with({url}, b.url)))"
    )
}

/// Creates missing hosts; returns the IDs of all named hosts.
pub(crate) async fn insert_hosts<'a>(
    tx: &mut Transaction<'_, Postgres>,
    names: impl Iterator<Item = &'a str>,
) -> Result<Vec<i64>> {
    let mut names: Vec<&str> = names.collect();
    names.sort_unstable();
    names.dedup();
    if names.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query("INSERT INTO host (name) SELECT unnest($1::text[]) ON CONFLICT (name) DO NOTHING")
        .bind(&names)
        .execute(&mut **tx)
        .await?;
    Ok(
        sqlx::query_scalar("SELECT id FROM host WHERE name = ANY($1)")
            .bind(&names)
            .fetch_all(&mut **tx)
            .await?,
    )
}

/// Recomputes when the hosts are next due: their earliest enabled endpoint, not before a back-off.
pub(crate) async fn wake_hosts(tx: &mut Transaction<'_, Postgres>, ids: &[i64]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "UPDATE host h SET next_due_at = (
             SELECT CASE WHEN m.due IS NULL THEN NULL ELSE GREATEST(m.due, COALESCE(h.backoff_until, m.due)) END
             FROM (SELECT min(e.next_fetch_at) AS due
                   FROM endpoint e JOIN source s ON s.id = e.source_id
                   WHERE e.host_id = h.id AND e.enabled AND s.enabled) m)
         WHERE h.id = ANY($1)",
    )
    .bind(ids)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn endpoint_from(row: &sqlx::postgres::PgRow) -> Result<Endpoint> {
    Ok(Endpoint {
        id: row.try_get("id")?,
        source_id: row.try_get("source_id")?,
        host_id: row.try_get("host_id")?,
        url: row.try_get("url")?,
        kind: EndpointKind::parse(row.try_get("kind")?)?,
        origin: Origin::parse(row.try_get("origin")?)?,
        follow_links: row.try_get("follow_links")?,
        interval_s: row.try_get("interval_s")?,
        source_interval_s: row.try_get("source_interval_s")?,
        source_authority: row.try_get("source_authority")?,
        source_licence: row.try_get("source_licence")?,
        source_country: row.try_get("source_country")?,
        source_terms: row.try_get("source_terms")?,
        fail_count: row.try_get("fail_count")?,
        etag: row.try_get("etag")?,
        last_modified: row.try_get("last_modified")?,
    })
}

const ENDPOINT_COLUMNS: &str = "e.id, e.source_id, e.host_id, e.url, e.kind, e.origin, e.follow_links, \
     e.interval_s, s.interval_s AS source_interval_s, s.authority AS source_authority, \
     s.licence AS source_licence, s.country AS source_country, s.terms AS source_terms, \
     e.fail_count, e.etag, e.last_modified";

/// Most addresses one source may have (register entries, feeds, sitemaps and discovered pages).
pub const MAX_ENDPOINTS_PER_SOURCE: i64 = 100_000;

impl Store {
    /// Adds endpoints found on `discovered_from` (feed items, sitemap entries, links); URLs that
    /// are already known are skipped. Returns how many were new.
    pub async fn add_endpoints(
        &self,
        source_id: &str,
        discovered_from: Option<i64>,
        endpoints: &[NewEndpoint],
    ) -> Result<u64> {
        if endpoints.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        // no source grows without bound, however many sitemaps or links it offers
        let held: i64 = sqlx::query_scalar("SELECT count(*) FROM endpoint WHERE source_id = $1")
            .bind(source_id)
            .fetch_one(&mut *tx)
            .await?;
        let room = usize::try_from(MAX_ENDPOINTS_PER_SOURCE - held).unwrap_or(0);
        let endpoints = &endpoints[..endpoints.len().min(room)];
        if endpoints.is_empty() {
            return Ok(0);
        }
        insert_hosts(&mut tx, endpoints.iter().map(|e| e.host.as_str())).await?;
        let column =
            |f: fn(&NewEndpoint) -> String| endpoints.iter().map(f).collect::<Vec<String>>();
        let hosts: Vec<i64> = sqlx::query_scalar(&format!(
            "INSERT INTO endpoint (source_id, host_id, url, kind, origin, follow_links, interval_s, discovered_from)
             SELECT $1, h.id, u.url, u.kind, u.origin, u.follow, u.interval_s, $2
             FROM unnest($3::text[], $4::text[], $5::text[], $6::text[], $7::bool[], $8::int[])
                  AS u(url, host, kind, origin, follow, interval_s)
             JOIN host h ON h.name = u.host
             WHERE NOT {BLOCKED}
             ON CONFLICT (url) DO NOTHING
             RETURNING host_id",
            BLOCKED = blocked("u.url")
        ))
        .bind(source_id)
        .bind(discovered_from)
        .bind(column(|e| e.url.clone()))
        .bind(column(|e| e.host.clone()))
        .bind(column(|e| e.kind.as_str().to_string()))
        .bind(column(|e| e.origin.as_str().to_string()))
        .bind(endpoints.iter().map(|e| e.follow_links).collect::<Vec<bool>>())
        .bind(endpoints.iter().map(|e| e.interval_s).collect::<Vec<i32>>())
        .fetch_all(&mut *tx)
        .await?;
        let added = hosts.len() as u64;
        let mut hosts = hosts;
        hosts.sort_unstable();
        hosts.dedup();
        wake_hosts(&mut tx, &hosts).await?;
        tx.commit().await?;
        Ok(added)
    }

    /// Leases up to `max` due hosts for `lease`; hosts leased by others are skipped.
    pub async fn lease_hosts(
        &self,
        worker: &str,
        max: i64,
        lease: Duration,
    ) -> Result<Vec<HostLease>> {
        let rows = sqlx::query(
            "UPDATE host SET lease_until = now() + make_interval(secs => $2), leased_by = $3
             WHERE id IN (
                 SELECT id FROM host
                 WHERE next_due_at <= now() AND (lease_until IS NULL OR lease_until < now())
                 ORDER BY next_due_at
                 LIMIT $1
                 FOR UPDATE SKIP LOCKED)
             RETURNING id, name",
        )
        .bind(max)
        .bind(secs(lease))
        .bind(worker)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(HostLease {
                    id: r.try_get("id")?,
                    name: r.try_get("name")?,
                })
            })
            .collect()
    }

    /// Due endpoints of a host, earliest first.
    pub async fn due_endpoints(&self, host_id: i64, limit: i64) -> Result<Vec<Endpoint>> {
        let rows = sqlx::query(&format!(
            "SELECT {ENDPOINT_COLUMNS}
             FROM endpoint e JOIN source s ON s.id = e.source_id
             WHERE e.host_id = $1 AND e.enabled AND s.enabled AND e.next_fetch_at <= now()
             ORDER BY e.next_fetch_at
             LIMIT $2"
        ))
        .bind(host_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(endpoint_from).collect()
    }

    /// An endpoint by ID.
    pub async fn endpoint(&self, id: i64) -> Result<Option<Endpoint>> {
        let row = sqlx::query(&format!(
            "SELECT {ENDPOINT_COLUMNS} FROM endpoint e JOIN source s ON s.id = e.source_id WHERE e.id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(endpoint_from).transpose()
    }

    /// Ends a lease. `backoff` keeps the host untouched for that long (after `429`/`503`).
    /// A lease that has meanwhile passed to another worker is left alone.
    pub async fn release_host(
        &self,
        host_id: i64,
        worker: &str,
        backoff: Option<Duration>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let released = sqlx::query(
            "UPDATE host SET lease_until = NULL, leased_by = NULL,
                 backoff_until = CASE WHEN $3::float8 IS NULL THEN backoff_until
                                      ELSE now() + make_interval(secs => $3) END
             WHERE id = $1 AND leased_by = $2",
        )
        .bind(host_id)
        .bind(worker)
        .bind(backoff.map(secs))
        .execute(&mut *tx)
        .await?;
        if released.rows_affected() > 0 {
            wake_hosts(&mut tx, &[host_id]).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Records a check: schedules the endpoint again and writes the check log.
    pub async fn record_check(&self, endpoint_id: i64, check: &Check) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let (new_validators, etag, last_modified) = match &check.validators {
            Some((etag, lm)) => (true, etag.clone(), lm.clone()),
            None => (false, None, None),
        };
        let status = check.status.map(|s| i16::try_from(s).unwrap_or(i16::MAX));
        let host_id: Option<i64> = sqlx::query_scalar(
            "UPDATE endpoint SET
                 last_fetch_at = now(), last_outcome = $2, last_status = $3,
                 next_fetch_at = now() + make_interval(secs => $4),
                 fail_count = CASE WHEN $5 THEN fail_count + 1 ELSE 0 END,
                 etag = CASE WHEN $6 THEN $7 ELSE etag END,
                 last_modified = CASE WHEN $6 THEN $8 ELSE last_modified END,
                 enabled = enabled AND NOT $9
             WHERE id = $1
             RETURNING host_id",
        )
        .bind(endpoint_id)
        .bind(&check.outcome)
        .bind(status)
        .bind(secs(check.next_in))
        .bind(check.failed)
        .bind(new_validators)
        .bind(etag)
        .bind(last_modified)
        .bind(check.disable)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(host_id) = host_id else {
            return Err(StoreError::Invalid(format!(
                "endpoint {endpoint_id} does not exist"
            )));
        };
        let detail = check
            .detail
            .as_ref()
            .map(|d| d.chars().take(MAX_DETAIL).collect::<String>());
        sqlx::query(
            "INSERT INTO fetch_log (endpoint_id, outcome, status, bytes, elapsed_ms, detail)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(endpoint_id)
        .bind(&check.outcome)
        .bind(status)
        .bind(check.bytes.map(|b| i64::try_from(b).unwrap_or(i64::MAX)))
        .bind(i32::try_from(check.elapsed.as_millis()).unwrap_or(i32::MAX))
        .bind(detail)
        .execute(&mut *tx)
        .await?;
        // keeps the host's due time right when no lease is held (e.g. a one-off check)
        let leased: bool = sqlx::query_scalar(
            "SELECT lease_until IS NOT NULL AND lease_until > now() FROM host WHERE id = $1",
        )
        .bind(host_id)
        .fetch_one(&mut *tx)
        .await?;
        if !leased {
            wake_hosts(&mut tx, &[host_id]).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
