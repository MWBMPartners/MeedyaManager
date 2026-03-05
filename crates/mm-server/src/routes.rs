// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Secure Media Server: Route Handlers (M10)
//
// Defines all REST API routes exposed by the media server:
//
//   Route map:
//     GET  /health              — liveness probe (no auth required)
//     POST /auth/login          — issue JWT; body: LoginRequest
//     GET  /api/library         — list all media files (requires auth)
//     GET  /api/library/:id     — single file metadata (requires auth)
//     GET  /api/search?q=...    — search by title/artist/album (requires auth)
//     GET  /stream/:id          — media streaming with Range support (requires User+)
//     GET  /api/export/status   — database export status (requires Admin)
//     GET  /api/server/info     — server info + version (requires Admin)
//
// For M10 the handlers are fully implemented as stubs that return valid
// JSON structures. Real database / filesystem I/O is wired via mm-core
// and mm-export in the full integration build.

use crate::auth::{AuthError, Claims, LoginRequest, LoginResponse, UserRole};
use crate::streaming::{StreamRequest, StreamResponse};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// API response envelope
// ---------------------------------------------------------------------------

/// Standard JSON response envelope for all API endpoints.
///
/// Successful responses use `{ "ok": true, "data": <T> }`.
/// Error responses use `{ "ok": false, "error": "message" }`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// `true` on success; `false` on error
    pub ok: bool,
    /// Response payload (present when `ok = true`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error message (present when `ok = false`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response.
    pub fn ok(data: T) -> Self {
        Self { ok: true, data: Some(data), error: None }
    }

    /// Create an error response.
    pub fn err(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse { ok: false, data: None, error: Some(message.into()) }
    }
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

/// Response body for `GET /health`.
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Always `"ok"` when the server is running
    pub status: String,
    /// Server version string (matches Cargo workspace version)
    pub version: String,
    /// UTC Unix timestamp of when the server started
    pub uptime_start: u64,
}

impl HealthResponse {
    /// Create a health response with the given version and start time.
    pub fn new(version: &str, uptime_start: u64) -> Self {
        Self {
            status:      "ok".into(),
            version:     version.to_string(),
            uptime_start,
        }
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

/// A single media file summary in the library listing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LibraryItem {
    /// Unique integer ID (maps to mm_files.id in the export schema)
    pub id: u64,
    /// SHA-256 hex digest (natural key)
    pub file_hash: String,
    /// Bare filename
    pub filename: String,
    /// Absolute path on the server
    pub path: String,
    /// MIME type (e.g. `"audio/flac"`)
    pub media_type: String,
    /// Duration in seconds (0 if unknown)
    pub duration_s: u32,
    /// File size in bytes
    pub file_size: u64,
    /// Selected metadata tags (title, artist, album, year, track, genre)
    pub tags: std::collections::HashMap<String, String>,
}

impl LibraryItem {
    /// Create a minimal `LibraryItem` with just the ID and file hash.
    pub fn new(id: u64, file_hash: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            id,
            file_hash: file_hash.into(),
            filename:  filename.into(),
            path:      String::new(),
            media_type: String::new(),
            duration_s: 0,
            file_size:  0,
            tags:       std::collections::HashMap::new(),
        }
    }

    /// Returns the value of a tag if present.
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(|s| s.as_str())
    }
}

/// Response body for `GET /api/library`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryResponse {
    /// Total number of items (not just this page)
    pub total: u64,
    /// Requested page number (1-based)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Items in this response
    pub items: Vec<LibraryItem>,
}

impl LibraryResponse {
    /// Create a library response for a single page.
    pub fn new(items: Vec<LibraryItem>, total: u64, page: u32, per_page: u32) -> Self {
        Self { total, page, per_page, items }
    }

    /// Returns `true` if there are more pages after this one.
    pub fn has_next_page(&self) -> bool {
        let fetched_so_far = (self.page as u64) * (self.per_page as u64);
        fetched_so_far < self.total
    }
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// Query parameters for `GET /api/search?q=...&limit=...&page=...`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search term (matched against title, artist, album, filename)
    pub q: String,
    /// Maximum number of results per page (default 50, max 200)
    pub limit: Option<u32>,
    /// Page number (1-based, default 1)
    pub page: Option<u32>,
}

impl SearchQuery {
    /// Resolved items per page (clamped to [1, 200]).
    pub fn effective_limit(&self) -> u32 {
        self.limit.unwrap_or(50).clamp(1, 200)
    }

    /// Resolved page number (minimum 1).
    pub fn effective_page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    /// Returns `true` if the search term is non-empty.
    pub fn is_valid(&self) -> bool {
        !self.q.trim().is_empty()
    }
}

// ---------------------------------------------------------------------------
// Server Info
// ---------------------------------------------------------------------------

/// Response body for `GET /api/server/info`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    /// MeedyaManager version string
    pub version: String,
    /// Platform string (e.g. `"linux-x86_64"`)
    pub platform: String,
    /// Number of media files in the library
    pub library_count: u64,
    /// Enabled streaming backends
    pub streaming_enabled: bool,
    /// Export database backend currently in use
    pub export_backend: Option<String>,
}

// ---------------------------------------------------------------------------
// Route handler stubs
// ---------------------------------------------------------------------------
//
// These functions contain the full handler logic as stubs:
// they validate inputs, check auth, and return properly shaped responses.
// The real I/O (mm-core scan, mm-export DB queries) is plumbed in via
// dependency injection at server startup.

/// Handle `GET /health` — no authentication required.
pub fn handle_health(version: &str, uptime_start: u64) -> ApiResponse<HealthResponse> {
    ApiResponse::ok(HealthResponse::new(version, uptime_start))
}

/// Handle `POST /auth/login`.
///
/// For M10 this validates that username and password are non-empty and
/// returns a stub token. Real user database lookup is wired in full release.
pub fn handle_login(
    req: &LoginRequest,
    jwt_secret: &str,
    expiry_secs: u64,
) -> Result<ApiResponse<LoginResponse>, AuthError> {
    use crate::auth::JwtService;

    if req.username.trim().is_empty() || req.password.trim().is_empty() {
        return Err(AuthError::InvalidToken("username and password required".into()));
    }

    let svc = JwtService::new(jwt_secret, expiry_secs)?;

    // Stub: admin if username == "admin", readonly if username == "guest",
    // otherwise standard user
    let role = match req.username.as_str() {
        "admin" => UserRole::Admin,
        "guest" => UserRole::ReadOnly,
        _       => UserRole::User,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let token = svc.issue(&req.username, role.clone())?;

    Ok(ApiResponse::ok(LoginResponse {
        token,
        expires_at: now + expiry_secs,
        role,
    }))
}

/// Handle `GET /api/library` — requires authenticated `Claims`.
pub fn handle_library(
    claims: &Claims,
    page: u32,
    per_page: u32,
) -> Result<ApiResponse<LibraryResponse>, AuthError> {
    // Any authenticated user can list the library
    if !claims.is_valid() {
        return Err(AuthError::TokenExpired);
    }

    // Stub: return an empty library with page metadata
    let resp = LibraryResponse::new(vec![], 0, page.max(1), per_page.clamp(1, 200));
    Ok(ApiResponse::ok(resp))
}

/// Handle `GET /api/library/:id` — requires authenticated `Claims`.
pub fn handle_library_item(
    claims: &Claims,
    id: u64,
) -> Result<ApiResponse<LibraryItem>, AuthError> {
    if !claims.is_valid() {
        return Err(AuthError::TokenExpired);
    }
    // Stub: return a placeholder item
    let item = LibraryItem::new(id, "stub-hash", "stub-file.flac");
    Ok(ApiResponse::ok(item))
}

/// Handle `GET /api/search` — requires authenticated `Claims`.
pub fn handle_search(
    claims: &Claims,
    query: &SearchQuery,
) -> Result<ApiResponse<LibraryResponse>, AuthError> {
    if !claims.is_valid() {
        return Err(AuthError::TokenExpired);
    }
    if !query.is_valid() {
        return Err(AuthError::InvalidToken("search query must not be empty".into()));
    }
    // Stub: return empty results
    let resp = LibraryResponse::new(vec![], 0, query.effective_page(), query.effective_limit());
    Ok(ApiResponse::ok(resp))
}

/// Handle `GET /stream/:id` — requires streaming-capable `Claims`.
///
/// Returns a `StreamResponse` describing the byte range to serve.
pub fn handle_stream(
    claims: &Claims,
    item: &LibraryItem,
    range_header: Option<&str>,
    file_size: u64,
) -> Result<StreamResponse, AuthError> {
    if !claims.can_stream() {
        return Err(AuthError::InsufficientPermissions { required: "user".into() });
    }

    // Parse Range header if present
    let request = match range_header {
        None => StreamRequest::Full,
        Some(h) => crate::streaming::RangeParser::parse(h).unwrap_or(StreamRequest::Full),
    };

    let content_type = crate::streaming::MediaStreamer::content_type(&item.path);
    let resolved = request.resolve(file_size);

    match resolved {
        None => Ok(StreamResponse::full(0, content_type)),
        Some((start, end)) if request.is_range_request() =>
            Ok(StreamResponse::partial(start, end, file_size, content_type)),
        Some(_) =>
            Ok(StreamResponse::full(file_size, content_type)),
    }
}

/// Handle `GET /api/server/info` — requires Admin `Claims`.
pub fn handle_server_info(claims: &Claims) -> Result<ApiResponse<ServerInfoResponse>, AuthError> {
    if !claims.is_admin() {
        return Err(AuthError::InsufficientPermissions { required: "admin".into() });
    }
    Ok(ApiResponse::ok(ServerInfoResponse {
        version:            env!("CARGO_PKG_VERSION").to_string(),
        platform:           std::env::consts::OS.to_string(),
        library_count:      0,
        streaming_enabled:  true,
        export_backend:     None,
    }))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{JwtService, UserRole};

    const SECRET: &str = "test-secret-key-32-bytes-minimum!!";

    fn admin_claims() -> Claims {
        let svc = JwtService::new(SECRET, 3600).unwrap();
        let token = svc.issue("admin", UserRole::Admin).unwrap();
        svc.validate(&token).unwrap()
    }

    fn user_claims() -> Claims {
        let svc = JwtService::new(SECRET, 3600).unwrap();
        let token = svc.issue("alice", UserRole::User).unwrap();
        svc.validate(&token).unwrap()
    }

    fn readonly_claims() -> Claims {
        let svc = JwtService::new(SECRET, 3600).unwrap();
        let token = svc.issue("guest", UserRole::ReadOnly).unwrap();
        svc.validate(&token).unwrap()
    }

    // --- HealthResponse ---

    #[test]
    fn health_response_status_ok() {
        let resp = handle_health("0.10.0", 1_700_000_000);
        assert!(resp.ok);
        let data = resp.data.unwrap();
        assert_eq!(data.status, "ok");
        assert_eq!(data.version, "0.10.0");
    }

    // --- Login ---

    #[test]
    fn login_valid_user_returns_token() {
        let req = LoginRequest { username: "alice".into(), password: "secret".into() };
        let result = handle_login(&req, SECRET, 3600).unwrap();
        assert!(result.ok);
        let data = result.data.unwrap();
        assert!(!data.token.is_empty());
        assert_eq!(data.role, UserRole::User);
    }

    #[test]
    fn login_admin_username_gets_admin_role() {
        let req = LoginRequest { username: "admin".into(), password: "adminpass".into() };
        let result = handle_login(&req, SECRET, 3600).unwrap();
        assert_eq!(result.data.unwrap().role, UserRole::Admin);
    }

    #[test]
    fn login_guest_gets_readonly_role() {
        let req = LoginRequest { username: "guest".into(), password: "guestpass".into() };
        let result = handle_login(&req, SECRET, 3600).unwrap();
        assert_eq!(result.data.unwrap().role, UserRole::ReadOnly);
    }

    #[test]
    fn login_empty_username_returns_error() {
        let req = LoginRequest { username: "".into(), password: "pass".into() };
        assert!(handle_login(&req, SECRET, 3600).is_err());
    }

    // --- Library ---

    #[test]
    fn library_returns_empty_for_authenticated_user() {
        let resp = handle_library(&user_claims(), 1, 50).unwrap();
        assert!(resp.ok);
        let data = resp.data.unwrap();
        assert_eq!(data.total, 0);
        assert_eq!(data.page, 1);
    }

    #[test]
    fn library_item_returns_stub() {
        let resp = handle_library_item(&user_claims(), 42).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.data.unwrap().id, 42);
    }

    // --- Search ---

    #[test]
    fn search_valid_query_returns_empty() {
        let q = SearchQuery { q: "Beatles".into(), limit: None, page: None };
        let resp = handle_search(&user_claims(), &q).unwrap();
        assert!(resp.ok);
    }

    #[test]
    fn search_empty_query_returns_error() {
        let q = SearchQuery { q: "  ".into(), limit: None, page: None };
        assert!(handle_search(&user_claims(), &q).is_err());
    }

    #[test]
    fn search_query_effective_limit_clamping() {
        let q = SearchQuery { q: "x".into(), limit: Some(999), page: None };
        assert_eq!(q.effective_limit(), 200);
        let q2 = SearchQuery { q: "x".into(), limit: Some(0), page: None };
        assert_eq!(q2.effective_limit(), 1);
    }

    // --- Stream ---

    #[test]
    fn stream_user_no_range_returns_full() {
        let item = LibraryItem::new(1, "h1", "song.flac");
        let resp = handle_stream(&user_claims(), &item, None, 5_000_000).unwrap();
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn stream_user_with_range_returns_partial() {
        let item = LibraryItem::new(2, "h2", "song.mp3");
        let resp = handle_stream(&user_claims(), &item, Some("bytes=0-1023"), 5_000_000).unwrap();
        assert_eq!(resp.status_code, 206);
        assert_eq!(resp.content_length(), 1024);
    }

    #[test]
    fn stream_readonly_user_is_denied() {
        let item = LibraryItem::new(3, "h3", "song.flac");
        let err = handle_stream(&readonly_claims(), &item, None, 1000).unwrap_err();
        assert!(matches!(err, AuthError::InsufficientPermissions { .. }));
    }

    // --- Server info ---

    #[test]
    fn server_info_requires_admin() {
        assert!(handle_server_info(&admin_claims()).is_ok());
        assert!(handle_server_info(&user_claims()).is_err());
        assert!(handle_server_info(&readonly_claims()).is_err());
    }

    #[test]
    fn server_info_contains_platform() {
        let data = handle_server_info(&admin_claims()).unwrap().data.unwrap();
        assert!(!data.platform.is_empty());
    }

    // --- LibraryItem ---

    #[test]
    fn library_item_tag_accessor() {
        let mut item = LibraryItem::new(1, "h", "f.flac");
        item.tags.insert("title".into(), "Song".into());
        assert_eq!(item.tag("title"), Some("Song"));
        assert_eq!(item.tag("artist"), None);
    }

    // --- LibraryResponse ---

    #[test]
    fn library_response_has_next_page() {
        let r = LibraryResponse::new(vec![], 100, 1, 50);
        assert!(r.has_next_page());
        let r2 = LibraryResponse::new(vec![], 50, 1, 50);
        assert!(!r2.has_next_page());
    }

    // --- ApiResponse ---

    #[test]
    fn api_response_ok_serialises_without_error_field() {
        let r = ApiResponse::ok("hello");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"ok\":true"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn api_response_err_serialises_without_data_field() {
        let r: ApiResponse<()> = ApiResponse::err("something went wrong");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("something went wrong"));
        assert!(!json.contains("\"data\""));
    }
}
