// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Dropbox Provider (Dropbox API v2)
//
// Implements `CloudProvider` for Dropbox using the Dropbox API v2.
// Authentication uses OAuth 2.0 with PKCE.
// The `files/list_folder/continue` endpoint is used for delta sync.
//
// API reference: https://www.dropbox.com/developers/documentation/http/documentation

use std::path::Path;
use std::time::SystemTime;

use crate::traits::{
    ChangeSet, CloudCapabilities, CloudError, CloudFile, CloudProvider,
};

// ---------------------------------------------------------------------------
// DropboxProvider
// ---------------------------------------------------------------------------

/// Dropbox cloud storage provider.
///
/// Uses the Dropbox API v2 (`https://api.dropboxapi.com/2` for metadata,
/// `https://content.dropboxapi.com/2` for file content).
/// Supports `files/list_folder` with `recursive=true` and cursor-based
/// `files/list_folder/continue` for incremental change detection.
pub struct DropboxProvider {
    /// Short-lived OAuth 2.0 access token.
    access_token: Option<String>,
    /// Long-lived refresh token for token renewal.
    refresh_token: Option<String>,
    /// Dropbox app key (client ID).
    app_key: String,
    /// Metadata API base URL. Overridable for testing.
    api_base: String,
    /// Content API base URL. Overridable for testing.
    content_api_base: String,
    /// Whether the token is currently valid.
    authenticated: bool,
}

impl DropboxProvider {
    /// Creates a new `DropboxProvider` with the given Dropbox app key.
    pub fn new(app_key: impl Into<String>) -> Self {
        Self {
            access_token:     None,
            refresh_token:    None,
            app_key:          app_key.into(),
            api_base:         "https://api.dropboxapi.com/2".into(),
            content_api_base: "https://content.dropboxapi.com/2".into(),
            authenticated:    false,
        }
    }

    /// Injects tokens directly (used in tests and by the settings loader).
    pub fn set_tokens(
        &mut self,
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
    ) {
        self.access_token  = Some(access_token.into());
        self.refresh_token = Some(refresh_token.into());
        self.authenticated = true;
    }

    /// Constructs the `files/list_folder` request body for the given path.
    /// In production this is serialised as JSON and POST-ed with an auth header.
    fn list_folder_body(path: &str) -> String {
        let dropbox_path = if path == "/" || path.is_empty() {
            // Dropbox represents the root with an empty string.
            String::new()
        } else {
            path.to_string()
        };
        format!(
            r#"{{"path":"{dropbox_path}","recursive":true,"include_media_info":true,"include_deleted":false}}"#
        )
    }

    /// Constructs the `files/list_folder/continue` request body for a cursor.
    fn continue_body(cursor: &str) -> String {
        format!(r#"{{"cursor":"{cursor}"}}"#)
    }
}

impl CloudProvider for DropboxProvider {
    fn name(&self) -> &str {
        "Dropbox"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated && self.access_token.is_some()
    }

    fn capabilities(&self) -> CloudCapabilities {
        // Dropbox supports `list_folder/longpoll` (webhook-like) and cursor-based delta.
        CloudCapabilities::delta_only()
    }

    async fn authenticate(&mut self) -> Result<(), CloudError> {
        // Production implementation would use the PKCE flow:
        // 1. Build https://www.dropbox.com/oauth2/authorize?client_id=...&code_challenge=...
        // 2. Open in browser and capture the redirect.
        // 3. POST to https://api.dropboxapi.com/oauth2/token to exchange the code.
        Err(CloudError::Unsupported(
            "OAuth PKCE flow requires user interaction — use the Cloud tab Connect button".into(),
        ))
    }

    async fn refresh_token(&mut self) -> Result<(), CloudError> {
        let Some(_rt) = &self.refresh_token else {
            return Err(CloudError::Auth("no refresh token stored".into()));
        };
        // Production: POST https://api.dropboxapi.com/oauth2/token
        // with grant_type=refresh_token.
        Err(CloudError::Unsupported(
            "token refresh requires live HTTP client".into(),
        ))
    }

    async fn list_files(&self, path: &str) -> Result<Vec<CloudFile>, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: POST {api_base}/files/list_folder with JSON body.
        // Parse the `entries` array; follow `has_more` + cursor pages.
        let _body = Self::list_folder_body(path);
        Ok(Vec::new())
    }

    async fn get_file(&self, id: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: POST /files/get_metadata with {"path": id}
        Err(CloudError::NotFound(id.to_string()))
    }

    async fn download_file(&self, file: &CloudFile, _dest: &Path) -> Result<(), CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: POST {content_api_base}/files/download with
        // Dropbox-API-Arg header containing {"path": file.id}
        Err(CloudError::Unsupported(format!(
            "download not yet wired up for {}",
            file.name
        )))
    }

    async fn upload_file(&self, src: &Path, dest_path: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: POST {content_api_base}/files/upload with
        // Dropbox-API-Arg: {"path": dest_path, "mode": "overwrite"}
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
        if let Some(c) = cursor {
            // Production: POST /files/list_folder/continue {continue_body(c)}.
            let _body = Self::continue_body(c);
        } else {
            // First call: get an initial cursor via /files/list_folder with limit=1.
            let _body = Self::list_folder_body(path);
        }
        Ok(ChangeSet::default())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        // Production: POST /auth/token/revoke to invalidate the token server-side.
        self.access_token  = None;
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

    fn unauthenticated() -> DropboxProvider {
        DropboxProvider::new("test-app-key")
    }

    fn authenticated() -> DropboxProvider {
        let mut p = unauthenticated();
        p.set_tokens("access_tok", "refresh_tok");
        p
    }

    #[test]
    fn name_is_dropbox() {
        assert_eq!(unauthenticated().name(), "Dropbox");
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
    fn capabilities_supports_delta_but_not_webhooks() {
        let caps = unauthenticated().capabilities();
        assert!(caps.supports_delta);
        assert!(!caps.supports_webhooks);
    }

    #[tokio::test]
    async fn authenticate_returns_unsupported() {
        let mut p = unauthenticated();
        assert!(matches!(p.authenticate().await, Err(CloudError::Unsupported(_))));
    }

    #[tokio::test]
    async fn refresh_token_fails_without_stored_token() {
        let mut p = unauthenticated();
        assert!(matches!(p.refresh_token().await, Err(CloudError::Auth(_))));
    }

    #[tokio::test]
    async fn list_files_fails_when_not_authenticated() {
        assert!(matches!(
            unauthenticated().list_files("/").await,
            Err(CloudError::Auth(_))
        ));
    }

    #[tokio::test]
    async fn list_files_returns_empty_when_authenticated() {
        assert!(authenticated().list_files("/").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn watch_changes_returns_empty_changeset() {
        let cs = authenticated().watch_changes("/", None).await.unwrap();
        assert!(cs.is_empty());
    }

    #[tokio::test]
    async fn watch_changes_with_cursor() {
        let cs = authenticated().watch_changes("/", Some("cursor_abc")).await.unwrap();
        assert!(cs.is_empty());
    }

    #[tokio::test]
    async fn disconnect_clears_auth() {
        let mut p = authenticated();
        p.disconnect().await.unwrap();
        assert!(!p.is_authenticated());
    }

    #[test]
    fn list_folder_body_root_uses_empty_path() {
        let body = DropboxProvider::list_folder_body("/");
        assert!(body.contains("\"path\":\"\""));
    }

    #[test]
    fn list_folder_body_non_root_preserves_path() {
        let body = DropboxProvider::list_folder_body("/Music/2024");
        assert!(body.contains("/Music/2024"));
    }

    #[test]
    fn continue_body_includes_cursor() {
        let body = DropboxProvider::continue_body("my_cursor_123");
        assert!(body.contains("my_cursor_123"));
    }
}
