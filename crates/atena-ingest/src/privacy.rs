//! Data minimisation (GDPR Art. 5(1)(c)): contact details are removed from texts before they are
//! stored. ATENA needs what a document says about education, not how to reach the people named
//! in it – the original stays one click away for anyone who needs the contact.
//!
//! Removed: e-mail addresses (also written as `name(at)school.sk`, `name [at] school.sk`,
//! `name(zavináč)school.sk`) and telephone numbers (9–15 digits starting with `+` or `0`, e.g.
//! `+421 2 5937 4111`, `0905 123 456`, `02/5937 4111`). Dates, years, law numbers (`245/2008`) and
//! statistics are left alone; so are ISBN and ISSN numbers.

/// Replaces an e-mail address.
pub const EMAIL_MARK: &str = "[email]";
/// Replaces a telephone number.
pub const PHONE_MARK: &str = "[phone]";

/// A text with contact details removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redacted {
    /// The text.
    pub text: String,
    /// E-mail addresses removed.
    pub emails: usize,
    /// Telephone numbers removed.
    pub phones: usize,
}

/// Ways of writing `@`, lower-case.
const AT_SIGNS: [&str; 7] = [
    "@",
    "(at)",
    "[at]",
    "{at}",
    "(zavináč)",
    "[zavináč]",
    "(zavinac)",
];

fn local_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn domain_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-')
}

/// The end of a domain starting at `from` (`school.sk`), if it is one.
fn domain_end(text: &str, from: usize) -> Option<usize> {
    let rest = &text[from..];
    let len = rest.find(|c: char| !domain_char(c)).unwrap_or(rest.len());
    let domain = rest[..len].trim_end_matches(['.', '-']);
    let (_, tld) = domain.rsplit_once('.')?;
    (tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()) && !domain.starts_with('.'))
        .then_some(from + domain.len())
}

fn redact_emails(text: &str) -> (String, usize) {
    let lower = text.to_lowercase();
    // byte offsets of `lower` equal those of `text` only for ASCII-length-preserving input;
    // fall back to a case-sensitive search when lower-casing changed the length
    let haystack = if lower.len() == text.len() {
        lower.as_str()
    } else {
        text
    };
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut copied = 0;
    let mut search = 0;
    while search < text.len() {
        let next = AT_SIGNS
            .iter()
            .filter_map(|sign| {
                haystack[search..]
                    .find(sign)
                    .map(|at| (search + at, sign.len()))
            })
            .min();
        let Some((at, sign_len)) = next else { break };
        let local_end = text[..at].trim_end_matches(' ').len();
        let local_start = text[..local_end]
            .char_indices()
            .rev()
            .take_while(|(_, c)| local_char(*c))
            .last()
            .map_or(local_end, |(i, _)| i);
        let after = at + sign_len;
        let domain_start =
            after + (text[after..].len() - text[after..].trim_start_matches(' ').len());
        let spaced = local_end != at || domain_start != after;
        let plain_at = sign_len == 1;
        match domain_end(text, domain_start) {
            // a plain "@" must touch both parts; bracketed forms may have spaces around them
            Some(end)
                if local_start < local_end && local_start >= copied && !(plain_at && spaced) =>
            {
                out.push_str(&text[copied..local_start]);
                out.push_str(EMAIL_MARK);
                copied = end;
                search = end;
                count += 1;
            }
            _ => search = after,
        }
    }
    out.push_str(&text[copied..]);
    (out, count)
}

fn phone_separator(c: char) -> bool {
    matches!(c, ' ' | '-' | '/' | '.' | '(' | ')' | '\u{a0}' | '\u{202f}')
}

fn redact_phones(text: &str) -> (String, usize) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut copied = 0;
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        let prev = i.checked_sub(1).map(|p| chars[p].1);
        let boundary = prev.is_none_or(|p| !p.is_alphanumeric() && p != '+' && p != '/');
        let opens =
            c == '+' || c == '0' || (c == '(' && chars.get(i + 1).is_some_and(|n| n.1 == '0'));
        if !(boundary && opens) {
            i += 1;
            continue;
        }
        // take digits and separators, at most 25 characters
        let mut j = i;
        let mut digits = 0;
        let mut last_digit = i;
        while j < chars.len() && j - i < 25 {
            let ch = chars[j].1;
            if ch.is_ascii_digit() {
                digits += 1;
                last_digit = j;
            } else if !(phone_separator(ch) || (ch == '+' && j == i)) {
                break;
            }
            j += 1;
        }
        let followed_by_digit = chars
            .get(last_digit + 1)
            .is_some_and(|n| n.1.is_ascii_digit());
        let before = text[chars[i.saturating_sub(12)].0..start].to_ascii_uppercase();
        let identifier = before.contains("ISBN") || before.contains("ISSN");
        if (9..=15).contains(&digits) && !followed_by_digit && !identifier {
            let end = chars[last_digit].0 + 1;
            out.push_str(&text[copied..start]);
            out.push_str(PHONE_MARK);
            copied = end;
            count += 1;
            i = last_digit + 1;
        } else {
            // the whole run is not a number to remove; no part of it is either (an IBAN, a code)
            i = j.max(i + 1);
        }
    }
    out.push_str(&text[copied..]);
    (out, count)
}

/// Removes e-mail addresses and telephone numbers.
pub fn redact(text: &str) -> Redacted {
    let (text, emails) = redact_emails(text);
    let (text, phones) = redact_phones(&text);
    Redacted {
        text,
        emails,
        phones,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(text: &str) -> String {
        redact(text).text
    }

    #[test]
    fn e_mail_addresses_also_obfuscated() {
        assert_eq!(r("Píšte na riaditel@zs-priklad.sk."), "Píšte na [email].");
        assert_eq!(
            r("jana.nova(at)skola.edu.sk alebo peter [at] skola.sk"),
            "[email] alebo [email]"
        );
        assert_eq!(r("info(zavináč)gymnazium.cz"), "[email]");
        assert_eq!(r("Mail: JANA.NOVA@Skola.SK, tel."), "Mail: [email], tel.");
        // not addresses
        assert_eq!(r("Stretnutie @ 15:00 v aule"), "Stretnutie @ 15:00 v aule");
        assert_eq!(r("cena 5 @ kus"), "cena 5 @ kus");
        assert_eq!(r("twitter @minedu"), "twitter @minedu");
        assert_eq!(redact("a@b.sk c@d.sk").emails, 2);
    }

    #[test]
    fn telephone_numbers() {
        assert_eq!(r("Tel.: +421 2 5937 4111, fax"), "Tel.: [phone], fax");
        assert_eq!(r("mobil 0905 123 456."), "mobil [phone].");
        assert_eq!(r("02/5937 4111"), "[phone]");
        assert_eq!(r("(02) 593 74 111"), "[phone]");
        assert_eq!(r("+420\u{a0}224\u{a0}398\u{a0}111"), "[phone]");
        assert_eq!(r("00421 905 123 456"), "[phone]");
    }

    #[test]
    fn numbers_that_are_not_phones() {
        for text in [
            "Zákon č. 245/2008 Z. z.",
            "Termín 15. 4. 2026 o 10:00",
            "01.02.2026",
            "Počet žiakov: 123 456 789",
            "ISBN 0-19-852663-6",
            "suma 0,50 €",
            "IČO 00 123 456",
            "SK89 7500 0000 0000 1234 5671",
            "kód 0123456789012345678",
        ] {
            assert_eq!(r(text), text, "{text}");
        }
    }
}
