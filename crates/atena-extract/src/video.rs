//! Links to videos on YouTube and Vimeo, recognised in a page's links and embedded players.
//!
//! ATENA never visits these services (their terms forbid automated access); it only records which
//! video a publisher links to or embeds, as one canonical address per video.

use url::Url;

/// Most videos recorded per page.
pub const MAX_VIDEOS: usize = 50;

/// A video a page links to or embeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    /// Canonical address: `https://www.youtube.com/watch?v=…` or `https://vimeo.com/…`.
    pub url: Url,
    /// The player's `title` or the link text, as the publisher wrote it (may be empty).
    pub title: String,
}

const YOUTUBE_HOSTS: [&str; 5] = [
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "youtube-nocookie.com",
    "www.youtube-nocookie.com",
];
const VIMEO_HOSTS: [&str; 2] = ["vimeo.com", "www.vimeo.com"];

/// A YouTube video ID: 11 characters of `[A-Za-z0-9_-]`.
fn is_youtube_id(text: &str) -> bool {
    text.len() == 11
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// A Vimeo video ID: up to 12 digits.
fn is_vimeo_id(text: &str) -> bool {
    !text.is_empty() && text.len() <= 12 && text.bytes().all(|b| b.is_ascii_digit())
}

/// The canonical address of the video `url` points to, if it points to one: watch pages, short
/// links, embedded players and shorts of YouTube, and video pages and players of Vimeo. Channels,
/// playlists and search pages are not videos.
pub fn video_url(url: &Url) -> Option<Url> {
    let host = url.host_str()?.to_ascii_lowercase();
    let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let youtube = |id: &str| {
        is_youtube_id(id)
            .then(|| Url::parse(&format!("https://www.youtube.com/watch?v={id}")).ok())
            .flatten()
    };
    let vimeo = |id: &str| {
        is_vimeo_id(id)
            .then(|| Url::parse(&format!("https://vimeo.com/{id}")).ok())
            .flatten()
    };
    match (host.as_str(), segments.as_slice()) {
        ("youtu.be", [id]) => youtube(id),
        (h, ["watch"]) if YOUTUBE_HOSTS.contains(&h) => url
            .query_pairs()
            .find(|(key, _)| key == "v")
            .and_then(|(_, id)| youtube(&id)),
        (h, ["embed" | "shorts" | "live" | "v", id]) if YOUTUBE_HOSTS.contains(&h) => youtube(id),
        ("player.vimeo.com", ["video", id]) => vimeo(id),
        (h, [id]) if VIMEO_HOSTS.contains(&h) => vimeo(id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(text: &str) -> Option<String> {
        video_url(&Url::parse(text).unwrap()).map(|u| u.to_string())
    }

    #[test]
    fn every_form_of_a_video_address_has_one_canonical_form() {
        let watch = Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string());
        for form in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s",
            "https://m.youtube.com/watch?feature=share&v=dQw4w9WgXcQ",
            "https://youtu.be/dQw4w9WgXcQ?si=abc",
            "https://www.youtube.com/embed/dQw4w9WgXcQ?rel=0",
            "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
            "http://YOUTUBE.com/v/dQw4w9WgXcQ",
        ] {
            assert_eq!(canonical(form), watch, "{form}");
        }
        let vimeo = Some("https://vimeo.com/76979871".to_string());
        assert_eq!(canonical("https://vimeo.com/76979871"), vimeo);
        assert_eq!(
            canonical("https://player.vimeo.com/video/76979871?h=x"),
            vimeo
        );
    }

    #[test]
    fn channels_playlists_and_other_sites_are_not_videos() {
        for other in [
            "https://www.youtube.com/@ministerstvo",
            "https://www.youtube.com/channel/UC1234567890",
            "https://www.youtube.com/playlist?list=PL123",
            "https://www.youtube.com/watch?v=short",
            "https://www.youtube.com/results?search_query=x",
            "https://vimeo.com/channels/staffpicks",
            "https://example.com/embed/dQw4w9WgXcQ",
            "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
        ] {
            assert_eq!(canonical(other), None, "{other}");
        }
    }
}
