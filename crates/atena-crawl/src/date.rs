//! Lenient parsing of the date formats found in feeds, sitemaps and HTTP headers.

use jiff::fmt::rfc2822::DateTimeParser;
use jiff::{Timestamp, civil, tz::TimeZone};

/// Zone abbreviations common in Central and Eastern Europe that RFC 2822 does not define.
const ZONES: [(&str, &str); 9] = [
    ("UTC", "+0000"),
    ("WET", "+0000"),
    ("WEST", "+0100"),
    ("BST", "+0100"),
    ("CET", "+0100"),
    ("CEST", "+0200"),
    ("EET", "+0200"),
    ("EEST", "+0300"),
    ("MSK", "+0300"),
];

static RFC2822: DateTimeParser = DateTimeParser::new().relaxed_weekday(true);

/// Parses RFC 3339 / ISO 8601 (`2026-10-24T08:00:00+02:00`, `2026-10-24`) and RFC 2822 / HTTP dates
/// (`Sat, 24 Oct 2026 08:00:00 GMT`). Values without an offset are taken as UTC.
pub(crate) fn parse_date(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(ts) = text.parse::<Timestamp>() {
        return Some(ts);
    }
    if let Ok(dt) = text.parse::<civil::DateTime>() {
        return dt.to_zoned(TimeZone::UTC).ok().map(|z| z.timestamp());
    }
    let with_offset = match text.rsplit_once(' ') {
        Some((head, zone)) => match ZONES
            .iter()
            .find(|(name, _)| zone.eq_ignore_ascii_case(name))
        {
            Some((_, offset)) => format!("{head} {offset}"),
            None => text.to_string(),
        },
        None => text.to_string(),
    };
    RFC2822.parse_timestamp(&with_offset).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(text: &str) -> String {
        parse_date(text).map(|t| t.to_string()).unwrap_or_default()
    }

    #[test]
    fn common_formats() {
        assert_eq!(iso("2026-10-24T08:00:00+02:00"), "2026-10-24T06:00:00Z");
        assert_eq!(iso("2026-10-24T06:00:00Z"), "2026-10-24T06:00:00Z");
        assert_eq!(iso("2026-10-24"), "2026-10-24T00:00:00Z");
        assert_eq!(iso("Sat, 24 Oct 2026 08:00:00 GMT"), "2026-10-24T08:00:00Z");
        assert_eq!(
            iso("Sat, 24 Oct 2026 08:00:00 +0200"),
            "2026-10-24T06:00:00Z"
        );
        assert_eq!(
            iso("Sat, 24 Oct 2026 08:00:00 CEST"),
            "2026-10-24T06:00:00Z"
        );
        // a wrong weekday is a common mistake and does not discard the date
        assert_eq!(iso("Mon, 24 Oct 2026 08:00:00 GMT"), "2026-10-24T08:00:00Z");
        assert_eq!(iso("24 Oct 2026 08:00 EST"), "2026-10-24T13:00:00Z");
        assert_eq!(iso("yesterday"), "");
        assert_eq!(iso(""), "");
    }
}
