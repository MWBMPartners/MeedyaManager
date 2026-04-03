// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — iCloud Drive Provider (Stub)
//
// iCloud Drive integration requires the Apple FileProvider framework, which is
// only available on macOS (and iOS). There is no public REST API for iCloud
// Drive that can be used from a cross-platform Rust crate.
//
// On macOS this will be handled natively by the SwiftUI layer using
// `NSFileProviderManager` — the Rust crate acts as a placeholder that surfaces
// a clear "macOS only" message when queried from Windows or Linux.
//
// Issue: https://github.com/MWBM-Partners-Ltd/MeedyaManager/issues/TBD

use std::path::Path;

use crate::traits::{ChangeSet, CloudCapabilities, CloudError, CloudFile, CloudProvider};

// ---------------------------------------------------------------------------
// ICloudProvider
// ---------------------------------------------------------------------------

/// iCloud Drive cloud storage provider (stub — macOS native only).
///
/// The actual iCloud integration is handled by the macOS SwiftUI layer via
/// `NSFileProviderManager`. This Rust stub exists to complete the module tree
/// and allow cross-platform code to reference the type without conditional
/// compilation at every call site.
pub struct ICloudProvider {
    /// `true` only on macOS where the native integration has authenticated.
    authenticated: bool,
}

impl ICloudProvider {
    /// Creates a new `ICloudProvider` stub.
    pub fn new() -> Self {
        Self {
            authenticated: false,
        }
    }

    /// Returns `true` if currently running on macOS (checked at runtime).
    pub fn is_macos() -> bool {
        // cfg! is evaluated at compile time; on non-macOS this is always false.
        cfg!(target_os = "macos")
    }
}

impl Default for ICloudProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudProvider for ICloudProvider {
    fn name(&self) -> &'static str {
        "iCloud Drive"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn capabilities(&self) -> CloudCapabilities {
        // iCloud integration delegates entirely to the macOS FileProvider.
        CloudCapabilities::polling_only()
    }

    async fn authenticate(&mut self) -> Result<(), CloudError> {
        if !Self::is_macos() {
            return Err(CloudError::Unsupported(
                "iCloud Drive is only available on macOS — use the macOS app".into(),
            ));
        }
        // On macOS the SwiftUI layer calls NSFileProviderManager; we just mark as
        // authenticated here when the callback confirms access was granted.
        Err(CloudError::Unsupported(
            "iCloud authentication is handled by the macOS native layer".into(),
        ))
    }

    async fn refresh_token(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Unsupported(
            "iCloud token refresh is managed by macOS automatically".into(),
        ))
    }

    async fn list_files(&self, _path: &str) -> Result<Vec<CloudFile>, CloudError> {
        if !Self::is_macos() {
            return Err(CloudError::Unsupported(
                "iCloud Drive is only available on macOS".into(),
            ));
        }
        Err(CloudError::Unsupported(
            "iCloud file listing is handled by the macOS native layer".into(),
        ))
    }

    async fn get_file(&self, _id: &str) -> Result<CloudFile, CloudError> {
        Err(CloudError::Unsupported(
            "iCloud not implemented in Rust layer".into(),
        ))
    }

    async fn download_file(&self, _file: &CloudFile, _dest: &Path) -> Result<(), CloudError> {
        Err(CloudError::Unsupported(
            "iCloud not implemented in Rust layer".into(),
        ))
    }

    async fn upload_file(&self, _src: &Path, _dest_path: &str) -> Result<CloudFile, CloudError> {
        Err(CloudError::Unsupported(
            "iCloud not implemented in Rust layer".into(),
        ))
    }

    async fn watch_changes(
        &self,
        _path: &str,
        _cursor: Option<&str>,
    ) -> Result<ChangeSet, CloudError> {
        Err(CloudError::Unsupported(
            "iCloud not implemented in Rust layer".into(),
        ))
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
    fn name_is_icloud_drive() {
        assert_eq!(ICloudProvider::new().name(), "iCloud Drive");
    }

    #[test]
    fn not_authenticated_by_default() {
        assert!(!ICloudProvider::new().is_authenticated());
    }

    #[test]
    fn capabilities_polling_only() {
        let caps = ICloudProvider::new().capabilities();
        assert!(!caps.supports_delta);
        assert!(!caps.supports_webhooks);
    }

    #[tokio::test]
    async fn authenticate_returns_unsupported_on_non_macos() {
        // On CI (Linux/Windows) this will always return Unsupported.
        let mut p = ICloudProvider::new();
        assert!(matches!(
            p.authenticate().await,
            Err(CloudError::Unsupported(_))
        ));
    }

    #[tokio::test]
    async fn list_files_returns_unsupported() {
        assert!(matches!(
            ICloudProvider::new().list_files("/").await,
            Err(CloudError::Unsupported(_))
        ));
    }

    #[tokio::test]
    async fn disconnect_succeeds() {
        let mut p = ICloudProvider::new();
        assert!(p.disconnect().await.is_ok());
    }

    #[test]
    fn is_macos_matches_target_os() {
        // On Linux/Windows CI this is false; on macOS CI it is true.
        let expected = cfg!(target_os = "macos");
        assert_eq!(ICloudProvider::is_macos(), expected);
    }
}
