// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — OneDrive Provider (Microsoft Graph API)
//
// Implements `CloudProvider` for Microsoft OneDrive using the Graph API.
// Authentication uses OAuth 2.0 device-code flow for desktop applications.
// Delta queries are used for efficient incremental change detection.
//
// API reference: https://learn.microsoft.com/graph/api/overview

use std::path::Path;
use std::time::SystemTime;

use crate::traits::{ChangeSet, CloudCapabilities, CloudError, CloudFile, CloudProvider};

// ---------------------------------------------------------------------------
// OneDriveProvider
// ---------------------------------------------------------------------------

/// Microsoft OneDrive cloud storage provider.
///
/// Uses the Microsoft Graph REST API (`https://graph.microsoft.com/v1.0`).
/// Supports delta queries for efficient incremental sync and thumbnail
/// generation for image/video files.
pub struct OneDriveProvider {
    /// OAuth 2.0 access token (short-lived, ~1 hour).
    access_token: Option<String>,
    /// OAuth 2.0 refresh token (long-lived, used to renew the access token).
    refresh_token: Option<String>,
    /// Microsoft OAuth application (client) ID.
    #[allow(dead_code)]
    client_id: String,
    /// The Graph API base URL. Overridable for testing.
    api_base: String,
    /// Whether the token has been validated against the API.
    authenticated: bool,
}

impl OneDriveProvider {
    /// Creates a new `OneDriveProvider` with the given OAuth client ID.
    ///
    /// The provider is unauthenticated until `authenticate()` is called.
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            client_id: client_id.into(),
            api_base: "https://graph.microsoft.com/v1.0".into(),
            authenticated: false,
        }
    }

    /// Injects tokens directly (used in tests and by the settings loader).
    pub fn set_tokens(
        &mut self,
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
    ) {
        self.access_token = Some(access_token.into());
        self.refresh_token = Some(refresh_token.into());
        self.authenticated = true;
    }

    /// Constructs the Graph API `driveItem` list URL for the given path.
    fn items_url(&self, path: &str) -> String {
        let clean = path.trim_start_matches('/');
        if clean.is_empty() {
            format!("{}/me/drive/root/children", self.api_base)
        } else {
            format!("{}/me/drive/root:/{}:/children", self.api_base, clean)
        }
    }

    /// Constructs the delta URL for incremental change detection.
    fn delta_url(&self, path: &str, cursor: Option<&str>) -> String {
        if let Some(c) = cursor {
            // Cursor is a full URL returned by Graph API.
            c.to_string()
        } else {
            let clean = path.trim_start_matches('/');
            if clean.is_empty() {
                format!("{}/me/drive/root/delta", self.api_base)
            } else {
                format!("{}/me/drive/root:/{}:/delta", self.api_base, clean)
            }
        }
    }

    /// Converts a raw Graph API `driveItem` JSON value into a `CloudFile`.
    /// In production this parses `reqwest::Response` JSON; here it is a stub
    /// that would be filled in with full `serde_json` parsing.
    #[allow(dead_code)]
    fn parse_drive_item(id: &str, name: &str, path: &str, size: u64, is_folder: bool) -> CloudFile {
        CloudFile {
            id: id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            size: if is_folder { None } else { Some(size) },
            modified: Some(SystemTime::UNIX_EPOCH),
            is_folder,
            mime_type: None,
            hash: None,
            download_url: None,
        }
    }
}

impl CloudProvider for OneDriveProvider {
    fn name(&self) -> &'static str {
        "OneDrive"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated && self.access_token.is_some()
    }

    fn capabilities(&self) -> CloudCapabilities {
        // OneDrive supports delta queries, webhooks (subscriptions), and thumbnails.
        CloudCapabilities::full()
    }

    async fn authenticate(&mut self) -> Result<(), CloudError> {
        // Production implementation would initiate the OAuth device-code flow:
        // 1. POST to https://login.microsoftonline.com/common/oauth2/v2.0/devicecode
        // 2. Display the user code and verification URL.
        // 3. Poll https://login.microsoftonline.com/common/oauth2/v2.0/token
        //    until the user completes the flow.
        // For now, return Unsupported so the UI can show "Connect" state.
        Err(CloudError::Unsupported(
            "OAuth device-code flow requires user interaction — use the Cloud tab Connect button"
                .into(),
        ))
    }

    async fn refresh_token(&mut self) -> Result<(), CloudError> {
        // Production implementation would POST to:
        // https://login.microsoftonline.com/common/oauth2/v2.0/token
        // with grant_type=refresh_token and the stored refresh_token.
        let Some(_rt) = &self.refresh_token else {
            return Err(CloudError::Auth("no refresh token stored".into()));
        };
        // Stub: in production this would update self.access_token with the new token.
        Err(CloudError::Unsupported(
            "token refresh requires live HTTP client".into(),
        ))
    }

    async fn list_files(&self, path: &str) -> Result<Vec<CloudFile>, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET {items_url(path)} with Authorization: Bearer {token}
        // Parse the `value` array of driveItem objects from the JSON response.
        let _url = self.items_url(path);
        // Stub: return an empty list until live HTTP is wired up.
        Ok(Vec::new())
    }

    async fn get_file(&self, id: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET /me/drive/items/{id}
        Err(CloudError::NotFound(id.to_string()))
    }

    async fn download_file(&self, file: &CloudFile, _dest: &Path) -> Result<(), CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET /me/drive/items/{id}/content — redirect to CDN URL.
        Err(CloudError::Unsupported(format!(
            "download not yet wired up for {}",
            file.name
        )))
    }

    async fn upload_file(&self, src: &Path, _dest_path: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: PUT /me/drive/root:/{dest_path}:/content for small files,
        // or multi-session upload for >4 MiB files.
        Err(CloudError::Unsupported(format!(
            "upload not yet wired up for {}",
            src.display()
        )))
    }

    async fn watch_changes(
        &self,
        path: &str,
        cursor: Option<&str>,
    ) -> Result<ChangeSet, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET {delta_url(path, cursor)} — follows @odata.nextLink until
        // @odata.deltaLink is returned (which becomes the next cursor).
        let _url = self.delta_url(path, cursor);
        Ok(ChangeSet::default())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        // Revoke stored tokens and reset state.
        self.access_token = None;
        self.refresh_token = None;
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

    fn unauthenticated() -> OneDriveProvider {
        OneDriveProvider::new("test-client-id")
    }

    fn authenticated() -> OneDriveProvider {
        let mut p = unauthenticated();
        p.set_tokens("access_tok", "refresh_tok");
        p
    }

    #[test]
    fn name_is_onedrive() {
        assert_eq!(unauthenticated().name(), "OneDrive");
    }

    #[test]
    fn not_authenticated_by_default() {
        assert!(!unauthenticated().is_authenticated());
    }

    #[test]
    fn set_tokens_marks_authenticated() {
        assert!(authenticated().is_authenticated());
    }

    #[test]
    fn capabilities_supports_delta() {
        assert!(unauthenticated().capabilities().supports_delta);
    }

    #[test]
    fn capabilities_supports_webhooks() {
        assert!(unauthenticated().capabilities().supports_webhooks);
    }

    #[test]
    fn capabilities_supports_thumbnails() {
        assert!(unauthenticated().capabilities().supports_thumbnails);
    }

    #[tokio::test]
    async fn authenticate_returns_unsupported() {
        let mut p = unauthenticated();
        let r = p.authenticate().await;
        assert!(matches!(r, Err(CloudError::Unsupported(_))));
    }

    #[tokio::test]
    async fn refresh_token_fails_without_stored_token() {
        let mut p = unauthenticated();
        let r = p.refresh_token().await;
        assert!(matches!(r, Err(CloudError::Auth(_))));
    }

    #[tokio::test]
    async fn list_files_fails_when_not_authenticated() {
        let p = unauthenticated();
        let r = p.list_files("/Music").await;
        assert!(matches!(r, Err(CloudError::Auth(_))));
    }

    #[tokio::test]
    async fn list_files_returns_empty_when_authenticated() {
        let p = authenticated();
        let r = p.list_files("/Music").await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_empty());
    }

    #[tokio::test]
    async fn watch_changes_returns_empty_changeset() {
        let p = authenticated();
        let cs = p.watch_changes("/Music", None).await.unwrap();
        assert!(cs.is_empty());
    }

    #[tokio::test]
    async fn disconnect_clears_auth() {
        let mut p = authenticated();
        p.disconnect().await.unwrap();
        assert!(!p.is_authenticated());
    }

    #[test]
    fn items_url_root() {
        let p = unauthenticated();
        let url = p.items_url("");
        assert!(url.ends_with("/me/drive/root/children"));
    }

    #[test]
    fn items_url_with_path() {
        let p = unauthenticated();
        let url = p.items_url("/Music/2024");
        assert!(url.contains("Music/2024"));
    }

    #[test]
    fn delta_url_with_cursor_returns_cursor_unchanged() {
        let p = unauthenticated();
        let cursor = "https://graph.microsoft.com/v1.0/me/drive/root/delta?token=XYZ";
        assert_eq!(p.delta_url("/Music", Some(cursor)), cursor);
    }
}
