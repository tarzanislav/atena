//! robots.txt as specified by RFC 9309, plus the widely used `crawl-delay` and `sitemap` records.
//!
//! The crawler obeys the group that names its product token; when there is none it obeys the
//! `*` group, and when that is missing too, nothing is restricted. Within the chosen rules the
//! longest matching pattern decides and `allow` wins a tie.

use std::time::Duration;

use crate::pattern::{Pattern, normalize_path};

/// Parsing limit; RFC 9309 asks crawlers to parse at least 500 KiB.
pub const MAX_ROBOTS_BYTES: usize = 500 * 1024;

#[derive(Debug, Clone)]
struct Rule {
    allow: bool,
    pattern: Pattern,
}

#[derive(Debug, Clone, Default)]
struct Group {
    /// Lower-cased product tokens (`*` for any crawler).
    agents: Vec<String>,
    rules: Vec<Rule>,
    crawl_delay: Option<Duration>,
}

/// A parsed robots.txt file.
#[derive(Debug, Clone, Default)]
pub struct RobotsTxt {
    groups: Vec<Group>,
    sitemaps: Vec<String>,
}

/// The rules that apply to one crawler.
#[derive(Debug, Clone, Default)]
pub struct AgentRules {
    rules: Vec<Rule>,
    crawl_delay: Option<Duration>,
}

/// What a crawler may do on one origin, derived from the robots.txt response (RFC 9309 §2.3.1).
#[derive(Debug, Clone)]
pub enum RobotsPolicy {
    /// No robots.txt (a 4xx response): everything may be crawled.
    AllowAll,
    /// robots.txt could not be fetched (5xx, network error): nothing may be crawled.
    DisallowAll,
    /// Rules from a robots.txt file.
    Rules(AgentRules),
}

/// The leading `[A-Za-z_-]` run of a `user-agent` value: `ATENA/1.0 (+https://…)` → `atena`.
fn product_token(value: &str) -> String {
    if value.trim() == "*" {
        return "*".to_string();
    }
    value
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || matches!(c, '_' | '-'))
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Crawl delay in seconds; negative, non-finite and absurd values are ignored.
fn parse_delay(value: &str) -> Option<Duration> {
    let seconds: f64 = value.trim().parse().ok()?;
    (seconds.is_finite() && (0.0..=86_400.0).contains(&seconds))
        .then(|| Duration::from_secs_f64(seconds))
}

/// Truncates to the parsing limit without splitting a UTF-8 character.
fn within_limit(text: &str) -> &str {
    if text.len() <= MAX_ROBOTS_BYTES {
        return text;
    }
    let mut end = MAX_ROBOTS_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

impl RobotsTxt {
    /// Parses robots.txt content; only the first [`MAX_ROBOTS_BYTES`] are read.
    pub fn parse(text: &str) -> Self {
        let text = within_limit(text.strip_prefix('\u{feff}').unwrap_or(text));
        let mut robots = Self::default();
        // a user-agent line right after another one joins its group
        let mut collecting_agents = false;
        for line in text.split(['\n', '\r']) {
            let line = line.split('#').next().unwrap_or_default();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "user-agent" | "user agent" | "useragent" => {
                    let token = product_token(value);
                    if token.is_empty() {
                        continue;
                    }
                    match robots.groups.last_mut() {
                        Some(group) if collecting_agents => group.agents.push(token),
                        _ => robots.groups.push(Group {
                            agents: vec![token],
                            ..Group::default()
                        }),
                    }
                    collecting_agents = true;
                }
                key @ ("allow" | "disallow" | "dissallow" | "disalow" | "dissalow") => {
                    collecting_agents = false;
                    // rules before the first user-agent line belong to no group
                    let (Some(group), false) = (robots.groups.last_mut(), value.is_empty()) else {
                        continue;
                    };
                    group.rules.push(Rule {
                        allow: key == "allow",
                        pattern: Pattern::new(value),
                    });
                }
                "crawl-delay" => {
                    if let (Some(group), Some(delay)) =
                        (robots.groups.last_mut(), parse_delay(value))
                    {
                        group.crawl_delay = Some(delay);
                    }
                }
                "sitemap" if !value.is_empty() => robots.sitemaps.push(value.to_string()),
                _ => {}
            }
        }
        robots
    }

    /// Sitemap URLs listed in the file (not tied to any group).
    pub fn sitemaps(&self) -> &[String] {
        &self.sitemaps
    }

    /// The combined rules of all groups naming `product_token`, or of all `*` groups.
    pub fn rules_for(&self, product_token: &str) -> AgentRules {
        let token = product_token.to_ascii_lowercase();
        let named: Vec<&Group> = self
            .groups
            .iter()
            .filter(|g| g.agents.contains(&token))
            .collect();
        let chosen = if named.is_empty() {
            self.groups
                .iter()
                .filter(|g| g.agents.iter().any(|a| a == "*"))
                .collect()
        } else {
            named
        };
        AgentRules {
            rules: chosen
                .iter()
                .flat_map(|g| g.rules.iter().cloned())
                .collect(),
            crawl_delay: chosen.iter().filter_map(|g| g.crawl_delay).max(),
        }
    }
}

impl AgentRules {
    /// `true` when the path with query (`/a/b?c=d`) may be crawled. `/robots.txt` always may.
    pub fn is_allowed(&self, path_and_query: &str) -> bool {
        if path_and_query == "/robots.txt" {
            return true;
        }
        let path = normalize_path(path_and_query);
        self.rules
            .iter()
            .filter(|r| r.pattern.matches(&path))
            .max_by_key(|r| (r.pattern.len(), r.allow))
            .is_none_or(|r| r.allow)
    }

    /// The crawl delay requested for this crawler, if any.
    pub fn crawl_delay(&self) -> Option<Duration> {
        self.crawl_delay
    }
}

impl RobotsPolicy {
    /// Policy for a robots.txt response: 2xx is parsed, 4xx (except 429) means no restrictions,
    /// anything else (429, 5xx) means the server is not ready for crawling.
    pub fn from_response(status: u16, body: &[u8], product_token: &str) -> Self {
        match status {
            200..=299 => Self::Rules(
                RobotsTxt::parse(&String::from_utf8_lossy(body)).rules_for(product_token),
            ),
            429 => Self::DisallowAll,
            400..=499 => Self::AllowAll,
            _ => Self::DisallowAll,
        }
    }

    /// `true` when the path with query may be crawled.
    pub fn is_allowed(&self, path_and_query: &str) -> bool {
        match self {
            Self::AllowAll => true,
            Self::DisallowAll => path_and_query == "/robots.txt",
            Self::Rules(rules) => rules.is_allowed(path_and_query),
        }
    }

    /// The crawl delay requested for this crawler, if any.
    pub fn crawl_delay(&self) -> Option<Duration> {
        match self {
            Self::Rules(rules) => rules.crawl_delay(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> AgentRules {
        RobotsTxt::parse(text).rules_for("ATENA")
    }

    #[test]
    fn longest_match_wins_and_allow_breaks_ties() {
        let r = rules("User-agent: *\nDisallow: /folder\nAllow: /folder/page\n");
        assert!(r.is_allowed("/folder/page.html"));
        assert!(!r.is_allowed("/folder/other"));
        assert!(r.is_allowed("/elsewhere"));
        let tie = rules("User-agent: *\nDisallow: /page\nAllow: /page\n");
        assert!(tie.is_allowed("/page"));
        let root = rules("User-agent: *\nDisallow: /\nAllow: /$\n");
        assert!(root.is_allowed("/"));
        assert!(!root.is_allowed("/index.html"));
    }

    #[test]
    fn named_group_replaces_the_star_group() {
        let text = "User-agent: *\nDisallow: /\n\nUser-agent: atena/1.0\nDisallow: /private\n";
        let r = rules(text);
        assert!(r.is_allowed("/public"));
        assert!(!r.is_allowed("/private/x"));
        assert!(
            !RobotsTxt::parse(text)
                .rules_for("OtherBot")
                .is_allowed("/public")
        );
    }

    #[test]
    fn groups_for_the_same_agent_are_combined() {
        let r = rules(
            "User-agent: ATENA\nDisallow: /a\n\nUser-agent: other\nDisallow: /b\n\n\
             User-agent: Atena\nDisallow: /c\nCrawl-delay: 4\n",
        );
        assert!(!r.is_allowed("/a"));
        assert!(r.is_allowed("/b"));
        assert!(!r.is_allowed("/c"));
        assert_eq!(r.crawl_delay(), Some(Duration::from_secs(4)));
    }

    #[test]
    fn consecutive_agents_share_a_group_and_other_records_do_not_split_it() {
        let text = "User-agent: a\nCrawl-delay: 2\nUser-agent: ATENA\n\nDisallow: /x\n\
                    Sitemap: https://example.org/sitemap.xml\nDisallow: /y\nUser-agent: b\nAllow: /\n";
        let robots = RobotsTxt::parse(text);
        let r = robots.rules_for("atena");
        assert!(!r.is_allowed("/x") && !r.is_allowed("/y"));
        assert_eq!(r.crawl_delay(), Some(Duration::from_secs(2)));
        assert!(robots.rules_for("b").is_allowed("/x"));
        assert_eq!(robots.sitemaps(), ["https://example.org/sitemap.xml"]);
    }

    #[test]
    fn tolerant_syntax() {
        let r = rules(
            "\u{feff}Disallow: /ignored-before-any-agent\nUSER-AGENT : *  # all\r\n\
             DISALLOW:/tmp/ # temp\r\ndisalow: /typo\nDisallow:\nnonsense line\n",
        );
        assert!(r.is_allowed("/ignored-before-any-agent"));
        assert!(!r.is_allowed("/tmp/file"));
        assert!(!r.is_allowed("/typo"));
        assert!(r.is_allowed("/other"));
    }

    #[test]
    fn no_groups_means_no_restrictions_and_robots_txt_is_always_allowed() {
        assert!(rules("Sitemap: https://example.org/s.xml").is_allowed("/anything"));
        assert!(rules("User-agent: *\nDisallow: /").is_allowed("/robots.txt"));
        assert!(rules("User-agent: *\nDisallow: /*?").is_allowed("/a"));
        assert!(!rules("User-agent: *\nDisallow: /*?").is_allowed("/a?b=1"));
    }

    #[test]
    fn response_status_semantics() {
        let allow = RobotsPolicy::from_response(404, b"", "ATENA");
        assert!(allow.is_allowed("/x"));
        let deny = RobotsPolicy::from_response(503, b"", "ATENA");
        assert!(!deny.is_allowed("/x"));
        assert!(deny.is_allowed("/robots.txt"));
        assert!(!RobotsPolicy::from_response(429, b"", "ATENA").is_allowed("/x"));
        let parsed = RobotsPolicy::from_response(200, b"User-agent: *\nDisallow: /x", "ATENA");
        assert!(!parsed.is_allowed("/x/y"));
    }

    #[test]
    fn parsing_stops_at_the_limit() {
        let mut text = String::from("User-agent: *\n");
        while text.len() < MAX_ROBOTS_BYTES {
            text.push_str("Allow: /filler\n");
        }
        text.push_str("Disallow: /late\n");
        assert!(rules(&text).is_allowed("/late"));
        assert!(parse_delay("-1").is_none() && parse_delay("NaN").is_none());
        assert_eq!(parse_delay("0.5"), Some(Duration::from_millis(500)));
    }
}
