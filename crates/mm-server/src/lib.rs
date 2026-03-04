// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Secure Media Server
//
// This crate implements the secure media server for MeedyaManager (Milestone 10).
// It provides an HTTPS media streaming server with JWT-based authentication,
// built on Axum and Tower with Rustls for TLS termination.
//
// Features:
//   - RESTful API routes for library browsing and search
//   - JWT authentication and authorization
//   - Adaptive bitrate media streaming (range requests, chunked transfer)
//   - CORS support for web-based clients
//   - Static file serving for embedded web UI

// --- Module declarations ---

/// HTTP route handlers for the media server API.
pub mod routes;

/// Authentication and authorization — JWT validation, user sessions, permissions.
pub mod auth;

/// Media streaming — range requests, chunked transfer, adaptive bitrate.
pub mod streaming;

// --- Unit tests ---

#[cfg(test)]
mod tests {
    /// Smoke test to verify the crate compiles and the module tree is valid.
    #[test]
    fn server_crate_loads() {
        // Confirms that the mm-server crate links correctly.
        // Route and streaming tests live in each submodule.
        assert!(true, "mm-server crate loaded successfully");
    }
}
