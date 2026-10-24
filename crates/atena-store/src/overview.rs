//! What an installation holds, at a glance: for the overview and the list of sources of the
//! desktop app.

use jiff::Timestamp;
use sqlx::Row;

use crate::{ImportState, Result, Store};

/// Numbers for the overview.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overview {
    /// Sources: the register and confirmed imports.
    pub sources: i64,
    /// Sources that are crawled.
    pub sources_crawled: i64,
    /// Imported sources waiting for a person's decision.
    pub proposals: i64,
    /// Documents that are still published.
    pub documents: i64,
    /// Versions of documents.
    pub versions: i64,
    /// Addresses that are checked.
    pub endpoints: i64,
    /// Addresses due for a check now.
    pub endpoints_due: i64,
    /// The latest check.
    pub last_check: Option<Timestamp>,
    /// Checks in the last 24 hours.
    pub checks_last_day: i64,
    /// Lists of proposals waiting to be handed to the editor.
    pub waiting_to_share: i64,
}

/// A source with what was collected from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSummary {
    /// ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// Entry address.
    pub url: String,
    /// Country.
    pub country: String,
    /// Authority level (A1…E).
    pub authority: String,
    /// Licence.
    pub licence: String,
    /// Terms of use.
    pub terms: String,
    /// `true`: from the register; `false`: imported by a user.
    pub register: bool,
    /// State of an imported source (register sources are `Confirmed`).
    pub state: ImportState,
    /// Crawled.
    pub enabled: bool,
    /// Documents that are still published.
    pub documents: i64,
    /// The latest check of one of its addresses.
    pub last_check: Option<Timestamp>,
}

fn timestamp(row: &sqlx::postgres::PgRow, column: &str) -> Result<Option<Timestamp>> {
    Ok(row
        .try_get::<Option<jiff_sqlx::Timestamp>, _>(column)?
        .map(|t| t.to_jiff()))
}

impl Store {
    /// The numbers of the overview.
    pub async fn overview(&self) -> Result<Overview> {
        let row = sqlx::query(
            "SELECT
                (SELECT count(*) FROM source WHERE state = 'confirmed') AS sources,
                (SELECT count(*) FROM source WHERE state = 'confirmed' AND enabled) AS sources_crawled,
                (SELECT count(*) FROM source WHERE state = 'proposed') AS proposals,
                (SELECT count(*) FROM document WHERE status = 'active') AS documents,
                (SELECT count(*) FROM document_version) AS versions,
                (SELECT count(*) FROM endpoint e JOIN source s ON s.id = e.source_id
                  WHERE e.enabled AND s.enabled) AS endpoints,
                (SELECT count(*) FROM endpoint e JOIN source s ON s.id = e.source_id
                  WHERE e.enabled AND s.enabled AND e.next_fetch_at <= now()) AS endpoints_due,
                (SELECT max(last_fetch_at) FROM endpoint) AS last_check,
                (SELECT count(*) FROM fetch_log WHERE at > now() - interval '1 day') AS checks_last_day,
                (SELECT count(*) FROM outbox) AS waiting_to_share",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Overview {
            sources: row.try_get("sources")?,
            sources_crawled: row.try_get("sources_crawled")?,
            proposals: row.try_get("proposals")?,
            documents: row.try_get("documents")?,
            versions: row.try_get("versions")?,
            endpoints: row.try_get("endpoints")?,
            endpoints_due: row.try_get("endpoints_due")?,
            last_check: timestamp(&row, "last_check")?,
            checks_last_day: row.try_get("checks_last_day")?,
            waiting_to_share: row.try_get("waiting_to_share")?,
        })
    }

    /// Every source except rejected imports: the register first, then imports, each by ID.
    pub async fn source_summaries(&self) -> Result<Vec<SourceSummary>> {
        let rows = sqlx::query(
            "SELECT s.id, s.name, s.url, s.country, s.authority, s.licence, s.terms, s.origin,
                    s.state, s.enabled,
                    (SELECT count(*) FROM document d
                      WHERE d.source_id = s.id AND d.status = 'active') AS documents,
                    (SELECT max(e.last_fetch_at) FROM endpoint e
                      WHERE e.source_id = s.id) AS last_check
               FROM source s
              WHERE s.state <> 'rejected'
              ORDER BY s.origin DESC, s.id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(SourceSummary {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    url: row.try_get("url")?,
                    country: row.try_get("country")?,
                    authority: row.try_get("authority")?,
                    licence: row.try_get("licence")?,
                    terms: row.try_get("terms")?,
                    register: row.try_get::<String, _>("origin")? == "register",
                    state: ImportState::parse(row.try_get("state")?),
                    enabled: row.try_get("enabled")?,
                    documents: row.try_get("documents")?,
                    last_check: timestamp(row, "last_check")?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::TestDb;
    use crate::{
        AssessmentRecord, EndpointKind, NewEndpoint, NewImport, Origin, RegisterSource,
        SourceRecord, Via,
    };

    fn register_source(id: &str, enabled: bool) -> RegisterSource {
        RegisterSource {
            source: SourceRecord {
                id: id.into(),
                name: format!("Source {id}"),
                url: format!("https://{}.example.sk/", id.to_lowercase()),
                country: "SK".into(),
                languages: vec!["sk".into()],
                authority: "A1".into(),
                priority: "P1".into(),
                connector: "html".into(),
                interval_s: 86_400,
                licence: "copyrighted".into(),
                terms: "unknown".into(),
                enabled,
            },
            endpoints: vec![NewEndpoint {
                url: format!("https://{}.example.sk/", id.to_lowercase()),
                host: format!("{}.example.sk", id.to_lowercase()),
                kind: EndpointKind::Page,
                origin: Origin::Register,
                follow_links: true,
                interval_s: 86_400,
            }],
        }
    }

    #[tokio::test]
    async fn the_overview_counts_what_is_there() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let store = &db.store;
        assert_eq!(store.overview().await.unwrap(), super::Overview::default());
        store
            .sync_register(&[
                register_source("SK01", true),
                register_source("SK02", false),
            ])
            .await
            .unwrap();
        let import = NewImport {
            id: "U-0123456789".into(),
            name: "Škola".into(),
            url: "https://skola.example.sk/".into(),
            country: Some("SK".into()),
            connector: "html".into(),
            interval_s: 86_400,
            via: Via::Local,
            endpoints: Vec::new(),
        };
        let auto = AssessmentRecord {
            by_person: None,
            licence: "copyrighted".into(),
            terms: "unknown".into(),
            country: Some("SK".into()),
            rights_page: "https://skola.example.sk/".into(),
            finding: "no terms found".into(),
            evidence: None,
        };
        store.import_source(&import, &auto).await.unwrap();
        store
            .outbox_add("ATENA-NAVRHY 1\nhttps://skola.example.sk/ SK\n")
            .await
            .unwrap();

        let overview = store.overview().await.unwrap();
        assert_eq!(overview.sources, 2);
        assert_eq!(overview.sources_crawled, 1);
        assert_eq!(overview.proposals, 1);
        assert_eq!(overview.documents, 0);
        assert_eq!(
            overview.endpoints, 1,
            "the disabled source's endpoint is not checked"
        );
        assert_eq!(overview.endpoints_due, 1);
        assert_eq!(overview.waiting_to_share, 1);
        assert_eq!(overview.last_check, None);

        let sources = store.source_summaries().await.unwrap();
        let ids: Vec<&str> = sources.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["SK01", "SK02", "U-0123456789"]);
        assert!(sources[0].register && sources[0].enabled);
        assert!(!sources[1].enabled);
        assert!(!sources[2].register);
        assert_eq!(sources[2].state, crate::ImportState::Proposed);
        db.drop().await;
    }
}
