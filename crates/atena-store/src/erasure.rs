//! Erasure on request (GDPR Art. 17) and the block list that keeps erased URLs from returning.
//!
//! [`Store::forget`] removes everything ATENA holds about a URL (or every URL below a prefix):
//! documents with all versions, labels and links, the endpoints and their check log, links to it
//! in other documents, and original files nothing else refers to. The URL goes on the block list,
//! so neither the register, nor a feed, a sitemap or a link can bring it back. The erasure itself
//! is logged without any of the removed content.

use crate::{Hash, Result, Store, StoreError};

/// What an erasure removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ErasureReport {
    /// Documents removed.
    pub documents: u64,
    /// Versions removed with them.
    pub versions: u64,
    /// Original files removed from the file store.
    pub files: u64,
}

impl Store {
    /// Erases `url` (with `prefix`, every URL that starts with it) and blocks it for good.
    pub async fn forget(&self, url: &str, prefix: bool, reason: &str) -> Result<ErasureReport> {
        if reason.trim().is_empty() {
            return Err(StoreError::Invalid("an erasure needs a reason".into()));
        }
        let matches =
            |column: &str| format!("({column} = $1 OR ($2 AND starts_with({column}, $1)))");
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO blocked_url (url, prefix, reason) VALUES ($1, $2, $3)
             ON CONFLICT (url) DO UPDATE SET prefix = blocked_url.prefix OR EXCLUDED.prefix, reason = EXCLUDED.reason",
        )
        .bind(url)
        .bind(prefix)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        // documents of matching endpoints, and documents that ended up at a matching URL by redirect
        let found: Vec<(i64, i64)> = sqlx::query_as(&format!(
            "SELECT d.id, d.endpoint_id FROM document d JOIN endpoint e ON e.id = d.endpoint_id
             WHERE {} OR {}",
            matches("e.url"),
            matches("d.url")
        ))
        .bind(url)
        .bind(prefix)
        .fetch_all(&mut *tx)
        .await?;
        let documents: Vec<i64> = found.iter().map(|(d, _)| *d).collect();
        let their_endpoints: Vec<i64> = found.iter().map(|(_, e)| *e).collect();
        let hashes: Vec<Vec<u8>> = sqlx::query_scalar(
            "SELECT DISTINCT raw_hash FROM document_version WHERE document_id = ANY($1)",
        )
        .bind(&documents)
        .fetch_all(&mut *tx)
        .await?;
        let versions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM document_version WHERE document_id = ANY($1)")
                .bind(&documents)
                .fetch_one(&mut *tx)
                .await?;
        sqlx::query("DELETE FROM document WHERE id = ANY($1)")
            .bind(&documents)
            .execute(&mut *tx)
            .await?;
        // endpoints go too (their check log with them); the block list keeps them from coming back
        sqlx::query(&format!(
            "DELETE FROM endpoint e WHERE {} OR e.id = ANY($3)",
            matches("e.url")
        ))
        .bind(url)
        .bind(prefix)
        .bind(&their_endpoints)
        .execute(&mut *tx)
        .await?;
        sqlx::query(&format!(
            "DELETE FROM document_link WHERE {}",
            matches("url")
        ))
        .bind(url)
        .bind(prefix)
        .execute(&mut *tx)
        .await?;
        let orphans: Vec<Vec<u8>> = sqlx::query_scalar(
            "DELETE FROM blob b WHERE b.hash = ANY($1)
                 AND NOT EXISTS (SELECT 1 FROM document_version v WHERE v.raw_hash = b.hash)
             RETURNING b.hash",
        )
        .bind(&hashes)
        .fetch_all(&mut *tx)
        .await?;
        let report = ErasureReport {
            documents: documents.len() as u64,
            versions: u64::try_from(versions).unwrap_or_default(),
            files: orphans.len() as u64,
        };
        sqlx::query(
            "INSERT INTO erasure_log (url, prefix, reason, documents, versions, files) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(url)
        .bind(prefix)
        .bind(reason)
        .bind(i32::try_from(report.documents).unwrap_or(i32::MAX))
        .bind(i32::try_from(report.versions).unwrap_or(i32::MAX))
        .bind(i32::try_from(report.files).unwrap_or(i32::MAX))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // files go after the commit: a failed transaction must not leave rows without files
        for raw in orphans {
            let Ok(hash) = Hash::try_from(raw.as_slice()) else {
                continue;
            };
            match std::fs::remove_file(self.blobs.path(&hash)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(report)
    }

    /// `true` when the URL is on the block list.
    pub async fn is_blocked(&self, url: &str) -> Result<bool> {
        Ok(
            sqlx::query_scalar(&format!("SELECT {}", crate::schedule::blocked("$1")))
                .bind(url)
                .fetch_one(&self.pool)
                .await?,
        )
    }
}
