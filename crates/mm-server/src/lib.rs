// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Secure Media Server (M10)
//
// Serves the MeedyaManager media library over HTTPS with JWT authentication
// and HTTP byte-range streaming for media files.
//
// Architecture:
//   auth::*       — ServerConfig, JwtService, Claims, UserRole, AuthError
//   streaming::*  — RangeParser, StreamRequest, StreamResponse, MediaStreamer
//   routes::*     — REST API handler stubs + response types
//
// Transport layer (axum + rustls) is wired in the `meedya serve` CLI command
// (mm-cli/src/commands/serve.rs), which creates the axum router and binds to
// the configured HTTPS port.

// --- Module declarations ---

/// Authentication and authorisation — JWT validation, user sessions, roles.
pub mod auth;

/// Media streaming — Range header parsing, byte-range response descriptors.
pub mod streaming;

/// REST API route handlers and JSON response types.
pub mod routes;

// --- Convenience re-exports ---

pub use auth::{AuthError, Claims, JwtService, LoginRequest, LoginResponse, ServerConfig, UserRole};
pub use streaming::{MediaStreamer, RangeParser, StreamConfig, StreamError, StreamRequest, StreamResponse};
pub use routes::{
    ApiResponse, HealthResponse, LibraryItem, LibraryResponse,
    SearchQuery, ServerInfoResponse,
    handle_health, handle_library, handle_library_item,
    handle_login, handle_search, handle_server_info, handle_stream,
};

// ---------------------------------------------------------------------------
// Integration / smoke tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "integration-test-secret-32-bytes!!";

    fn make_admin_token() -> String {
        JwtService::new(SECRET, 3600)
            .unwrap()
            .issue("admin", UserRole::Admin)
            .unwrap()
    }

    fn validate(token: &str) -> Claims {
        JwtService::new(SECRET, 3600).unwrap().validate(token).unwrap()
    }

    // ---- Auth integration ----

    #[test]
    fn jwt_round_trip_all_roles() {
        let svc = JwtService::new(SECRET, 3600).unwrap();
        for role in [UserRole::Admin, UserRole::User, UserRole::ReadOnly] {
            let token = svc.issue("testuser", role.clone()).unwrap();
            let claims = svc.validate(&token).unwrap();
            assert_eq!(claims.role, role);
            assert!(claims.is_valid());
        }
    }

    #[test]
    fn login_handler_issues_valid_jwt() {
        let req = LoginRequest { username: "alice".into(), password: "pw".into() };
        let resp = handle_login(&req, SECRET, 3600).unwrap();
        let token = &resp.data.unwrap().token;
        // The token must be parseable by JwtService
        let claims = validate(token);
        assert_eq!(claims.sub, "alice");
    }

    // ---- Route + auth integration ----

    #[test]
    fn full_library_flow_admin() {
        let claims = validate(&make_admin_token());
        let lib_resp = handle_library(&claims, 1, 50).unwrap();
        assert!(lib_resp.ok);
        let item_resp = handle_library_item(&claims, 1).unwrap();
        assert!(item_resp.ok);
    }

    #[test]
    fn search_flow() {
        let claims = validate(&make_admin_token());
        let q = SearchQuery { q: "rock".into(), limit: Some(10), page: Some(1) };
        let resp = handle_search(&claims, &q).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.data.unwrap().per_page, 10);
    }

    // ---- Streaming integration ----

    #[test]
    fn stream_range_pipeline() {
        let token = make_admin_token();
        let claims = validate(&token);
        let item = LibraryItem::new(1, "abc123", "track.flac");
        // Full request
        let resp = handle_stream(&claims, &item, None, 10_000_000).unwrap();
        assert_eq!(resp.status_code, 200);
        // Partial request
        let resp2 = handle_stream(&claims, &item, Some("bytes=0-65535"), 10_000_000).unwrap();
        assert_eq!(resp2.status_code, 206);
        assert_eq!(resp2.content_length(), 65536);
    }

    #[test]
    fn range_parser_and_stream_response_integration() {
        let range = RangeParser::parse("bytes=1000-1999").unwrap();
        let resolved = range.resolve(5000).unwrap();
        assert_eq!(resolved, (1000, 1999));
        let resp = StreamResponse::partial(resolved.0, resolved.1, 5000, "audio/flac");
        assert!(resp.is_partial());
        assert_eq!(resp.content_range.as_deref(), Some("bytes 1000-1999/5000"));
    }

    // ---- Media streamer integration ----

    #[test]
    fn media_streamer_full_pipeline() {
        let streamer = MediaStreamer::new(StreamConfig {
            media_root: "/media".into(),
            ..StreamConfig::default()
        }).unwrap();

        let resp = streamer.prepare_response(
            "music/test.flac",
            &StreamRequest::FromStart { start: 0 },
            2_000_000,
        ).unwrap();
        assert_eq!(resp.status_code, 206);
        assert_eq!(resp.content_type, "audio/flac");
    }

    // ---- Server info ----

    #[test]
    fn server_info_admin_access() {
        let claims = validate(&make_admin_token());
        let info = handle_server_info(&claims).unwrap().data.unwrap();
        assert!(info.streaming_enabled);
    }

    // ---- Health check ----

    #[test]
    fn health_no_auth_required() {
        let resp = handle_health("1.0.0", 0);
        assert!(resp.ok);
        assert_eq!(resp.data.unwrap().status, "ok");
    }

    // ---- ServerConfig ----

    #[test]
    fn server_config_validation() {
        let mut cfg = ServerConfig::default();
        assert!(!cfg.is_valid());
        cfg.tls_cert_path = "/cert.pem".into();
        cfg.tls_key_path  = "/key.pem".into();
        cfg.jwt_secret    = "s3cr3t".into();
        assert!(cfg.is_valid());
        assert_eq!(cfg.bind_addr(), "0.0.0.0:8443");
    }
}
