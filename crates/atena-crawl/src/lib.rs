//! ATENA crawler: polite and safe fetching of education sources.
//!
//! * [`Fetcher`] – the only way ATENA downloads anything: network safety checks, robots.txt
//!   (RFC 9309), per-host pacing, `Retry-After`, conditional requests, size and type limits,
//!   TDMRep and `X-Robots-Tag`.
//! * [`RobotsTxt`], [`TdmFile`], [`IndexRules`] – the publisher's rules, usable on their own.
//! * [`parse_feed`], [`parse_sitemap`] – discovering new and changed items.
//!
//! The crate never decides what ATENA does with content; it reports what the publisher allows
//! (`Page::tdm`, `Page::index`) and leaves the decision to the pipeline.

mod date;
mod directives;
mod feed;
mod fetch;
mod net;
mod pace;
mod pattern;
mod robots;
mod sitemap;
#[cfg(feature = "testing")]
pub mod testing;
mod xml;

pub use directives::{IndexRules, Tdm, TdmFile, parse_reservation};
pub use feed::{Feed, FeedError, FeedFormat, FeedItem, MAX_ITEMS, parse_feed};
pub use fetch::{
    Config, DEFAULT_TYPES, FetchError, Fetched, Fetcher, PRODUCT_TOKEN, Page, Validators,
};
pub use net::{Blocked, DNS_TIMEOUT, IpRange, MAX_URL_LEN, NetPolicy, VettingResolver, is_public};
pub use robots::{AgentRules, MAX_ROBOTS_BYTES, RobotsPolicy, RobotsTxt};
pub use sitemap::{
    ChangeFreq, MAX_SITEMAP_BYTES, MAX_SITEMAP_URLS, Sitemap, SitemapError, SitemapRef, SitemapUrl,
    parse_sitemap,
};
pub use xml::XmlError;
