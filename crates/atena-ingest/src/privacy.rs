//! Data minimisation (GDPR Art. 5(1)(c)): contact details are removed from texts before they are
//! stored. ATENA needs what a document says about education, not how to reach the people named
//! in it – the original stays one click away for anyone who needs the contact.
//!
//! Removed:
//! * e-mail addresses, also with international domains and written as `name(at)school.sk`,
//!   `name [at] school.sk`, `name(zavináč)school.sk` or `name (at) school (dot) sk`;
//! * telephone numbers: 9–15 digits starting with `+` or `0` (`+421 2 5937 4111`, `0905 123 456`,
//!   `02/5937 4111`), Russian `8 (495) 123-45-67`, Chinese mobile numbers (`138 0013 8000`), and any
//!   7–15 digits right after a label such as `tel.`, `mobil`, `fax`, `телефон` or `电话` (Czech
//!   `224 398 111`, Polish `22 123 45 67`);
//! * birth numbers (`905123/1234`, or ten digits after `rodné číslo`).
//!
//! Dates, years, law numbers (`245/2008`) and statistics are left alone; so are ISBN and ISSN
//! numbers.

/// Replaces an e-mail address.
pub const EMAIL_MARK: &str = "[email]";
/// Replaces a telephone number.
pub const PHONE_MARK: &str = "[phone]";
/// Replaces a birth number.
pub const ID_MARK: &str = "[ID]";

/// A text with contact details removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redacted {
    /// The text.
    pub text: String,
    /// E-mail addresses removed.
    pub emails: usize,
    /// Telephone numbers removed.
    pub phones: usize,
    /// Birth numbers removed.
    pub ids: usize,
}

impl Redacted {
    /// All details removed.
    pub fn total(&self) -> usize {
        self.emails + self.phones + self.ids
    }
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

/// Ways of writing the dots of a domain when `@` is written out, lower-case.
const DOT_SIGNS: [&str; 7] = [
    "(dot)", "[dot]", "{dot}", "(bodka)", "[bodka]", "(tečka)", "[tečka]",
];

fn local_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

/// Length of a written-out dot (with spaces around it) at the start of `text`.
fn dot_sign(text: &str) -> Option<usize> {
    let spaces = text.len() - text.trim_start_matches(' ').len();
    let rest = &text[spaces..];
    DOT_SIGNS.iter().find_map(|sign| {
        rest.get(..sign.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(sign))
            .then(|| spaces + sign.len())
    })
}

/// The end of a domain starting at `from` (`school.sk`, `škola.sk`; with `written_out`, also
/// `school (dot) sk`), if it is one: at least two labels, the last of at least two letters.
fn domain_end(text: &str, from: usize, written_out: bool) -> Option<usize> {
    let mut at = from;
    let mut labels = 0;
    let mut end = from;
    let mut last = "";
    loop {
        let rest = &text[at..];
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '-'))
            .unwrap_or(rest.len());
        if len == 0 {
            break;
        }
        last = &rest[..len];
        labels += 1;
        end = at + len;
        let tail = &text[end..];
        let separator = if tail.starts_with('.') {
            Some(1)
        } else if written_out {
            dot_sign(tail)
        } else {
            None
        };
        let Some(separator) = separator else { break };
        let next = &text[end + separator..];
        let spaces = next.len() - next.trim_start_matches(' ').len();
        if !next[spaces..].starts_with(char::is_alphanumeric) {
            break;
        }
        at = end + separator + spaces;
    }
    let tld = last.trim_end_matches('-');
    (labels >= 2 && tld.chars().count() >= 2 && tld.chars().all(char::is_alphabetic)).then_some(end)
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
    // the next position of every sign, looked up again only once the search has passed it, so the
    // text is read a fixed number of times however many `@` it holds (a page of thousands of `@`
    // must not take minutes)
    let mut next: Vec<Option<usize>> = AT_SIGNS.iter().map(|s| haystack.find(s)).collect();
    while search < text.len() {
        for (position, sign) in next.iter_mut().zip(AT_SIGNS) {
            if position.is_some_and(|p| p < search) {
                *position = haystack[search..].find(sign).map(|p| search + p);
            }
        }
        let Some((at, sign_len)) = next
            .iter()
            .zip(AT_SIGNS)
            .filter_map(|(p, sign)| p.map(|p| (p, sign.len())))
            .min()
        else {
            break;
        };
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
        match domain_end(text, domain_start, !plain_at) {
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

/// Labels after which a number is a telephone number, lower-case.
const PHONE_LABELS: [&str; 14] = [
    "tel", "telefon", "telefón", "mobil", "mob.", "fax", "phone", "kontakt", "тел", "моб", "факс",
    "电话", "手机", "☎",
];
/// Labels after which a number is a birth number, lower-case.
const ID_LABELS: [&str; 5] = ["rodné číslo", "rodne cislo", "rodné č", "r. č.", "r.č."];
/// How far before a number a label may stand, in characters.
const LABEL_REACH: usize = 20;

/// `true` when one of `labels` starts a word within [`LABEL_REACH`] characters before `chars[i]`.
fn labelled(chars: &[(usize, char)], i: usize, labels: &[&str]) -> bool {
    let from = i.saturating_sub(LABEL_REACH);
    let window: String = chars[from..i].iter().map(|(_, c)| *c).collect();
    let window = window.to_lowercase();
    labels.iter().any(|label| {
        window.match_indices(label).any(|(at, _)| {
            window[..at]
                .chars()
                .next_back()
                .is_none_or(|p| !p.is_alphabetic())
        })
    })
}

/// A plausible birth date `YYMMDD` (month + 20 / 50 / 70 for women and for the later series).
fn birth_date(digits: &str) -> bool {
    let number =
        |range: std::ops::Range<usize>| digits.get(range).and_then(|d| d.parse::<u32>().ok());
    matches!(
        (number(2..4), number(4..6)),
        (Some(1..=12 | 21..=32 | 51..=62 | 71..=82), Some(1..=31))
    )
}

fn redact_ids(text: &str) -> (String, usize) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let (mut count, mut copied, mut i) = (0, 0, 0);
    while i < chars.len() {
        let (start, c) = chars[i];
        let boundary = i == 0 || !chars[i - 1].1.is_alphanumeric();
        if !(boundary && c.is_ascii_digit()) {
            i += 1;
            continue;
        }
        // digits, at most one slash, more digits
        let mut j = i;
        let mut digits = String::new();
        let mut slash = None;
        while j < chars.len() {
            match chars[j].1 {
                d if d.is_ascii_digit() => digits.push(d),
                '/' if slash.is_none() && j > i => slash = Some(digits.len()),
                _ => break,
            }
            j += 1;
        }
        let followed = chars.get(j).is_some_and(|n| n.1.is_alphanumeric());
        let is_id = !followed
            && birth_date(&digits)
            && match slash {
                // 905123/1234: six digits, a slash, three or four digits
                Some(6) => (9..=10).contains(&digits.len()),
                // 9051231234 only right after a label
                None => (9..=10).contains(&digits.len()) && labelled(&chars, i, &ID_LABELS),
                Some(_) => false,
            };
        if is_id {
            out.push_str(&text[copied..start]);
            out.push_str(ID_MARK);
            copied = chars.get(j).map_or(text.len(), |n| n.0);
            count += 1;
        }
        i = j.max(i + 1);
    }
    out.push_str(&text[copied..]);
    (out, count)
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
        let next = chars.get(i + 1).map(|n| n.1);
        let prefixed = c == '+' || c == '0' || (c == '(' && next == Some('0'));
        // Russian "8 (495) 123-45-67"
        let russian = c == '8' && {
            let rest = &text[start + 1..];
            rest.trim_start_matches(' ').starts_with('(')
        };
        let chinese = c == '1' && next.is_some_and(|n| ('3'..='9').contains(&n));
        let after_label = (c.is_ascii_digit() || c == '(') && labelled(&chars, i, &PHONE_LABELS);
        if !(boundary && (prefixed || russian || chinese || after_label)) {
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
        // a Chinese mobile number: 11 digits, written whole or as 3-4-4
        let groups: Vec<usize> = text[start..chars[last_digit].0 + 1]
            .split(|ch: char| !ch.is_ascii_digit())
            .map(str::len)
            .filter(|n| *n > 0)
            .collect();
        let enough = if prefixed || russian {
            (9..=15).contains(&digits)
        } else if after_label {
            (7..=15).contains(&digits)
        } else {
            groups == [11] || groups == [3, 4, 4]
        };
        if enough && !followed_by_digit && !identifier {
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
    // birth numbers first: `905123/1234` must not be taken for a telephone number
    let (text, ids) = redact_ids(&text);
    let (text, phones) = redact_phones(&text);
    Redacted {
        text,
        emails,
        phones,
        ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(text: &str) -> String {
        redact(text).text
    }

    #[test]
    fn thousands_of_at_signs_take_no_time() {
        let hostile = "a@".repeat(200_000) + "riaditel@skola.sk";
        let started = std::time::Instant::now();
        let out = redact(&hostile);
        assert_eq!(out.emails, 1);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
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
    fn numbers_of_other_countries_and_after_labels() {
        assert_eq!(r("Tel.: 224 398 111"), "Tel.: [phone]");
        assert_eq!(
            r("telefon 22 123 45 67, sekretariat"),
            "telefon [phone], sekretariat"
        );
        assert_eq!(r("Телефон: 8 (495) 123-45-67"), "Телефон: [phone]");
        assert_eq!(r("8 (495) 123-45-67"), "[phone]");
        assert_eq!(r("电话：138 0013 8000"), "电话：[phone]");
        assert_eq!(r("手机 13800138000"), "手机 [phone]");
        assert_eq!(r("fax (02) 5937 4112"), "fax [phone]");
        // a hotel is not a telephone label
        assert_eq!(r("hotel 123 456 789"), "hotel 123 456 789");
    }

    #[test]
    fn birth_numbers() {
        assert_eq!(
            r("Žiak, nar. 905123/1234, trieda 5.A"),
            "Žiak, nar. [ID], trieda 5.A"
        );
        assert_eq!(r("rodné číslo: 9051231234"), "rodné číslo: [ID]");
        assert_eq!(redact("r.č. 015406/123").ids, 1);
        // law numbers and other slashes stay
        assert_eq!(r("Zákon č. 245/2008 Z. z."), "Zákon č. 245/2008 Z. z.");
        assert_eq!(r("číslo spisu 202399/1234"), "číslo spisu 202399/1234");
    }

    #[test]
    fn international_and_written_out_addresses() {
        assert_eq!(r("jana@škola.sk"), "[email]");
        assert_eq!(r("пишите: info@школа.рф"), "пишите: [email]");
        assert_eq!(r("jana (at) skola (dot) sk"), "[email]");
        assert_eq!(r("jana[at]skola[dot]edu[dot]sk."), "[email].");
        // a written-out dot needs a written-out at
        assert_eq!(r("pozri skola (dot) sk"), "pozri skola (dot) sk");
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
            "rozpočet 15 000 000 000 €",
            "12 345 678 901 obyvateľov",
        ] {
            assert_eq!(r(text), text, "{text}");
        }
    }
}
