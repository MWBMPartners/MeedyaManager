// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Auto-Update Version Checker (mm-update)
//
// Milestone 8 — Packaging & Public Beta.
//
// This crate queries the GitHub Releases API to check for newer versions of
// MeedyaManager and surface an update notification in each platform UI.
//
// Usage (from any platform crate):
//
//   use mm_update::UpdateChecker;
//
//   let checker = UpdateChecker::new(env!("CARGO_PKG_VERSION"))?;
//   if let Some(release) = checker.check().await? {
//       println!("Update available: {} — {}", release.version, release.release_url);
//   }

// --- Module declarations ---

/// Parsed release information returned after a successful API check.
pub mod release;

/// `UpdateChecker` — queries the GitHub API and compares versions.
pub mod checker;

// --- Re-exports ---

pub use checker::UpdateChecker;
pub use release::ReleaseInfo;

// --- Error type ---

/// All errors that can arise during an update check.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The running or remote version string could not be parsed as semver.
    #[error("version parse error: {0}")]
    VersionParse(String),

    /// A network error occurred while contacting the GitHub API.
    #[error("network error: {0}")]
    Network(String),

    /// The GitHub API response could not be deserialized.
    #[error("parse error: {0}")]
    Parse(String),

    /// No published releases were found in the repository.
    #[error("no releases found")]
    NoReleasesFound,

    /// The GitHub API returned a rate-limit response (HTTP 429).
    #[error("GitHub API rate limit exceeded — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
}

impl UpdateError {
    /// Returns `true` if the error is transient and worth retrying.
    pub fn is_retryable(&self) -> bool {
        matches!(self, UpdateError::Network(_) | UpdateError::RateLimited { .. })
    }
}

// ---------------------------------------------------------------------------
// Integration / smoke tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_error_version_parse_not_retryable() {
        let e = UpdateError::VersionParse("bad".into());
        assert!(!e.is_retryable());
    }

    #[test]
    fn update_error_network_is_retryable() {
        let e = UpdateError::Network("timeout".into());
        assert!(e.is_retryable());
    }

    #[test]
    fn update_error_rate_limited_is_retryable() {
        let e = UpdateError::RateLimited { retry_after_secs: 60 };
        assert!(e.is_retryable());
    }

    #[test]
    fn update_error_no_releases_not_retryable() {
        assert!(!UpdateError::NoReleasesFound.is_retryable());
    }

    #[test]
    fn checker_constructs_from_valid_version() {
        assert!(UpdateChecker::new("0.8.0").is_ok());
    }

    #[test]
    fn checker_fails_on_invalid_version() {
        assert!(UpdateChecker::new("invalid").is_err());
    }

    #[test]
    fn checker_detects_newer_version() {
        let c = UpdateChecker::new("0.8.0").unwrap();
        assert!(c.is_newer(&semver::Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn checker_does_not_flag_same_version() {
        let c = UpdateChecker::new("0.9.0").unwrap();
        assert!(!c.is_newer(&semver::Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn release_info_round_trips_through_serde() {
        let info = ReleaseInfo {
            tag:          "v0.9.0".into(),
            version:      "0.9.0".into(),
            name:         "MeedyaManager v0.9.0".into(),
            changelog:    "Bug fixes and improvements".into(),
            is_prerelease: false,
            published_at: "2026-03-10T12:00:00Z".into(),
            release_url:  "https://github.com/example/releases/tag/v0.9.0".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ReleaseInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }
}
