//! Network safety: the crawler only talks to the public Internet.
//!
//! A source URL, a redirect or a link in a feed could point to `localhost`, a router or a cloud
//! metadata service (server-side request forgery). Two checks prevent that:
//! 1. [`NetPolicy::check_url`] before every request (scheme, credentials, host, port, IP literals);
//! 2. [`VettingResolver`] after DNS resolution: only public addresses are passed on to the
//!    connection, so a host name that resolves to a private address – also on a later lookup
//!    (DNS rebinding) – is never connected to.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

/// An IP network in CIDR notation, e.g. `10.0.0.0/8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpRange {
    network: IpAddr,
    prefix: u8,
}

impl IpRange {
    /// A range from a network address and a prefix length; `None` when the prefix is too long.
    pub fn new(network: IpAddr, prefix: u8) -> Option<Self> {
        let max = if network.is_ipv4() { 32 } else { 128 };
        (prefix <= max).then_some(Self { network, prefix })
    }

    /// `true` when `ip` lies in the range (IPv4 and IPv6 never match each other).
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => prefix_eq(
                u128::from(net.to_bits()),
                u128::from(ip.to_bits()),
                32,
                self.prefix,
            ),
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                prefix_eq(net.to_bits(), ip.to_bits(), 128, self.prefix)
            }
            _ => false,
        }
    }
}

fn prefix_eq(a: u128, b: u128, bits: u8, prefix: u8) -> bool {
    prefix == 0 || (a ^ b) >> (bits - prefix) == 0
}

fn v4(a: u8, b: u8, c: u8, d: u8, prefix: u8) -> IpRange {
    IpRange {
        network: IpAddr::V4(Ipv4Addr::new(a, b, c, d)),
        prefix,
    }
}

fn v6(segments: [u16; 8], prefix: u8) -> IpRange {
    IpRange {
        network: IpAddr::V6(Ipv6Addr::from(segments)),
        prefix,
    }
}

/// IPv4 ranges that are not public unicast (IANA special-purpose registry).
fn blocked_v4() -> [IpRange; 15] {
    [
        v4(0, 0, 0, 0, 8),       // "this network"
        v4(10, 0, 0, 0, 8),      // private
        v4(100, 64, 0, 0, 10),   // carrier-grade NAT
        v4(127, 0, 0, 0, 8),     // loopback
        v4(169, 254, 0, 0, 16),  // link-local, cloud metadata services
        v4(172, 16, 0, 0, 12),   // private
        v4(192, 0, 0, 0, 24),    // IETF protocol assignments
        v4(192, 0, 2, 0, 24),    // documentation
        v4(192, 88, 99, 0, 24),  // 6to4 relay anycast (deprecated)
        v4(192, 168, 0, 0, 16),  // private
        v4(198, 18, 0, 0, 15),   // benchmarking
        v4(198, 51, 100, 0, 24), // documentation
        v4(203, 0, 113, 0, 24),  // documentation
        v4(224, 0, 0, 0, 4),     // multicast
        v4(240, 0, 0, 0, 4),     // reserved and broadcast
    ]
}

/// IPv6 ranges inside global unicast `2000::/3` that are still not public.
fn blocked_v6() -> [IpRange; 4] {
    [
        v6([0x2001, 0, 0, 0, 0, 0, 0, 0], 23), // IETF protocol assignments, incl. Teredo
        v6([0x2001, 0x0db8, 0, 0, 0, 0, 0, 0], 32), // documentation
        v6([0x2002, 0, 0, 0, 0, 0, 0, 0], 16), // 6to4: carries an arbitrary IPv4 address
        v6([0x3fff, 0, 0, 0, 0, 0, 0, 0], 20), // documentation (RFC 9637)
    ]
}

/// `true` for public unicast addresses. IPv6 must be in `2000::/3`; everything else
/// (loopback, link-local, unique local, multicast, IPv4-mapped, NAT64…) is refused.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(_) => !blocked_v4().iter().any(|r| r.contains(ip)),
        IpAddr::V6(v6addr) => {
            v6addr.segments()[0] & 0xe000 == 0x2000 && !blocked_v6().iter().any(|r| r.contains(ip))
        }
    }
}

/// Suffixes of names that never belong to the public Internet (RFC 2606, 6761, 6762, 7686, 8375).
const RESERVED_SUFFIXES: [&str; 8] = [
    "localhost",
    "local",
    "internal",
    "home.arpa",
    "test",
    "example",
    "invalid",
    "onion",
];

/// Time allowed for one DNS lookup.
pub const DNS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Longest accepted URL.
pub const MAX_URL_LEN: usize = 2_000;

/// Why a URL was refused before any connection was made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Blocked {
    /// Only `http` and `https` are fetched.
    #[error("scheme {0} is not allowed")]
    Scheme(String),
    /// User names and passwords in URLs are refused.
    #[error("URLs with credentials are not allowed")]
    Credentials,
    /// The URL has no host.
    #[error("URL has no host")]
    NoHost,
    /// Single-label names (`intranet`) and reserved names (`*.local`, `localhost`…) are refused.
    #[error("host {0} is not a public name")]
    HostName(String),
    /// The port is not in the allowed list.
    #[error("port {0} is not allowed")]
    Port(u16),
    /// The host is, or resolves only to, a non-public address.
    #[error("address {0} is not public")]
    Address(String),
    /// The URL is longer than [`MAX_URL_LEN`].
    #[error("URL is too long")]
    TooLong,
    /// A redirect from an encrypted address to an unencrypted one, where anyone on the way could
    /// change the content.
    #[error("redirect from https to plain http")]
    Downgrade,
}

/// Which destinations the crawler may contact.
#[derive(Debug, Clone)]
pub struct NetPolicy {
    /// Allowed ports; default 80, 443, 8080 and 8443.
    pub ports: Vec<u16>,
    /// Non-public ranges that are allowed anyway – for an intranet deployment or tests. Empty by default.
    pub allowed_ranges: Vec<IpRange>,
}

impl Default for NetPolicy {
    fn default() -> Self {
        Self {
            ports: vec![80, 443, 8080, 8443],
            allowed_ranges: Vec::new(),
        }
    }
}

impl NetPolicy {
    /// `true` when the crawler may connect to `ip`.
    pub fn allows_ip(&self, ip: IpAddr) -> bool {
        is_public(ip) || self.allowed_ranges.iter().any(|r| r.contains(ip))
    }

    /// Checks everything that can be checked without DNS.
    pub fn check_url(&self, url: &Url) -> Result<(), Blocked> {
        if url.as_str().len() > MAX_URL_LEN {
            return Err(Blocked::TooLong);
        }
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Blocked::Scheme(url.scheme().to_string()));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(Blocked::Credentials);
        }
        let port = url.port_or_known_default().ok_or(Blocked::NoHost)?;
        if !self.ports.contains(&port) {
            return Err(Blocked::Port(port));
        }
        match url.host().ok_or(Blocked::NoHost)? {
            Host::Ipv4(ip) => self.check_ip(IpAddr::V4(ip)),
            Host::Ipv6(ip) => self.check_ip(IpAddr::V6(ip)),
            Host::Domain(name) => {
                let name = name.trim_end_matches('.').to_ascii_lowercase();
                let reserved = RESERVED_SUFFIXES
                    .iter()
                    .any(|s| name == *s || name.ends_with(&format!(".{s}")));
                if reserved || !name.contains('.') {
                    Err(Blocked::HostName(name))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn check_ip(&self, ip: IpAddr) -> Result<(), Blocked> {
        if self.allows_ip(ip) {
            Ok(())
        } else {
            Err(Blocked::Address(ip.to_string()))
        }
    }

    /// The resolved addresses the crawler may connect to; an error when none is left.
    pub fn vet(&self, host: &str, addrs: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, Blocked> {
        let allowed: Vec<SocketAddr> = addrs
            .into_iter()
            .filter(|a| self.allows_ip(a.ip()))
            .collect();
        if allowed.is_empty() {
            Err(Blocked::Address(host.to_string()))
        } else {
            Ok(allowed)
        }
    }
}

/// DNS resolver for the HTTP client that drops every non-public address.
#[derive(Debug, Clone)]
pub struct VettingResolver {
    policy: Arc<NetPolicy>,
}

impl VettingResolver {
    /// A resolver enforcing `policy`.
    pub fn new(policy: Arc<NetPolicy>) -> Self {
        Self { policy }
    }
}

impl Resolve for VettingResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = Arc::clone(&self.policy);
        Box::pin(async move {
            let host = name.as_str().to_string();
            // a DNS server that does not answer must not hold a worker for minutes
            let lookup = tokio::net::lookup_host((host.as_str(), 0));
            let found: Vec<SocketAddr> = tokio::time::timeout(DNS_TIMEOUT, lookup)
                .await
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("DNS lookup of {host} timed out"),
                    )
                })??
                .collect();
            let allowed = policy.vet(&host, found)?;
            Ok(Box::new(allowed.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn public_and_non_public_addresses() {
        for public in [
            "1.1.1.1",
            "185.15.59.224",
            "2a00:1450:4001:82a::200e",
            "2606:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(is_public(ip(public)), "{public}");
        }
        for private in [
            "0.0.0.0",
            "10.1.2.3",
            "100.100.100.200",
            "127.0.0.1",
            "169.254.169.254",
            "172.31.255.255",
            "192.168.1.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:8.8.8.8",
            "64:ff9b::a00:1",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "2001:0:4136:e378::1",
            "2002:7f00:1::1",
            "3fff::1",
        ] {
            assert!(!is_public(ip(private)), "{private}");
        }
        assert!(is_public(ip("172.32.0.1")) && is_public(ip("100.128.0.1")));
    }

    #[test]
    fn urls_are_checked_before_any_connection() {
        let p = NetPolicy::default();
        let check = |u: &str| p.check_url(&Url::parse(u).unwrap());
        assert_eq!(check("https://www.minedu.sk/zakony/"), Ok(()));
        assert_eq!(check("http://example.org.:8080/"), Ok(()));
        assert!(matches!(
            check("file:///etc/passwd"),
            Err(Blocked::Scheme(_))
        ));
        assert!(matches!(
            check("ftp://example.org/"),
            Err(Blocked::Scheme(_))
        ));
        assert_eq!(
            check("https://user:pw@example.org/"),
            Err(Blocked::Credentials)
        );
        assert_eq!(check("https://example.org:22/"), Err(Blocked::Port(22)));
        for name in [
            "http://localhost/",
            "http://a.localhost/",
            "http://printer.local/",
            "http://db.internal/",
            "http://intranet/",
            "http://router.home.arpa/",
        ] {
            assert!(matches!(check(name), Err(Blocked::HostName(_))), "{name}");
        }
        // the URL parser normalises odd IPv4 notations, so they are caught too
        for literal in [
            "http://127.0.0.1/",
            "http://2130706433/",
            "http://0x7f.1/",
            "http://[::1]/",
            "http://[::ffff:169.254.169.254]/",
            "http://169.254.169.254/",
        ] {
            assert!(
                matches!(check(literal), Err(Blocked::Address(_))),
                "{literal}"
            );
        }
        let long = format!("https://example.org/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(check(&long), Err(Blocked::TooLong));
    }

    #[test]
    fn resolved_addresses_are_filtered() {
        let p = NetPolicy::default();
        let sa = |s: &str| SocketAddr::new(ip(s), 443);
        assert_eq!(
            p.vet("mixed.example", vec![sa("10.0.0.5"), sa("93.184.215.14")]),
            Ok(vec![sa("93.184.215.14")])
        );
        assert!(
            p.vet("rebind.example", vec![sa("127.0.0.1"), sa("::1")])
                .is_err()
        );
        let lab = NetPolicy {
            allowed_ranges: vec![IpRange::new(ip("127.0.0.0"), 8).unwrap()],
            ..NetPolicy::default()
        };
        assert!(lab.allows_ip(ip("127.0.0.1")) && !lab.allows_ip(ip("10.0.0.1")));
        assert!(IpRange::new(ip("10.0.0.0"), 33).is_none());
        assert!(
            IpRange::new(ip("::"), 0)
                .unwrap()
                .contains(ip("2001:db8::1"))
        );
    }
}
