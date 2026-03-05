// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — MEGA Provider (Stub)
//
// MEGA does not offer an officially supported REST API or Rust SDK.
// The community `mega` crate is immature and not suitable for production use.
//
// This stub implements `CloudProvider` to make the module tree compile and
// to surface a clear "Coming Soon" message in the UI. The provider will be
// wired up fully in M8 or a subsequent milestone once a stable API path exists.
//
// Issue: https://github.com/MWBM-Partners-Ltd/MeedyaManager/issues/TBD

use std::path::Path;

use crate::traits::{
    ChangeSet, CloudCapabilities, CloudError, CloudFile, CloudProvider,
};

// ---------------------------------------------------------------------------
// MegaProvider
// ---------------------------------------------------------------------------

/// MEGA cloud storage provider (stub — not yet implemented).
///
/// MEGA uses a proprietary encrypted API without an official Rust binding.
/// Marked as `Unsupported` until a stable integration path is available.
pub struct MegaProvider {
    /// Whether the user has attempted authentication (always `false` in stub).
    authenticated: bool,
}

impl MegaProvider {
    /// Creates a new `MegaProvider` stub.
    pub fn new() -> Self {
        Self { authenticated: false }
    }
}

impl Default for MegaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudProvider for MegaProvider {
    fn name(&self) -> &str {
        "MEGA"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn capabilities(&self) -> CloudCapabilities {
        // MEGA stub supports nothing yet.
        CloudCapabilities::polling_only()
    }

    async fn authenticate(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Unsupported(
            "MEGA integration is coming soon — no official API is currently available".into(),
        ))
    }

    async fn refresh_token(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn list_files(&self, _path: &str) -> Result<Vec<CloudFile>, CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn get_file(&self, _id: &str) -> Result<CloudFile, CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn download_file(&self, _file: &CloudFile, _dest: &Path) -> Result<(), CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn upload_file(&self, _src: &Path, _dest_path: &str) -> Result<CloudFile, CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn watch_changes(
        &self,
        _path: &str,
        _cursor: Option<&str>,
    ) -> Result<ChangeSet, CloudError> {
        Err(CloudError::Unsupported("MEGA not implemented".into()))
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        self.authenticated = false;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_mega() {
        assert_eq!(MegaProvider::new().name(), "MEGA");
    }

    #[test]
    fn not_authenticated_by_default() {
        assert!(!MegaProvider::new().is_authenticated());
    }

    #[test]
    fn capabilities_polling_only() {
        let caps = MegaProvider::new().capabilities();
        assert!(!caps.supports_delta);
        assert!(!caps.supports_webhooks);
    }

    #[tokio::test]
    async fn authenticate_returns_unsupported() {
        let mut p = MegaProvider::new();
        assert!(matches!(p.authenticate().await, Err(CloudError::Unsupported(_))));
    }

    #[tokio::test]
    async fn list_files_returns_unsupported() {
        assert!(matches!(
            MegaProvider::new().list_files("/").await,
            Err(CloudError::Unsupported(_))
        ));
    }

    #[tokio::test]
    async fn disconnect_succeeds() {
        let mut p = MegaProvider::new();
        assert!(p.disconnect().await.is_ok());
    }
}
