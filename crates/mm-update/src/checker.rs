// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Update Checker (mm-update)
//
// Queries the GitHub Releases API to determine whether a newer version of
// MeedyaManager is available.  The check is performed asynchronously and
// respects the user's `include_prerelease` preference.
//
// GitHub API endpoint used:
//   GET https://api.github.com/repos/{owner}/{repo}/releases/latest
//   GET https://api.github.com/repos/{owner}/{repo}/releases  (if prerelease enabled)

use semver::Version;
use tracing::{debug, info, warn};

use crate::{
    release::{GitHubRelease, ReleaseInfo},
    UpdateError,
};

// ---------------------------------------------------------------------------
// UpdateChecker
// ---------------------------------------------------------------------------

/// Checks the GitHub Releases API for a newer version of MeedyaManager.
pub struct UpdateChecker {
    /// The running version of MeedyaManager (parsed from `CARGO_PKG_VERSION`).
    current_version: Version,
    /// GitHub repository owner.
    owner: String,
    /// GitHub repository name.
    repo: String,
    /// Whether to include pre-releases (alpha, beta, rc) in the check.
    include_prerelease: bool,
    /// Base URL for the GitHub API (overridable for testing).
    api_base: String,
}

impl UpdateChecker {
    /// Creates an `UpdateChecker` for the MeedyaManager GitHub repository.
    ///
    /// `current_version` should be the value of `CARGO_PKG_VERSION` for the
    /// crate that embeds the update check (e.g. `mm-gtk` or `mm-cli`).
    pub fn new(current_version: &str) -> Result<Self, UpdateError> {
        let version = Version::parse(current_version)
            .map_err(|e| UpdateError::VersionParse(e.to_string()))?;

        Ok(Self {
            current_version:    version,
            owner:              "MWBMPartners".into(),
            repo:               "MeedyaManager".into(),
            include_prerelease: false,
            api_base:           "https://api.github.com".into(),
        })
    }

    /// Enables or disables pre-release version checking.
    pub fn with_prerelease(mut self, include: bool) -> Self {
        self.include_prerelease = include;
        self
    }

    /// Overrides the API base URL (used in tests with `wiremock`).
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    /// Returns `true` when `candidate` is strictly newer than the running version.
    pub fn is_newer(&self, candidate: &Version) -> bool {
        candidate > &self.current_version
    }

    /// Returns the parsed `Version` of the running application.
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    /// Constructs the GitHub API URL for the latest non-prerelease release.
    pub fn latest_release_url(&self) -> String {
        format!(
            "{}/repos/{}/{}/releases/latest",
            self.api_base, self.owner, self.repo
        )
    }

    /// Constructs the GitHub API URL for the list of all releases.
    pub fn releases_list_url(&self) -> String {
        format!(
            "{}/repos/{}/{}/releases?per_page=10",
            self.api_base, self.owner, self.repo
        )
    }

    /// Fetches the latest release from the GitHub API and compares it against
    /// the running version.
    ///
    /// Returns `Ok(Some(ReleaseInfo))` when an update is available, or
    /// `Ok(None)` when already on the latest version.
    pub async fn check(&self) -> Result<Option<ReleaseInfo>, UpdateError> {
        let client = reqwest::Client::builder()
            .user_agent(format!(
                "MeedyaManager/{} (update-check; github.com/MWBMPartners/MeedyaManager)",
                self.current_version
            ))
            .build()
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        let release: GitHubRelease = if self.include_prerelease {
            // Fetch the list and pick the most recent non-draft release.
            self.fetch_latest_including_prerelease(&client).await?
        } else {
            // Fetch the single "latest" release (GitHub excludes pre-releases here).
            self.fetch_latest_stable(&client).await?
        };

        debug!(
            tag = %release.tag_name,
            prerelease = release.prerelease,
            draft = release.draft,
            "Fetched release from GitHub"
        );

        if release.draft {
            // Draft releases are unpublished — skip.
            return Ok(None);
        }

        let candidate_str = release.tag_name.trim_start_matches('v');
        let candidate = Version::parse(candidate_str)
            .map_err(|e| UpdateError::VersionParse(e.to_string()))?;

        if self.is_newer(&candidate) {
            info!(
                current = %self.current_version,
                available = %candidate,
                "Update available"
            );
            Ok(Some(ReleaseInfo::from_github(&release)))
        } else {
            debug!(
                current = %self.current_version,
                latest = %candidate,
                "Already on the latest version"
            );
            Ok(None)
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Fetches the single "latest" stable release (pre-releases excluded by GitHub).
    async fn fetch_latest_stable(
        &self,
        client: &reqwest::Client,
    ) -> Result<GitHubRelease, UpdateError> {
        let url = self.latest_release_url();
        debug!(url = %url, "GET latest release");

        let resp = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(UpdateError::NoReleasesFound);
        }
        if !resp.status().is_success() {
            return Err(UpdateError::Network(format!(
                "GitHub API returned HTTP {}",
                resp.status()
            )));
        }

        resp.json::<GitHubRelease>()
            .await
            .map_err(|e| UpdateError::Parse(e.to_string()))
    }

    /// Fetches the list of all releases and returns the first non-draft entry.
    async fn fetch_latest_including_prerelease(
        &self,
        client: &reqwest::Client,
    ) -> Result<GitHubRelease, UpdateError> {
        let url = self.releases_list_url();
        debug!(url = %url, "GET releases list");

        let resp = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(UpdateError::Network(format!(
                "GitHub API returned HTTP {}",
                resp.status()
            )));
        }

        let releases: Vec<GitHubRelease> = resp
            .json()
            .await
            .map_err(|e| UpdateError::Parse(e.to_string()))?;

        releases
            .into_iter()
            .find(|r| !r.draft)
            .ok_or(UpdateError::NoReleasesFound)
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no network — pure logic)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn checker(ver: &str) -> UpdateChecker {
        UpdateChecker::new(ver).unwrap()
    }

    // ── Version comparison ────────────────────────────────────────────────────

    #[test]
    fn is_newer_returns_true_for_higher_minor() {
        let c = checker("0.8.0");
        assert!(c.is_newer(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn is_newer_returns_true_for_higher_patch() {
        let c = checker("0.8.0");
        assert!(c.is_newer(&Version::parse("0.8.1").unwrap()));
    }

    #[test]
    fn is_newer_returns_false_for_same_version() {
        let c = checker("0.8.0");
        assert!(!c.is_newer(&Version::parse("0.8.0").unwrap()));
    }

    #[test]
    fn is_newer_returns_false_for_older_version() {
        let c = checker("0.9.0");
        assert!(!c.is_newer(&Version::parse("0.8.0").unwrap()));
    }

    #[test]
    fn is_newer_returns_true_for_major_bump() {
        let c = checker("0.9.0");
        assert!(c.is_newer(&Version::parse("1.0.0").unwrap()));
    }

    // ── Constructor ───────────────────────────────────────────────────────────

    #[test]
    fn new_parses_valid_version() {
        let c = checker("0.8.0");
        assert_eq!(c.current_version().to_string(), "0.8.0");
    }

    #[test]
    fn new_fails_for_invalid_version() {
        assert!(UpdateChecker::new("not-a-version").is_err());
    }

    #[test]
    fn with_prerelease_flag_is_stored() {
        let c = checker("0.8.0").with_prerelease(true);
        assert!(c.include_prerelease);
    }

    #[test]
    fn api_base_override_applied() {
        let c = checker("0.8.0").with_api_base("http://localhost:9999");
        assert!(c.latest_release_url().starts_with("http://localhost:9999"));
    }

    // ── URL construction ──────────────────────────────────────────────────────

    #[test]
    fn latest_release_url_contains_owner_and_repo() {
        let c = checker("0.8.0");
        let url = c.latest_release_url();
        assert!(url.contains("MWBMPartners/MeedyaManager/releases/latest"));
    }

    #[test]
    fn releases_list_url_contains_per_page() {
        let c = checker("0.8.0");
        let url = c.releases_list_url();
        assert!(url.contains("per_page=10"));
    }
}
