//! Sharing proposals between installations without a server of ATENA's own.
//!
//! An installation posts the addresses its users import – nothing else, no assessments, no
//! personal data – as short text messages to a relay: a topic on a public [ntfy](https://ntfy.sh)
//! server, which deletes messages after 12 hours. A scheduled GitHub Actions workflow of the ATENA
//! repository copies new messages into issues labelled `navrh-zdrojov`, so none is lost while the
//! editor's computer is off. The editor's installation reads both, assesses every address itself
//! (a received assessment would prove nothing) and decides.
//!
//! A message: the line `ATENA-NAVRHY 1`, then one address or DOI per line, optionally followed by
//! a country code – the same lines a user types into an import.

use atena_crawl::{Method, ServiceClient};
use url::Url;

/// First line of every message.
pub const HEADER: &str = "ATENA-NAVRHY 1";
/// The relay used unless `ATENA_RELAY` names another (any ntfy server and topic works).
pub const DEFAULT_RELAY: &str = "https://ntfy.sh/atena-navrhy-spty1ssg9ywc";
/// The repository whose issues keep the messages.
pub const DEFAULT_REPOSITORY: &str = "tarzanislav/atena";
/// Label of those issues.
pub const ISSUE_LABEL: &str = "navrh-zdrojov";
/// Largest message body (ntfy stores up to 4,096 bytes as text).
pub const MAX_MESSAGE_BYTES: usize = 3_800;
/// Most lines read from one received message.
pub const MAX_LINES: usize = 100;

/// A message received from the relay or from an issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    /// The relay's message ID (an issue carries it in its title).
    pub id: String,
    /// The item lines, without the header.
    pub lines: String,
    /// The issue it came from, if any.
    pub issue: Option<u64>,
}

/// Packs item lines into messages of at most [`MAX_MESSAGE_BYTES`] and [`MAX_LINES`] lines, each
/// with the header.
pub fn pack(lines: &[String]) -> Vec<String> {
    let mut messages = Vec::new();
    let mut current = String::from(HEADER);
    let mut count = 0;
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.len() + HEADER.len() + 2 > MAX_MESSAGE_BYTES {
            continue;
        }
        if count == MAX_LINES || current.len() + 1 + line.len() > MAX_MESSAGE_BYTES {
            messages.push(std::mem::replace(&mut current, String::from(HEADER)));
            count = 0;
        }
        current.push('\n');
        current.push_str(line);
        count += 1;
    }
    if current.len() > HEADER.len() {
        messages.push(current);
    }
    messages
}

/// The item lines of a message, or `None` when it is not an ATENA message. At most
/// [`MAX_LINES`] lines are kept; anything else is for the import to judge.
pub fn unpack(body: &str) -> Option<String> {
    let mut lines = body.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next()? != HEADER {
        return None;
    }
    let items: Vec<&str> = lines.take(MAX_LINES).collect();
    (!items.is_empty()).then(|| items.join("\n"))
}

fn relay_json(relay: &Url) -> Result<Url, String> {
    let mut url = relay.clone();
    url.path_segments_mut()
        .map_err(|_| format!("{relay}: not a relay address"))?
        .pop_if_empty()
        .push("json");
    url.set_query(Some("poll=1&since=all"));
    Ok(url)
}

/// Posts one message to the relay.
pub async fn send(client: &ServiceClient, relay: &Url, message: &str) -> Result<(), String> {
    let answer = client
        .send(
            Method::POST,
            relay,
            &[
                ("Title", "ATENA".to_string()),
                ("Content-Type", "text/plain; charset=utf-8".to_string()),
            ],
            Some(message.to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;
    if (200..300).contains(&answer.status) {
        Ok(())
    } else {
        Err(format!("{relay}: HTTP {}", answer.status))
    }
}

/// The ATENA messages the relay still holds.
pub async fn receive(client: &ServiceClient, relay: &Url) -> Result<Vec<Received>, String> {
    let url = relay_json(relay)?;
    let body = client.get(&url, &[]).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&body).lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event["event"] != "message" {
            continue;
        }
        let (Some(id), Some(message)) = (event["id"].as_str(), event["message"].as_str()) else {
            continue;
        };
        if !id.chars().all(|c| c.is_ascii_alphanumeric()) || id.is_empty() || id.len() > 64 {
            continue;
        }
        if let Some(lines) = unpack(message) {
            out.push(Received {
                id: id.to_string(),
                lines,
                issue: None,
            });
        }
    }
    Ok(out)
}

/// The lines inside the first ```` ``` ```` block of an issue body (the workflow fences the
/// message so that GitHub renders nothing of it).
fn fenced(body: &str) -> &str {
    let Some(start) = body.find("```") else {
        return body;
    };
    let rest = &body[start + 3..];
    let rest = rest.split_once('\n').map_or(rest, |(_, r)| r);
    rest.split("```").next().unwrap_or(rest)
}

/// The messages kept as open issues of `repository` (no sign-in needed for a public repository).
pub async fn receive_issues(
    client: &ServiceClient,
    api: &Url,
    repository: &str,
) -> Result<Vec<Received>, String> {
    let url = api
        .join(&format!(
            "repos/{repository}/issues?labels={ISSUE_LABEL}&state=open&per_page=100"
        ))
        .map_err(|e| e.to_string())?;
    let body = client
        .get(
            &url,
            &[("Accept", "application/vnd.github+json".to_string())],
        )
        .await
        .map_err(|e| e.to_string())?;
    let issues: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for issue in issues.as_array().into_iter().flatten() {
        let (Some(number), Some(title)) = (issue["number"].as_u64(), issue["title"].as_str())
        else {
            continue;
        };
        let id = title.rsplit(' ').next().unwrap_or_default();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        if let Some(lines) = unpack(fenced(issue["body"].as_str().unwrap_or_default())) {
            out.push(Received {
                id: id.to_string(),
                lines,
                issue: Some(number),
            });
        }
    }
    Ok(out)
}

/// Closes a processed issue (needs a token that may write issues of the repository).
pub async fn close_issue(
    client: &ServiceClient,
    api: &Url,
    repository: &str,
    number: u64,
    token: &str,
) -> Result<(), String> {
    let url = api
        .join(&format!("repos/{repository}/issues/{number}"))
        .map_err(|e| e.to_string())?;
    let answer = client
        .send(
            Method::PATCH,
            &url,
            &[
                ("Accept", "application/vnd.github+json".to_string()),
                ("Authorization", format!("Bearer {token}")),
            ],
            Some(r#"{"state":"closed","state_reason":"completed"}"#.to_string()),
        )
        .await
        .map_err(|e| e.to_string())?;
    if (200..300).contains(&answer.status) {
        Ok(())
    } else {
        Err(format!("issue {number}: HTTP {}", answer.status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_split_and_read_back() {
        let lines: Vec<String> = (0..300)
            .map(|i| format!("https://skola{i}.example.sk/ SK"))
            .collect();
        let messages = pack(&lines);
        assert!(messages.len() > 1);
        assert!(
            messages
                .iter()
                .all(|m| m.len() <= MAX_MESSAGE_BYTES && m.starts_with(HEADER))
        );
        let back: Vec<String> = messages
            .iter()
            .flat_map(|m| {
                unpack(m)
                    .unwrap()
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(back, lines);
        assert_eq!(unpack("hello\nhttps://x.sk/"), None);
        assert_eq!(unpack(HEADER), None);
        assert!(pack(&[]).is_empty());
    }

    #[test]
    fn issue_bodies_are_read_inside_the_fence() {
        let body = "Návrh zdrojov z inštalácie ATENA.\n\n```text\nATENA-NAVRHY 1\nhttps://a.example.sk/ SK\n```\n";
        assert_eq!(
            unpack(fenced(body)).as_deref(),
            Some("https://a.example.sk/ SK")
        );
        let relay = Url::parse("https://ntfy.sh/atena-navrhy-x").unwrap();
        assert_eq!(
            relay_json(&relay).unwrap().as_str(),
            "https://ntfy.sh/atena-navrhy-x/json?poll=1&since=all"
        );
    }
}
