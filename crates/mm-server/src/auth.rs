// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Secure Media Server: Authentication & Authorization (M10)
//
// Implements JWT-based authentication for the media server:
//
//   ServerConfig  — TLS/JWT/CORS/binding configuration
//   Claims        — JWT payload: subject, expiry, role
//   UserRole      — Admin | User | ReadOnly
//   AuthError     — typed authentication failure modes
//   JwtService    — issue and validate HS256 JWTs
//   LoginRequest  — JSON body for POST /auth/login
//   LoginResponse — JSON body returned on successful login
//   AuthMiddleware — tower middleware that validates Bearer tokens

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Server Configuration
// ---------------------------------------------------------------------------

/// Full server configuration (loaded from settings.json5 / .env).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// IP address to bind (default `0.0.0.0`)
    pub bind_address: String,
    /// HTTPS port (default `8443`)
    pub port: u16,
    /// Path to TLS certificate file (PEM)
    pub tls_cert_path: String,
    /// Path to TLS private key file (PEM)
    pub tls_key_path: String,
    /// JWT signing secret (loaded from `MM_JWT_SECRET` env var or `.env`)
    pub jwt_secret: String,
    /// JWT token lifetime in seconds (default 86400 = 24 h)
    pub jwt_expiry_secs: u64,
    /// Allowed CORS origins (empty = deny cross-origin)
    pub cors_origins: Vec<String>,
    /// Maximum number of concurrent connections (default 1000)
    pub max_connections: usize,
    /// Whether to enable request logging (default true)
    pub request_logging: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".into(),
            port: 8443,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            jwt_secret: String::new(),
            jwt_expiry_secs: 86_400,
            cors_origins: Vec::new(),
            max_connections: 1000,
            request_logging: true,
        }
    }
}

impl ServerConfig {
    /// Returns `true` if the config has TLS paths and a JWT secret.
    pub fn is_valid(&self) -> bool {
        !self.tls_cert_path.is_empty()
            && !self.tls_key_path.is_empty()
            && !self.jwt_secret.is_empty()
    }

    /// Returns the full bind address string `host:port`.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.bind_address, self.port)
    }
}

// ---------------------------------------------------------------------------
// User Roles
// ---------------------------------------------------------------------------

/// Permission levels for authenticated users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Full access — can read, stream, and manage settings
    Admin,
    /// Standard user — can read and stream, cannot change settings
    User,
    /// Read-only — can browse the library and read metadata, cannot stream
    ReadOnly,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::User => write!(f, "user"),
            Self::ReadOnly => write!(f, "readonly"),
        }
    }
}

impl UserRole {
    /// Returns `true` if this role can access streaming endpoints.
    pub fn can_stream(&self) -> bool {
        matches!(self, Self::Admin | Self::User)
    }

    /// Returns `true` if this role can access admin-only endpoints.
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

// ---------------------------------------------------------------------------
// JWT Claims
// ---------------------------------------------------------------------------

/// JWT payload claims.
///
/// We use HS256 (HMAC-SHA256) for signing. The `exp` (expiry) and
/// `sub` (subject / username) claims are standard RFC 7519 fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the authenticated username
    pub sub: String,
    /// Expiry — UTC Unix timestamp after which the token is invalid
    pub exp: u64,
    /// Issued at — UTC Unix timestamp when the token was issued
    pub iat: u64,
    /// Role — the permission level of the authenticated user
    pub role: UserRole,
}

impl Claims {
    /// Returns `true` if the token has not yet expired.
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.exp > now
    }

    /// Returns `true` if the claims allow streaming access.
    pub fn can_stream(&self) -> bool {
        self.is_valid() && self.role.can_stream()
    }

    /// Returns `true` if the claims allow admin access.
    pub fn is_admin(&self) -> bool {
        self.is_valid() && self.role.is_admin()
    }
}

// ---------------------------------------------------------------------------
// Auth Errors
// ---------------------------------------------------------------------------

/// Failure modes for JWT authentication operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    /// No `Authorization: Bearer <token>` header was present
    #[error("Missing authorisation header")]
    MissingToken,

    /// The token failed JWT signature verification or is malformed
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// The token's `exp` claim is in the past
    #[error("Token has expired")]
    TokenExpired,

    /// The user's role is insufficient for the requested operation
    #[error("Insufficient permissions: requires {required}")]
    InsufficientPermissions { required: String },

    /// The JWT secret is empty — server is misconfigured
    #[error("JWT secret is not configured")]
    MissingSecret,

    /// An unexpected error occurred during token encoding
    #[error("Token encoding error: {0}")]
    EncodingError(String),
}

// ---------------------------------------------------------------------------
// Login Request / Response
// ---------------------------------------------------------------------------

/// JSON body for `POST /auth/login`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Username / identifier
    pub username: String,
    /// Password (plaintext over HTTPS — never log this field)
    pub password: String,
}

/// JSON body returned by `POST /auth/login` on success.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    /// Signed JWT — client must include as `Authorization: Bearer <token>`
    pub token: String,
    /// UTC Unix timestamp when the token expires
    pub expires_at: u64,
    /// The user's assigned role
    pub role: UserRole,
}

// ---------------------------------------------------------------------------
// JwtService
// ---------------------------------------------------------------------------

/// Issues and validates HS256 JSON Web Tokens.
#[derive(Clone)]
pub struct JwtService {
    /// HMAC encoding key derived from the JWT secret
    encoding_key: EncodingKey,
    /// HMAC decoding key derived from the JWT secret
    decoding_key: DecodingKey,
    /// Token lifetime in seconds
    expiry_secs: u64,
}

impl JwtService {
    /// Create a new `JwtService` with the given secret and expiry.
    ///
    /// Returns `AuthError::MissingSecret` if `secret` is empty.
    pub fn new(secret: &str, expiry_secs: u64) -> Result<Self, AuthError> {
        if secret.is_empty() {
            return Err(AuthError::MissingSecret);
        }
        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiry_secs,
        })
    }

    /// Issue a signed JWT for `username` with the given `role`.
    pub fn issue(&self, username: &str, role: UserRole) -> Result<String, AuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let claims = Claims {
            sub: username.to_string(),
            exp: now + self.expiry_secs,
            iat: now,
            role,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::EncodingError(e.to_string()))
    }

    /// Validate a JWT and return the embedded `Claims`.
    ///
    /// Returns `AuthError::InvalidToken` if the signature is wrong or the
    /// token is malformed, and `AuthError::TokenExpired` if `exp` is past.
    pub fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        // Require `exp` claim — reject tokens without expiry
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("expired") {
                AuthError::TokenExpired
            } else {
                AuthError::InvalidToken(msg)
            }
        })?;

        Ok(token_data.claims)
    }

    /// Extract the Bearer token from an `Authorization` header value.
    ///
    /// Returns `AuthError::MissingToken` if the header is absent or
    /// does not start with `"Bearer "`.
    pub fn extract_bearer(header: Option<&str>) -> Result<&str, AuthError> {
        match header {
            None => Err(AuthError::MissingToken),
            Some(h) if h.starts_with("Bearer ") => Ok(&h[7..]),
            Some(_) => Err(AuthError::InvalidToken(
                "Authorization header must use Bearer scheme".into(),
            )),
        }
    }

    /// Returns the expiry duration in seconds.
    pub fn expiry_secs(&self) -> u64 {
        self.expiry_secs
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> JwtService {
        JwtService::new("test-secret-key-32-bytes-minimum!!", 3600).unwrap()
    }

    // --- ServerConfig ---

    #[test]
    fn server_config_default_values() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.port, 8443);
        assert_eq!(cfg.bind_address, "0.0.0.0");
        assert_eq!(cfg.jwt_expiry_secs, 86_400);
        assert!(!cfg.is_valid()); // empty TLS paths + secret
    }

    #[test]
    fn server_config_bind_addr() {
        let cfg = ServerConfig {
            bind_address: "127.0.0.1".into(),
            port: 9000,
            ..Default::default()
        };
        assert_eq!(cfg.bind_addr(), "127.0.0.1:9000");
    }

    #[test]
    fn server_config_is_valid_when_all_set() {
        let cfg = ServerConfig {
            tls_cert_path: "/etc/ssl/cert.pem".into(),
            tls_key_path: "/etc/ssl/key.pem".into(),
            jwt_secret: "my-secret".into(),
            ..ServerConfig::default()
        };
        assert!(cfg.is_valid());
    }

    // --- UserRole ---

    #[test]
    fn admin_can_stream_and_is_admin() {
        assert!(UserRole::Admin.can_stream());
        assert!(UserRole::Admin.is_admin());
    }

    #[test]
    fn user_can_stream_but_not_admin() {
        assert!(UserRole::User.can_stream());
        assert!(!UserRole::User.is_admin());
    }

    #[test]
    fn readonly_cannot_stream() {
        assert!(!UserRole::ReadOnly.can_stream());
        assert!(!UserRole::ReadOnly.is_admin());
    }

    #[test]
    fn role_display_names() {
        assert_eq!(UserRole::Admin.to_string(), "admin");
        assert_eq!(UserRole::User.to_string(), "user");
        assert_eq!(UserRole::ReadOnly.to_string(), "readonly");
    }

    // --- JwtService ---

    #[test]
    fn new_rejects_empty_secret() {
        assert!(JwtService::new("", 3600).is_err());
    }

    #[test]
    fn issue_and_validate_round_trip() {
        let token = svc().issue("alice", UserRole::User).unwrap();
        assert!(!token.is_empty());
        let claims = svc().validate(&token).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.role, UserRole::User);
        assert!(claims.is_valid());
    }

    #[test]
    fn validate_invalid_token_returns_error() {
        let result = svc().validate("not.a.valid.jwt");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidToken(_)));
    }

    #[test]
    fn extract_bearer_from_valid_header() {
        let token = JwtService::extract_bearer(Some("Bearer my-token-xyz"));
        assert_eq!(token.unwrap(), "my-token-xyz");
    }

    #[test]
    fn extract_bearer_missing_header() {
        let result = JwtService::extract_bearer(None);
        assert_eq!(result.unwrap_err(), AuthError::MissingToken);
    }

    #[test]
    fn extract_bearer_wrong_scheme() {
        let result = JwtService::extract_bearer(Some("Basic dXNlcjpwYXNz"));
        assert!(matches!(result.unwrap_err(), AuthError::InvalidToken(_)));
    }

    #[test]
    fn claims_can_stream_checks_role_and_validity() {
        let token = svc().issue("bob", UserRole::Admin).unwrap();
        let claims = svc().validate(&token).unwrap();
        assert!(claims.can_stream());
        assert!(claims.is_admin());
    }

    #[test]
    fn auth_error_display_messages() {
        assert!(
            AuthError::MissingToken
                .to_string()
                .contains("authorisation")
        );
        assert!(AuthError::TokenExpired.to_string().contains("expired"));
        assert!(AuthError::MissingSecret.to_string().contains("JWT secret"));
    }

    #[test]
    fn expiry_secs_returned_correctly() {
        let svc = JwtService::new("secret", 7200).unwrap();
        assert_eq!(svc.expiry_secs(), 7200);
    }

    // --- LoginRequest / LoginResponse ---

    #[test]
    fn login_request_serialises() {
        let req = LoginRequest {
            username: "alice".into(),
            password: "pass".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"username\""));
        assert!(json.contains("alice"));
    }

    #[test]
    fn login_response_contains_token_and_role() {
        let svc = svc();
        let token = svc.issue("carol", UserRole::ReadOnly).unwrap();
        let resp = LoginResponse {
            token,
            expires_at: 9_999_999_999,
            role: UserRole::ReadOnly,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"role\""));
        assert!(json.contains("readonly"));
    }

    #[test]
    fn insufficient_permissions_error_shows_required_role() {
        let e = AuthError::InsufficientPermissions {
            required: "admin".into(),
        };
        assert!(e.to_string().contains("admin"));
    }
}
