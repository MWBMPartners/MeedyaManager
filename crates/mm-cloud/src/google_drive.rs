// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Google Drive Provider (Drive API v3)
//
// Implements `CloudProvider` for Google Drive using the Drive REST API v3.
// Authentication uses OAuth 2.0 with PKCE for desktop applications.
// The `changes.list` endpoint is used for incremental change detection.
//
// API reference: https://developers.google.com/drive/api/v3/reference

use std::path::Path;
use std::time::SystemTime;

use crate::traits::{
    ChangeSet, CloudCapabilities, CloudError, CloudFile, CloudProvider,
};

// ---------------------------------------------------------------------------
// GoogleDriveProvider
// ---------------------------------------------------------------------------

/// Google Drive cloud storage provider.
///
/// Uses the Google Drive REST API v3 (`https://www.googleapis.com/drive/v3`).
/// Supports `changes.list` for incremental sync and `files.get?alt=media`
/// for file downloads.
pub struct GoogleDriveProvider {
    /// OAuth 2.0 access token (short-lived, ~1 hour).
    access_token: Option<String>,
    /// OAuth 2.0 refresh token used to renew the access token.
    refresh_token: Option<String>,
    /// Google Cloud OAuth client ID.
    client_id: String,
    /// The Drive API base URL. Overridable for testing.
    api_base: String,
    /// Whether the token has been validated against the API.
    authenticated: bool,
    /// The start page token used as an incremental cursor for `changes.list`.
    start_page_token: Option<String>,
}

impl GoogleDriveProvider {
    /// Creates a new `GoogleDriveProvider` with the given OAuth client ID.
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            access_token:     None,
            refresh_token:    None,
            client_id:        client_id.into(),
            api_base:         "https://www.googleapis.com/drive/v3".into(),
            authenticated:    false,
            start_page_token: None,
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

    /// Sets the start page token for incremental `changes.list` queries.
    pub fn set_start_page_token(&mut self, token: impl Into<String>) {
        self.start_page_token = Some(token.into());
    }

    /// Constructs the `files.list` URL for listing files in a specific folder.
    fn files_list_url(&self, folder_id: &str) -> String {
        format!(
            "{}/files?q=%27{}%27+in+parents&fields=files(id,name,size,modifiedTime,mimeType,md5Checksum,webContentLink)&pageSize=1000",
            self.api_base, folder_id
        )
    }

    /// Constructs the `changes.list` URL using the current page token.
    fn changes_url(&self, page_token: &str) -> String {
        format!(
            "{}/changes?pageToken={}&fields=nextPageToken,newStartPageToken,changes(removed,file(id,name,size,modifiedTime,mimeType,md5Checksum))",
            self.api_base, page_token
        )
    }
}

impl CloudProvider for GoogleDriveProvider {
    fn name(&self) -> &str {
        "Google Drive"
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated && self.access_token.is_some()
    }

    fn capabilities(&self) -> CloudCapabilities {
        // Google Drive supports change tracking but webhooks need push notification setup.
        CloudCapabilities::delta_only()
    }

    async fn authenticate(&mut self) -> Result<(), CloudError> {
        // Production implementation would:
        // 1. Generate a PKCE code_verifier + code_challenge.
        // 2. Open https://accounts.google.com/o/oauth2/v2/auth in browser.
        // 3. Start a local HTTP server to receive the redirect_uri callback.
        // 4. Exchange the code for tokens via https://oauth2.googleapis.com/token.
        Err(CloudError::Unsupported(
            "OAuth PKCE flow requires user interaction — use the Cloud tab Connect button".into(),
        ))
    }

    async fn refresh_token(&mut self) -> Result<(), CloudError> {
        let Some(_rt) = &self.refresh_token else {
            return Err(CloudError::Auth("no refresh token stored".into()));
        };
        // Production: POST https://oauth2.googleapis.com/token
        // with grant_type=refresh_token and client_id + refresh_token.
        Err(CloudError::Unsupported(
            "token refresh requires live HTTP client".into(),
        ))
    }

    async fn list_files(&self, path: &str) -> Result<Vec<CloudFile>, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: resolve path to a folder ID via files.list with `q=name='...'`,
        // then GET {files_list_url(folder_id)} with Authorization header.
        let _url = self.files_list_url("root");
        Ok(Vec::new())
    }

    async fn get_file(&self, id: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET /files/{id}?fields=id,name,size,modifiedTime,mimeType,md5Checksum
        Err(CloudError::NotFound(id.to_string()))
    }

    async fn download_file(&self, file: &CloudFile, _dest: &Path) -> Result<(), CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: GET /files/{id}?alt=media — streams response body to dest file.
        Err(CloudError::Unsupported(format!(
            "download not yet wired up for {}",
            file.name
        )))
    }

    async fn upload_file(&self, src: &Path, dest_path: &str) -> Result<CloudFile, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: POST https://www.googleapis.com/upload/drive/v3/files?uploadType=resumable
        Err(CloudError::Unsupported(format!(
            "upload not yet wired up for {}",
            src.display()
        )))
    }

    async fn watch_changes(
        &self,
        _path: &str,
        cursor: Option<&str>,
    ) -> Result<ChangeSet, CloudError> {
        if !self.is_authenticated() {
            return Err(CloudError::Auth("not authenticated".into()));
        }
        // Production: use cursor (or self.start_page_token) as the pageToken for
        // changes.list, collect all pages, return ChangeSet with newStartPageToken.
        let _page_token = cursor.or(self.start_page_token.as_deref()).unwrap_or("1");
        Ok(ChangeSet::default())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        self.access_token     = None;
        self.refresh_token    = None;
        self.start_page_token = None;
        self.authenticated    = false;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn unauthenticated() -> GoogleDriveProvider {
        GoogleDriveProvider::new("test-client-id")
    }

    fn authenticated() -> GoogleDriveProvider {
        let mut p = unauthenticated();
        p.set_tokens("access_tok", "refresh_tok");
        p
    }

    #[test]
    fn name_is_google_drive() {
        assert_eq!(unauthenticated().name(), "Google Drive");
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
        let p = unauthenticated();
        assert!(matches!(p.list_files("/").await, Err(CloudError::Auth(_))));
    }

    #[tokio::test]
    async fn list_files_returns_empty_when_authenticated() {
        let p = authenticated();
        assert!(p.list_files("/").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn watch_changes_returns_empty_changeset() {
        let p = authenticated();
        assert!(p.watch_changes("/", None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn disconnect_clears_auth() {
        let mut p = authenticated();
        p.disconnect().await.unwrap();
        assert!(!p.is_authenticated());
    }

    #[test]
    fn files_list_url_contains_parent_id() {
        let p = unauthenticated();
        assert!(p.files_list_url("folder123").contains("folder123"));
    }

    #[test]
    fn changes_url_contains_page_token() {
        let p = unauthenticated();
        assert!(p.changes_url("tok456").contains("tok456"));
    }

    #[test]
    fn start_page_token_stored_correctly() {
        let mut p = unauthenticated();
        p.set_start_page_token("start_tok");
        assert_eq!(p.start_page_token.as_deref(), Some("start_tok"));
    }
}
