// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Release Info Types (mm-update)
//
// Deserialization types for the GitHub Releases API response and
// higher-level `ReleaseInfo` returned to callers.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GitHub API response types (partial — only fields we need)
// ---------------------------------------------------------------------------

/// Minimal representation of a GitHub Releases API response object.
/// Only the fields needed by the update checker are included.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    /// The git tag name, e.g. `"v0.9.0"`.
    pub tag_name: String,
    /// Human-readable release name, e.g. `"MeedyaManager v0.9.0"`.
    pub name: Option<String>,
    /// Markdown body (release notes / changelog excerpt).
    pub body: Option<String>,
    /// `true` if this is a draft release (not yet published).
    pub draft: bool,
    /// `true` if this is a pre-release (alpha, beta, rc).
    pub prerelease: bool,
    /// ISO 8601 publication timestamp.
    pub published_at: Option<String>,
    /// URL to the release page on github.com.
    pub html_url: String,
}

// ---------------------------------------------------------------------------
// Parsed release info
// ---------------------------------------------------------------------------

/// Parsed release information returned to callers after fetching the
/// latest release from the GitHub API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// Raw tag string as returned by GitHub (e.g. `"v0.9.0"`).
    pub tag: String,
    /// The version string without the `v` prefix (e.g. `"0.9.0"`).
    pub version: String,
    /// Human-readable release name (falls back to tag if absent).
    pub name: String,
    /// A truncated changelog excerpt (first 500 chars of the body).
    pub changelog: String,
    /// Whether this release is marked as a pre-release.
    pub is_prerelease: bool,
    /// ISO 8601 publication timestamp (empty string if unavailable).
    pub published_at: String,
    /// URL to the full release page.
    pub release_url: String,
}

impl ReleaseInfo {
    /// Constructs a `ReleaseInfo` from a raw `GitHubRelease` API response.
    pub fn from_github(r: &GitHubRelease) -> Self {
        // Strip the leading 'v' from the tag to get the bare semver string.
        let version = r.tag_name.trim_start_matches('v').to_string();
        let name    = r.name.clone().unwrap_or_else(|| r.tag_name.clone());

        // Truncate the body to avoid overwhelming the UI.
        let changelog = r.body
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(500)
            .collect::<String>();

        Self {
            tag:          r.tag_name.clone(),
            version,
            name,
            changelog,
            is_prerelease: r.prerelease,
            published_at:  r.published_at.clone().unwrap_or_default(),
            release_url:   r.html_url.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gh_release() -> GitHubRelease {
        GitHubRelease {
            tag_name:     "v0.9.0".into(),
            name:         Some("MeedyaManager v0.9.0".into()),
            body:         Some("## What's new\n- Feature A\n- Feature B".into()),
            draft:        false,
            prerelease:   false,
            published_at: Some("2026-03-10T12:00:00Z".into()),
            html_url:     "https://github.com/MWBMPartners/MeedyaManager/releases/tag/v0.9.0".into(),
        }
    }

    #[test]
    fn from_github_strips_v_prefix() {
        let info = ReleaseInfo::from_github(&sample_gh_release());
        assert_eq!(info.version, "0.9.0");
    }

    #[test]
    fn from_github_preserves_tag() {
        let info = ReleaseInfo::from_github(&sample_gh_release());
        assert_eq!(info.tag, "v0.9.0");
    }

    #[test]
    fn from_github_uses_name_field() {
        let info = ReleaseInfo::from_github(&sample_gh_release());
        assert_eq!(info.name, "MeedyaManager v0.9.0");
    }

    #[test]
    fn from_github_falls_back_to_tag_when_name_absent() {
        let mut r = sample_gh_release();
        r.name = None;
        let info = ReleaseInfo::from_github(&r);
        assert_eq!(info.name, "v0.9.0");
    }

    #[test]
    fn from_github_truncates_long_body() {
        let long_body: String = "x".repeat(1000);
        let mut r = sample_gh_release();
        r.body = Some(long_body);
        let info = ReleaseInfo::from_github(&r);
        assert_eq!(info.changelog.len(), 500);
    }

    #[test]
    fn from_github_handles_empty_body() {
        let mut r = sample_gh_release();
        r.body = None;
        let info = ReleaseInfo::from_github(&r);
        assert!(info.changelog.is_empty());
    }

    #[test]
    fn from_github_stores_prerelease_flag() {
        let mut r = sample_gh_release();
        r.prerelease = true;
        let info = ReleaseInfo::from_github(&r);
        assert!(info.is_prerelease);
    }

    #[test]
    fn from_github_stores_published_at() {
        let info = ReleaseInfo::from_github(&sample_gh_release());
        assert_eq!(info.published_at, "2026-03-10T12:00:00Z");
    }

    #[test]
    fn from_github_stores_release_url() {
        let info = ReleaseInfo::from_github(&sample_gh_release());
        assert!(info.release_url.contains("v0.9.0"));
    }
}
