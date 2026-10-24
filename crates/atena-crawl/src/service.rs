//! Requests that are not crawling: made only on a user's explicit command (downloading the shared
//! register) or to the operator's own server – never to third-party services
//! (`atena_core::legality::CONNECTIONS`). robots.txt is a rule for crawlers and does not apply; the
//! address checks and the limits do.

use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use reqwest::{Client, Method};
use url::Url;

use crate::fetch::{classify, install_crypto_provider};
use crate::net::{NetPolicy, VettingResolver};
use crate::{Config, FetchError};

/// Largest response read from a service.
pub const MAX_SERVICE_BYTES: usize = 20 * 1024 * 1024;

/// A client for ATENA's own services.
#[derive(Debug, Clone)]
pub struct ServiceClient {
    client: Client,
    net: Arc<NetPolicy>,
}

/// A service's answer.
#[derive(Debug, Clone)]
pub struct Answer {
    /// HTTP status.
    pub status: u16,
    /// Body, at most [`MAX_SERVICE_BYTES`].
    pub body: Bytes,
}

impl ServiceClient {
    /// A client with the fetcher's user agent, address checks and timeouts; redirects are not
    /// followed.
    pub fn new(config: &Config) -> Result<Self, FetchError> {
        install_crypto_provider();
        let net = Arc::new(config.net.clone());
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(VettingResolver::new(Arc::clone(&net)))
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout.max(Duration::from_secs(30)))
            .build()
            .map_err(|e| FetchError::Setup(e.to_string()))?;
        Ok(Self { client, net })
    }

    /// Sends a request; `headers` are added as given.
    pub async fn send(
        &self,
        method: Method,
        url: &Url,
        headers: &[(&str, String)],
        body: Option<String>,
    ) -> Result<Answer, FetchError> {
        self.net
            .check_url(url)
            .map_err(|reason| FetchError::Blocked {
                url: url.to_string(),
                reason,
            })?;
        let mut request = self.client.request(method, url.clone());
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        if let Some(body) = body {
            request = request.body(body);
        }
        let mut response = request.send().await.map_err(|e| classify(url, &e))?;
        let status = response.status().as_u16();
        let mut body = BytesMut::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| classify(url, &e))? {
            if body.len() + chunk.len() > MAX_SERVICE_BYTES {
                return Err(FetchError::TooLarge {
                    url: url.to_string(),
                    limit: MAX_SERVICE_BYTES,
                });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Answer {
            status,
            body: body.freeze(),
        })
    }

    /// `GET` that must succeed (2xx).
    pub async fn get(&self, url: &Url, headers: &[(&str, String)]) -> Result<Bytes, FetchError> {
        let answer = self.send(Method::GET, url, headers, None).await?;
        if !(200..300).contains(&answer.status) {
            return Err(FetchError::Status {
                url: url.to_string(),
                status: answer.status,
            });
        }
        Ok(answer.body)
    }
}
