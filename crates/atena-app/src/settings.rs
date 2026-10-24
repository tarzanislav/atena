//! The user's settings, kept in `settings.json` in the app's folder – on this computer only.

use std::path::Path;

use atena_core::safety::clean_line;
use serde::{Deserialize, Serialize};

use crate::AppError;

/// Languages of the user interface; Slovak is the reference.
pub const LANGUAGES: &[&str] = &["sk", "en", "cs", "ru", "zh"];
/// Longest name of the person who decides.
pub const MAX_EDITOR_CHARS: usize = 80;
/// Most hosts crawled at the same time.
pub const MAX_CONCURRENCY: usize = 16;

/// Light or dark.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    /// As the system is set.
    #[default]
    System,
    /// Light.
    Light,
    /// Dark.
    Dark,
}

/// The settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Language of the user interface (one of [`LANGUAGES`]).
    pub language: String,
    /// Light or dark.
    pub theme: Theme,
    /// The name recorded with the user's decisions on proposals; it stays in this installation
    /// (the shared register names no person).
    pub editor: String,
    /// Imported addresses are kept for handing to the editor (nothing is ever sent by itself).
    pub share_proposals: bool,
    /// Hosts crawled at the same time (1 to [`MAX_CONCURRENCY`]).
    pub concurrency: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "sk".into(),
            theme: Theme::System,
            editor: String::new(),
            share_proposals: true,
            concurrency: 4,
        }
    }
}

impl Settings {
    /// The settings made safe: a known language, a clean name of limited length, a concurrency in
    /// range.
    pub fn checked(mut self) -> Self {
        if !LANGUAGES.contains(&self.language.as_str()) {
            self.language = Self::default().language;
        }
        self.editor = clean_line(&self.editor)
            .chars()
            .take(MAX_EDITOR_CHARS)
            .collect();
        self.concurrency = self.concurrency.clamp(1, MAX_CONCURRENCY);
        self
    }

    /// The settings in `path`; the defaults when there are none or they cannot be read (a broken
    /// file is kept as `settings.json.broken` for the user to look at).
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(settings) => settings.checked(),
            Err(e) => {
                tracing::warn!("{}: {e}; using the default settings", path.display());
                let _ = std::fs::rename(path, path.with_extension("json.broken"));
                Self::default()
            }
        }
    }

    /// Writes the settings to `path` (through a temporary file, so a crash leaves the old ones).
    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::new("settings", e.to_string()))?;
        let temporary = path.with_extension("json.new");
        std::fs::write(&temporary, text).map_err(|e| AppError::io(&temporary, e))?;
        std::fs::rename(&temporary, path).map_err(|e| AppError::io(path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_checked() {
        let s = Settings {
            language: "xx".into(),
            editor: format!("  Ján\u{202e}{}", "a".repeat(200)),
            concurrency: 0,
            ..Settings::default()
        }
        .checked();
        assert_eq!(s.language, "sk");
        assert!(s.editor.starts_with("Ján") && !s.editor.contains('\u{202e}'));
        assert_eq!(s.editor.chars().count(), MAX_EDITOR_CHARS);
        assert_eq!(s.concurrency, 1);
        assert_eq!(
            Settings {
                concurrency: 99,
                ..Settings::default()
            }
            .checked()
            .concurrency,
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn settings_are_kept_and_broken_files_set_aside() {
        let folder = std::env::temp_dir().join(format!("atena-settings-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());
        let mine = Settings {
            language: "cs".into(),
            theme: Theme::Dark,
            editor: "Ján".into(),
            share_proposals: false,
            concurrency: 2,
        };
        mine.save(&path).unwrap();
        assert_eq!(Settings::load(&path), mine);
        // fields the file does not have take their defaults
        std::fs::write(&path, r#"{"language": "en"}"#).unwrap();
        assert_eq!(Settings::load(&path).language, "en");
        assert!(Settings::load(&path).share_proposals);
        std::fs::write(&path, "{ broken").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(folder.join("settings.json.broken").exists());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
