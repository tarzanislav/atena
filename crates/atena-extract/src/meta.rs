//! Dates in document metadata: ISO 8601 (HTML, DOCX, ODT, XMP) and PDF date strings.

use jiff::{Timestamp, civil, tz::TimeZone};

/// ISO 8601 / RFC 3339 with or without offset or time (`2026-10-24T08:00:00+02:00`, `2026-10-24`);
/// values without an offset are taken as UTC.
pub(crate) fn parse_date(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    if let Ok(ts) = text.parse::<Timestamp>() {
        return Some(ts);
    }
    text.parse::<civil::DateTime>()
        .ok()
        .and_then(|dt| dt.to_zoned(TimeZone::UTC).ok())
        .map(|z| z.timestamp())
}

/// PDF date `D:YYYYMMDDHHmmSSOHH'mm'` where everything after the year is optional.
pub(crate) fn parse_pdf_date(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    let s = text.strip_prefix("D:").unwrap_or(text);
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 4 {
        return None;
    }
    let field = |from: usize, len: usize, default: i32| -> Option<i32> {
        digits
            .get(from..from + len)
            .map_or(Some(default), |d| d.parse().ok())
    };
    let (y, mo, d) = (field(0, 4, 0)?, field(4, 2, 1)?, field(6, 2, 1)?);
    let (h, mi, sec) = (field(8, 2, 0)?, field(10, 2, 0)?, field(12, 2, 0)?);
    let small = |v: i32| i8::try_from(v).ok();
    // invalid values (month 13, day 99) are an error, never a panic
    let dt = civil::DateTime::new(
        i16::try_from(y).ok()?,
        small(mo)?,
        small(d)?,
        small(h)?,
        small(mi)?,
        small(sec)?,
        0,
    )
    .ok()?;
    let rest = &s[digits.len()..];
    let offset_minutes = match rest.chars().next() {
        Some(sign @ ('+' | '-')) => {
            let nums: Vec<i32> = rest[1..]
                .split(|c: char| !c.is_ascii_digit())
                .filter(|p| !p.is_empty())
                .take(2)
                .filter_map(|p| p.parse().ok())
                .collect();
            let (hours, mins) = (
                nums.first().copied().unwrap_or(0),
                nums.get(1).copied().unwrap_or(0),
            );
            if hours > 23 || mins > 59 {
                return None;
            }
            let minutes = hours * 60 + mins;
            if sign == '-' { -minutes } else { minutes }
        }
        _ => 0,
    };
    let utc = dt.to_zoned(TimeZone::UTC).ok()?.timestamp();
    utc.checked_sub(jiff::SignedDuration::from_mins(i64::from(offset_minutes)))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(ts: Option<Timestamp>) -> String {
        ts.map(|t| t.to_string()).unwrap_or_default()
    }

    #[test]
    fn iso_and_pdf_dates() {
        assert_eq!(
            s(parse_date("2026-10-24T08:00:00+02:00")),
            "2026-10-24T06:00:00Z"
        );
        assert_eq!(s(parse_date("2026-10-24")), "2026-10-24T00:00:00Z");
        assert_eq!(s(parse_date("2026-10-24T08:00:00")), "2026-10-24T08:00:00Z");
        assert_eq!(s(parse_date("24. 10. 2026")), "");
        assert_eq!(
            s(parse_pdf_date("D:20261024080000+02'00'")),
            "2026-10-24T06:00:00Z"
        );
        assert_eq!(
            s(parse_pdf_date("D:20261024080000Z")),
            "2026-10-24T08:00:00Z"
        );
        assert_eq!(s(parse_pdf_date("D:2026")), "2026-01-01T00:00:00Z");
        assert_eq!(s(parse_pdf_date("20261024")), "2026-10-24T00:00:00Z");
        assert_eq!(s(parse_pdf_date("D:20261399")), "");
        assert_eq!(s(parse_pdf_date("garbage")), "");
        assert_eq!(s(parse_pdf_date("D:20261024080000+99999999'00'")), "");
    }
}
