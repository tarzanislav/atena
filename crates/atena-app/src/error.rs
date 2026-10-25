//! Errors as the user interface gets them: a code it translates (`error-<code>` in the Fluent
//! files) and a technical detail in English for those who look further.

use atena_ingest::import::{ConfirmError, DecisionError};
use atena_ingest::register::SharedRegisterError;
use atena_search::SearchError;
use atena_store::StoreError;
use atena_vault::VaultError;
use serde::Serialize;

/// An error of the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{code}: {detail}")]
pub struct AppError {
    /// What went wrong, as a code: see [`CODES`].
    pub code: &'static str,
    /// The technical detail.
    pub detail: String,
}

/// Every code an [`AppError`] can have; the user interface has a text for each.
pub const CODES: &[&str] = &[
    "already-running",
    "crawl",
    "database",
    "database-start",
    "database-version",
    "decision-country",
    "decision-incomplete",
    "decision-licence",
    "decision-official",
    "decision-terms",
    "editor-missing",
    "index",
    "index-foreign",
    "input-too-long",
    "internal",
    "io",
    "not-a-list",
    "not-found",
    "not-ready",
    "private-damaged",
    "private-exists",
    "private-keychain",
    "private-locked",
    "private-login",
    "private-name",
    "private-none",
    "private-recovery",
    "private-remember-none",
    "private-weak",
    "reason-missing",
    "search",
    "search-empty",
    "search-filter",
    "settings",
    "shared-register-invalid",
    "shared-register-unreachable",
    "url",
];

impl AppError {
    /// An error with `code` (one of [`CODES`]).
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        debug_assert!(CODES.contains(&code), "unknown error code {code}");
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// A file or folder could not be read or written.
    pub fn io(path: &std::path::Path, error: impl std::fmt::Display) -> Self {
        Self::new("io", format!("{}: {error}", path.display()))
    }
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        Self::new("database", e.to_string())
    }
}

impl From<SearchError> for AppError {
    fn from(e: SearchError) -> Self {
        let code = match &e {
            SearchError::NotAnIndex(_) => "index-foreign",
            SearchError::EmptyQuery => "search-empty",
            SearchError::Filter(_) | SearchError::TooManyFilters(_) => "search-filter",
            SearchError::Store(_) => "database",
            SearchError::Index(_) | SearchError::Io { .. } | SearchError::Outdated(_) => "index",
        };
        Self::new(code, e.to_string())
    }
}

impl From<DecisionError> for AppError {
    fn from(e: DecisionError) -> Self {
        let code = match &e {
            DecisionError::Incomplete => "decision-incomplete",
            DecisionError::Country(_) => "decision-country",
            DecisionError::Licence(_) => "decision-licence",
            DecisionError::Terms(_) => "decision-terms",
            DecisionError::Official(_) => "decision-official",
        };
        Self::new(code, e.to_string())
    }
}

impl From<ConfirmError> for AppError {
    fn from(e: ConfirmError) -> Self {
        match e {
            ConfirmError::NotFound(id) => Self::new("not-found", id),
            ConfirmError::Decision(_, decision) => decision.into(),
            ConfirmError::Store(store) => store.into(),
        }
    }
}

impl From<SharedRegisterError> for AppError {
    fn from(e: SharedRegisterError) -> Self {
        let code = match &e {
            SharedRegisterError::Fetch { .. } => "shared-register-unreachable",
            SharedRegisterError::Address(_)
            | SharedRegisterError::NotHttps(_)
            | SharedRegisterError::NotText(_)
            | SharedRegisterError::Invalid { .. } => "shared-register-invalid",
        };
        Self::new(code, e.to_string())
    }
}

impl From<VaultError> for AppError {
    fn from(e: VaultError) -> Self {
        let code = match &e {
            VaultError::NoAccount => "private-none",
            VaultError::Exists => "private-exists",
            VaultError::WrongLogin => "private-login",
            VaultError::WrongRecoveryKey => "private-recovery",
            VaultError::WeakPassword => "private-weak",
            VaultError::BadName => "private-name",
            VaultError::Damaged(_) => "private-damaged",
            VaultError::Io { .. } => "io",
            VaultError::Random(_) => "internal",
            VaultError::NoKeychain => "private-remember-none",
            VaultError::Keychain(_) => "private-keychain",
        };
        Self::new(code, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_reach_the_interface_as_code_and_detail() {
        let e = AppError::new("not-found", "U-0123456789");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({"code": "not-found", "detail": "U-0123456789"})
        );
        let e: AppError = DecisionError::Country("XX".into()).into();
        assert_eq!(e.code, "decision-country");
    }

    #[test]
    fn codes_are_sorted_and_unique() {
        let mut sorted = CODES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, CODES);
    }
}
