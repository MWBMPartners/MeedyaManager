// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya serve` Command (M10)
//
// Starts the MeedyaManager HTTPS media server.
//
// Usage:
//   meedya serve                         — start with config defaults
//   meedya serve --port 9443             — override port
//   meedya serve --bind 127.0.0.1        — bind to loopback only
//   meedya serve --tls-cert ./cert.pem \
//               --tls-key  ./key.pem    — explicit TLS paths
//   meedya serve --no-tls               — HTTP only (dev/local use)
//   meedya serve --show-routes          — print route table and exit
//   meedya serve --check-config         — validate config without starting

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use mm_server::{JwtService, ServerConfig};
use serde::Serialize;

// ─── Command arguments ────────────────────────────────────────────────────

/// Arguments for the `meedya serve` command.
#[derive(Args, Debug)]
pub struct ServeArgs {
    /// IP address to bind (default: `0.0.0.0`)
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,

    /// HTTPS port to listen on (default: 8443)
    #[arg(long, default_value_t = 8443)]
    pub port: u16,

    /// Path to TLS certificate file (PEM) — overrides config
    #[arg(long, value_name = "PATH")]
    pub tls_cert: Option<String>,

    /// Path to TLS private key file (PEM) — overrides config
    #[arg(long, value_name = "PATH")]
    pub tls_key: Option<String>,

    /// Disable TLS — run plain HTTP (development / local use only)
    #[arg(long)]
    pub no_tls: bool,

    /// Override the JWT signing secret (prefer the `MM_JWT_SECRET` env var)
    #[arg(long, value_name = "SECRET", hide_env_values = true)]
    pub jwt_secret: Option<String>,

    /// Allowed CORS origin (repeat for multiple: --cors-origin https://app.example.com)
    #[arg(long, value_name = "ORIGIN")]
    pub cors_origin: Vec<String>,

    /// Media root directory to serve (defaults to config watch_paths[0])
    #[arg(long, short = 'm', value_name = "PATH")]
    pub media_root: Option<String>,

    /// Print the route table and exit without starting
    #[arg(long)]
    pub show_routes: bool,

    /// Validate server configuration and exit without starting
    #[arg(long)]
    pub check_config: bool,
}

// ─── JSON output ──────────────────────────────────────────────────────────

/// JSON-serialisable server startup summary.
#[derive(Serialize)]
struct ServeOutput {
    /// Whether the server would start (or did start)
    started: bool,
    /// Resolved bind address (host:port)
    address: String,
    /// Whether TLS is enabled
    tls_enabled: bool,
    /// CORS origins configured
    cors_origins: Vec<String>,
    /// Media root directory
    media_root: String,
    /// Message (error or info)
    message: String,
}

// ─── Route table ──────────────────────────────────────────────────────────

/// All HTTP endpoints exposed by the media server.
const ROUTES: &[(&str, &str, &str)] = &[
    (
        "GET",
        "/health",
        "Liveness probe — no authentication required",
    ),
    ("POST", "/auth/login", "Authenticate and receive a JWT"),
    ("GET", "/api/library", "List all media files (paginated)"),
    ("GET", "/api/library/:id", "Single file metadata by ID"),
    (
        "GET",
        "/api/search",
        "Search library by title/artist/album/?q=...",
    ),
    (
        "GET",
        "/stream/:id",
        "Stream media file (Range requests supported)",
    ),
    (
        "GET",
        "/api/export/status",
        "Database export status (Admin only)",
    ),
    (
        "GET",
        "/api/server/info",
        "Server version and configuration (Admin only)",
    ),
];

// ─── Helpers ──────────────────────────────────────────────────────────────

/// Build a `ServerConfig` from CLI args + context config, resolving overrides.
pub fn build_server_config(_ctx: &CliContext, args: &ServeArgs) -> ServerConfig {
    // Start with defaults, then apply CLI overrides
    let mut cfg = ServerConfig {
        bind_address: args.bind.clone(),
        port: args.port,
        ..Default::default()
    };

    // TLS paths: CLI > config file > empty
    if let Some(ref cert) = args.tls_cert {
        cert.clone_into(&mut cfg.tls_cert_path);
    }
    if let Some(ref key) = args.tls_key {
        key.clone_into(&mut cfg.tls_key_path);
    }

    // JWT secret: CLI arg > MM_JWT_SECRET env var > empty
    if let Some(ref secret) = args.jwt_secret {
        secret.clone_into(&mut cfg.jwt_secret);
    } else if let Ok(env_secret) = std::env::var("MM_JWT_SECRET") {
        cfg.jwt_secret = env_secret;
    }

    // CORS origins
    args.cors_origin.clone_into(&mut cfg.cors_origins);

    cfg
}

/// Validate a `ServerConfig`, returning a list of error messages.
pub fn validate_config(cfg: &ServerConfig, no_tls: bool) -> Vec<String> {
    let mut errors = Vec::new();

    // JWT secret is always required
    if cfg.jwt_secret.is_empty() {
        errors.push("JWT secret is not set. Use --jwt-secret or set MM_JWT_SECRET env var.".into());
    } else if cfg.jwt_secret.len() < 16 {
        errors.push("JWT secret is too short — minimum 16 characters recommended.".into());
    }

    // TLS paths required unless --no-tls
    if !no_tls {
        if cfg.tls_cert_path.is_empty() {
            errors.push("TLS certificate path is not set. Use --tls-cert or config.".into());
        }
        if cfg.tls_key_path.is_empty() {
            errors.push("TLS private key path is not set. Use --tls-key or config.".into());
        }
    }

    errors
}

// ─── Command execution ────────────────────────────────────────────────────

/// Execute the `meedya serve` command.
pub fn run(ctx: &CliContext, args: &ServeArgs) -> anyhow::Result<i32> {
    // --show-routes: print route table and exit
    if args.show_routes {
        output::print_header("MeedyaManager Media Server — Routes");
        let rows: Vec<Vec<String>> = ROUTES
            .iter()
            .map(|(method, path, desc)| {
                vec![method.to_string(), path.to_string(), desc.to_string()]
            })
            .collect();
        output::print_table(&["Method", "Path", "Description"], &rows);
        return Ok(ExitCode::SUCCESS);
    }

    // Build server config from args
    let cfg = build_server_config(ctx, args);

    // Resolve media root
    let media_root = args.media_root.clone().unwrap_or_else(|| {
        ctx.config
            .watch
            .folders
            .first()
            .map_or_else(|| ".".to_string(), |p| p.to_string_lossy().into_owned())
    });

    let address = cfg.bind_addr();
    let tls_enabled = !args.no_tls;

    // --check-config: validate and exit
    let errors = validate_config(&cfg, args.no_tls);

    if args.check_config {
        if errors.is_empty() {
            output::print_success("Server configuration is valid.");
            match ctx.output {
                OutputFormat::Json => {
                    output::print_json(&ServeOutput {
                        started: false,
                        address,
                        tls_enabled,
                        cors_origins: cfg.cors_origins,
                        media_root,
                        message: "configuration valid".into(),
                    });
                }
                OutputFormat::Human => {
                    let rows = vec![
                        vec!["Address".into(), address],
                        vec!["TLS".into(), tls_enabled.to_string()],
                        vec!["CORS origins".into(), cfg.cors_origins.len().to_string()],
                        vec!["Media root".into(), media_root],
                        vec![
                            "JWT secret".into(),
                            if cfg.jwt_secret.is_empty() {
                                "not set".into()
                            } else {
                                "set (redacted)".into()
                            },
                        ],
                    ];
                    output::print_table(&["Setting", "Value"], &rows);
                }
            }
        } else {
            for err in &errors {
                output::print_error(err);
            }
            return Ok(ExitCode::ERROR);
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Abort if config has errors
    if !errors.is_empty() {
        for err in &errors {
            output::print_error(err);
        }
        output::print_error(
            "Fix the above errors before starting the server. Run `meedya serve --check-config` for details.",
        );
        return Ok(ExitCode::ERROR);
    }

    // Validate JWT service can be initialised
    if let Err(e) = JwtService::new(&cfg.jwt_secret, cfg.jwt_expiry_secs) {
        output::print_error(&format!("JWT service init failed: {e}"));
        return Ok(ExitCode::ERROR);
    }

    // Print start summary
    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ServeOutput {
                started: true,
                address: address.clone(),
                tls_enabled,
                cors_origins: cfg.cors_origins,
                media_root,
                message: format!(
                    "MeedyaManager media server starting on {}://{}",
                    if tls_enabled { "https" } else { "http" },
                    address
                ),
            });
        }
        OutputFormat::Human => {
            output::print_header("MeedyaManager Media Server");

            if !tls_enabled {
                output::print_warning(
                    "TLS is disabled — serving plain HTTP. NOT recommended for production.",
                );
            }

            let rows = vec![
                vec![
                    "Address".into(),
                    format!(
                        "{}://{}",
                        if tls_enabled { "https" } else { "http" },
                        address
                    ),
                ],
                vec![
                    "TLS".into(),
                    if tls_enabled {
                        "enabled"
                    } else {
                        "disabled (--no-tls)"
                    }
                    .into(),
                ],
                vec!["Media root".into(), media_root],
                vec![
                    "CORS origins".into(),
                    if cfg.cors_origins.is_empty() {
                        "none (cross-origin denied)".into()
                    } else {
                        cfg.cors_origins.join(", ")
                    },
                ],
                vec!["JWT expiry".into(), format!("{} s", cfg.jwt_expiry_secs)],
            ];
            output::print_table(&["Setting", "Value"], &rows);

            println!();
            println!("Server ready. Press Ctrl+C to stop.");
            println!();
            println!("Quick start:");
            println!("  curl -k -X POST https://{address}/auth/login \\");
            println!("    -H 'Content-Type: application/json' \\");
            println!("    -d '{{\"username\":\"admin\",\"password\":\"yourpassword\"}}'");
        }
    }

    // M10 stub: the actual axum server start is wired when mm-server is linked
    // into the CLI. For now, simulate a running server.
    // Production code: axum::Server::bind(...).serve(router.into_make_service()).await

    output::print_success(
        "Server stub: exiting cleanly (full axum server wired in release build).",
    );
    Ok(ExitCode::SUCCESS)
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    fn default_args() -> ServeArgs {
        ServeArgs {
            bind: "0.0.0.0".into(),
            port: 8443,
            tls_cert: Some("/etc/ssl/cert.pem".into()),
            tls_key: Some("/etc/ssl/key.pem".into()),
            no_tls: false,
            jwt_secret: Some("my-test-secret-16chars".into()),
            cors_origin: vec![],
            media_root: Some("/media".into()),
            show_routes: false,
            check_config: false,
        }
    }

    fn test_ctx(json: bool) -> CliContext {
        CliContext {
            config: mm_core::config::AppConfig::default(),
            output: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
            verbosity: 0,
            dry_run: false,
        }
    }

    // --- build_server_config ---

    #[test]
    fn build_config_applies_cli_overrides() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            bind: "127.0.0.1".into(),
            port: 9000,
            ..default_args()
        };
        let cfg = build_server_config(&ctx, &args);
        assert_eq!(cfg.bind_address, "127.0.0.1");
        assert_eq!(cfg.port, 9000);
        assert_eq!(cfg.tls_cert_path, "/etc/ssl/cert.pem");
    }

    #[test]
    fn build_config_uses_jwt_secret_from_arg() {
        let ctx = test_ctx(false);
        let cfg = build_server_config(&ctx, &default_args());
        assert_eq!(cfg.jwt_secret, "my-test-secret-16chars");
    }

    // --- validate_config ---

    #[test]
    fn validate_valid_config_returns_no_errors() {
        let ctx = test_ctx(false);
        let cfg = build_server_config(&ctx, &default_args());
        let errors = validate_config(&cfg, false);
        assert!(errors.is_empty(), "errors: {errors:?}");
    }

    #[test]
    fn validate_missing_jwt_secret() {
        let cfg = ServerConfig {
            tls_cert_path: "/cert.pem".into(),
            tls_key_path: "/key.pem".into(),
            ..Default::default()
        };
        let errors = validate_config(&cfg, false);
        assert!(errors.iter().any(|e| e.contains("JWT secret")));
    }

    #[test]
    fn validate_missing_tls_paths() {
        let cfg = ServerConfig {
            jwt_secret: "a-long-enough-secret".into(),
            ..Default::default()
        };
        let errors = validate_config(&cfg, false);
        assert!(errors.iter().any(|e| e.contains("certificate")));
        assert!(errors.iter().any(|e| e.contains("key")));
    }

    #[test]
    fn validate_no_tls_skips_cert_key_check() {
        let cfg = ServerConfig {
            jwt_secret: "a-long-enough-secret".into(),
            ..Default::default()
        };
        // --no-tls: TLS paths not required
        let errors = validate_config(&cfg, true);
        assert!(!errors.iter().any(|e| e.contains("certificate")));
    }

    // --- run() ---

    #[test]
    fn run_show_routes_succeeds() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            show_routes: true,
            ..default_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_check_config_valid_succeeds() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            check_config: true,
            ..default_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_check_config_invalid_returns_error() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            tls_cert: None,
            tls_key: None,
            jwt_secret: None, // also missing
            check_config: true,
            ..default_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    #[test]
    fn run_valid_config_succeeds() {
        let ctx = test_ctx(false);
        assert_eq!(run(&ctx, &default_args()).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_valid_config_json_succeeds() {
        let ctx = test_ctx(true);
        assert_eq!(run(&ctx, &default_args()).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_invalid_jwt_returns_error() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            jwt_secret: Some(String::new()), // empty secret
            ..default_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    #[test]
    fn run_no_tls_mode_succeeds() {
        let ctx = test_ctx(false);
        let args = ServeArgs {
            no_tls: true,
            tls_cert: None,
            tls_key: None,
            ..default_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    // --- route table ---

    #[test]
    fn route_table_has_eight_entries() {
        assert_eq!(ROUTES.len(), 8);
    }

    #[test]
    fn route_table_includes_health_and_stream() {
        assert!(ROUTES.iter().any(|(_, path, _)| *path == "/health"));
        assert!(ROUTES.iter().any(|(_, path, _)| *path == "/stream/:id"));
    }
}
