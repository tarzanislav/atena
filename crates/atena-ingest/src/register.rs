//! From the source register (`data/sources.yaml`) to sources and their register endpoints.

use atena_core::legality::{SourceFacts, permission};
use atena_core::source::{Review, Source};
use atena_store::{EndpointKind, NewEndpoint, Origin, RegisterSource, SourceRecord};
use url::Url;

use crate::urls::{host, normalize};

/// Archived sources are captured once and then only checked yearly.
const ARCHIVE_INTERVAL_S: i32 = 365 * 86_400;

/// Connectors whose entry page is a landing page of an API: it is watched, but its links are not followed.
const API_CONNECTORS: [&str; 3] = ["api", "oai", "youtube-api"];

fn register_endpoint(
    url: &str,
    kind: EndpointKind,
    follow_links: bool,
    interval_s: i32,
) -> Option<NewEndpoint> {
    let url = normalize(&Url::parse(url).ok()?);
    Some(NewEndpoint {
        host: host(&url)?,
        url: url.to_string(),
        kind,
        origin: Origin::Register,
        follow_links,
        interval_s,
    })
}

/// The register entries for [`atena_store::Store::sync_register`]: every source is watched
/// through its entry page (links followed unless it is an API) and its feed, if it has one.
pub fn register_entries(sources: &[Source]) -> Vec<RegisterSource> {
    sources
        .iter()
        .map(|s| {
            let mut interval_s = i32::try_from(s.interval_s).unwrap_or(i32::MAX).max(60);
            if s.review == Review::Archive {
                interval_s = interval_s.max(ARCHIVE_INTERVAL_S);
            }
            let follow = !API_CONNECTORS.contains(&s.connector.as_str());
            let endpoints = [
                register_endpoint(&s.url, EndpointKind::Page, follow, interval_s),
                s.feed
                    .as_deref()
                    .and_then(|f| register_endpoint(f, EndpointKind::Feed, false, interval_s)),
            ]
            .into_iter()
            .flatten()
            .collect();
            RegisterSource {
                source: SourceRecord {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    url: s.url.clone(),
                    country: s.country.clone(),
                    languages: s.languages.clone(),
                    authority: s.authority.clone(),
                    priority: s.priority.clone(),
                    connector: s.connector.clone(),
                    interval_s,
                    licence: s.licence.as_str().to_string(),
                    terms: s.terms.as_str().to_string(),
                    // the legality principle decides whether the source is crawled at all
                    enabled: permission(SourceFacts::from(s), false).crawl,
                },
                endpoints,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use atena_core::Taxonomy;
    use atena_core::source::{BUNDLED_SOURCES, load_sources};

    use super::*;

    #[test]
    fn the_bundled_register_maps_to_endpoints() {
        let sources = load_sources(BUNDLED_SOURCES, &Taxonomy::bundled().unwrap()).unwrap();
        let entries = register_entries(&sources);
        assert_eq!(entries.len(), sources.len());
        let sk01 = entries.iter().find(|e| e.source.id == "SK01").unwrap();
        let kinds: Vec<EndpointKind> = sk01.endpoints.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, [EndpointKind::Page, EndpointKind::Feed]);
        assert!(sk01.endpoints[0].follow_links);
        assert_eq!(sk01.endpoints[0].host, "www.minedu.sk");
        assert!(
            entries
                .iter()
                .all(|e| !e.endpoints.is_empty() && e.source.interval_s >= 60)
        );
        for e in entries
            .iter()
            .filter(|e| API_CONNECTORS.contains(&e.source.connector.as_str()))
        {
            assert!(!e.endpoints[0].follow_links, "{}", e.source.id);
        }
        let youtube = entries.iter().find(|e| e.source.id == "N39").unwrap();
        assert!(!youtube.source.enabled && youtube.source.terms == "forbidden");
        let slovlex = entries.iter().find(|e| e.source.id == "SK03").unwrap();
        assert_eq!(slovlex.source.licence, "official");
    }
}
