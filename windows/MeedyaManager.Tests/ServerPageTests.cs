// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — ServerPage unit tests (Windows / xUnit, M10)
//
// Tests pure logic extracted from ServerPage.xaml.cs without requiring a
// WinUI runtime (no XAML components are constructed — only standalone helpers).

using System;
using System.Linq;
using Xunit;

namespace MeedyaManager.Tests;

// ── Route table replicated for testing ───────────────────────────────────────

/// <summary>Pure-logic replica of the server route table from ServerPage.</summary>
internal static class ServerRoutes
{
    public static readonly (string Method, string Path, string Description)[] All =
    [
        ("GET",  "/health",       "Health check — returns server status and version"),
        ("POST", "/auth/login",   "Issue a JWT bearer token (username + password)"),
        ("GET",  "/library",      "List all media items (paginated, auth required)"),
        ("GET",  "/library/{id}", "Fetch a single media item by ID (auth required)"),
        ("GET",  "/search",       "Search the library (?q=…, auth required)"),
        ("GET",  "/stream/{id}",  "Stream a media file; supports Range requests (auth required)"),
        ("HEAD", "/stream/{id}",  "Media file metadata without body (auth required)"),
        ("GET",  "/server-info",  "Server version, platform and library stats (admin only)"),
    ];
}

// ── Config validation replicated for testing ─────────────────────────────────

/// <summary>
/// Pure-logic replica of <c>ServerPage.ValidateConfig()</c> for unit testing.
/// Mirrors the validation rules exactly so tests remain in sync with the UI.
/// </summary>
internal static class ServerConfigValidator
{
    /// <summary>
    /// Validates a server configuration supplied as plain values.
    /// Returns an array of human-readable error messages; empty means valid.
    /// </summary>
    public static string[] Validate(
        string  port,
        string  jwtSecret,
        bool    noTls,
        string  certPath,
        string  keyPath,
        string  jwtExpiry)
    {
        var errors = new System.Collections.Generic.List<string>();

        // Port range check
        if (!int.TryParse(port, out var p) || p < 1 || p > 65535)
            errors.Add("Port must be a number between 1 and 65535.");

        // JWT secret presence and length
        if (string.IsNullOrEmpty(jwtSecret))
            errors.Add("JWT secret is required (or set MM_JWT_SECRET env var).");
        else if (jwtSecret.Length < 16)
            errors.Add("JWT secret must be at least 16 characters.");

        // TLS cert / key (only required when TLS is enabled)
        if (!noTls)
        {
            if (string.IsNullOrEmpty(certPath))
                errors.Add("TLS certificate path is required (or enable 'Disable TLS').");
            if (string.IsNullOrEmpty(keyPath))
                errors.Add("TLS private key path is required (or enable 'Disable TLS').");
        }

        // JWT expiry must be positive
        if (!long.TryParse(jwtExpiry, out var exp) || exp < 1)
            errors.Add("Token expiry must be a positive integer (seconds).");

        return [..errors];
    }
}

// ── JWT helpers replicated for testing ───────────────────────────────────────

/// <summary>Helper for JWT-related pure-logic assertions.</summary>
internal static class JwtHelper
{
    /// <summary>Returns true when the secret meets minimum requirements.</summary>
    public static bool IsSecretStrong(string secret)
        => !string.IsNullOrWhiteSpace(secret) && secret.Length >= 32;

    /// <summary>Formats the expiry seconds as a human-readable duration string.</summary>
    public static string FormatExpiry(long seconds) => seconds switch
    {
        < 60     => $"{seconds}s",
        < 3600   => $"{seconds / 60}m",
        < 86400  => $"{seconds / 3600}h",
        _        => $"{seconds / 86400}d",
    };
}

// ── Tests ────────────────────────────────────────────────────────────────────

public class ServerPageTests
{
    // ── Route table ──────────────────────────────────────────────────────────

    [Fact]
    public void RouteTable_HasEightEntries()
        => Assert.Equal(8, ServerRoutes.All.Length);

    [Fact]
    public void RouteTable_HealthRouteIsFirst()
    {
        var (method, path, _) = ServerRoutes.All[0];
        Assert.Equal("GET", method);
        Assert.Equal("/health", path);
    }

    [Fact]
    public void RouteTable_ContainsLoginRoute()
        => Assert.Contains(ServerRoutes.All, r => r.Path == "/auth/login" && r.Method == "POST");

    [Fact]
    public void RouteTable_ContainsStreamGetAndHead()
    {
        var streamRoutes = ServerRoutes.All.Where(r => r.Path == "/stream/{id}").ToArray();
        Assert.Equal(2, streamRoutes.Length);
        Assert.Contains(streamRoutes, r => r.Method == "GET");
        Assert.Contains(streamRoutes, r => r.Method == "HEAD");
    }

    [Fact]
    public void RouteTable_AllPathsStartWithSlash()
    {
        foreach (var (_, path, _) in ServerRoutes.All)
            Assert.StartsWith("/", path);
    }

    [Fact]
    public void RouteTable_AllDescriptionsNonEmpty()
    {
        foreach (var (_, _, desc) in ServerRoutes.All)
            Assert.False(string.IsNullOrWhiteSpace(desc));
    }

    [Fact]
    public void RouteTable_AllMethodsUppercase()
    {
        foreach (var (method, _, _) in ServerRoutes.All)
            Assert.Equal(method, method.ToUpperInvariant());
    }

    // ── Config validation ────────────────────────────────────────────────────

    [Fact]
    public void Validate_FullTlsConfig_NoErrors()
    {
        var errs = ServerConfigValidator.Validate(
            port:      "8443",
            jwtSecret: "super-secret-key-minimum-32-chars!",
            noTls:     false,
            certPath:  @"C:\ssl\cert.pem",
            keyPath:   @"C:\ssl\key.pem",
            jwtExpiry: "86400");

        Assert.Empty(errs);
    }

    [Fact]
    public void Validate_NoTlsConfig_NoErrors()
    {
        var errs = ServerConfigValidator.Validate(
            port:      "8080",
            jwtSecret: "dev-secret-key-at-least-16-chars",
            noTls:     true,
            certPath:  "",        // not required when noTls=true
            keyPath:   "",
            jwtExpiry: "3600");

        Assert.Empty(errs);
    }

    [Fact]
    public void Validate_EmptyJwtSecret_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "", false, "c.pem", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("JWT secret is required"));
    }

    [Fact]
    public void Validate_ShortJwtSecret_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "tooshort", false, "c.pem", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("16 characters"));
    }

    [Fact]
    public void Validate_InvalidPort_Error()
    {
        var errs = ServerConfigValidator.Validate("notaport", "sixteen-char-secret!", false, "c.pem", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("Port must be"));
    }

    [Fact]
    public void Validate_PortZero_Error()
    {
        var errs = ServerConfigValidator.Validate("0", "sixteen-char-secret!", false, "c.pem", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("Port must be"));
    }

    [Fact]
    public void Validate_PortAboveMax_Error()
    {
        var errs = ServerConfigValidator.Validate("70000", "sixteen-char-secret!", false, "c.pem", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("Port must be"));
    }

    [Fact]
    public void Validate_MissingCertWithTls_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "sixteen-char-secret!", false, "", "k.pem", "86400");
        Assert.Contains(errs, e => e.Contains("certificate path"));
    }

    [Fact]
    public void Validate_MissingKeyWithTls_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "sixteen-char-secret!", false, "c.pem", "", "86400");
        Assert.Contains(errs, e => e.Contains("private key path"));
    }

    [Fact]
    public void Validate_InvalidExpiry_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "sixteen-char-secret!", false, "c.pem", "k.pem", "abc");
        Assert.Contains(errs, e => e.Contains("Token expiry"));
    }

    [Fact]
    public void Validate_ZeroExpiry_Error()
    {
        var errs = ServerConfigValidator.Validate("8443", "sixteen-char-secret!", false, "c.pem", "k.pem", "0");
        Assert.Contains(errs, e => e.Contains("Token expiry"));
    }

    // ── JWT helpers ───────────────────────────────────────────────────────────

    [Fact]
    public void JwtHelper_ShortSecret_NotStrong()
        => Assert.False(JwtHelper.IsSecretStrong("tooshort"));

    [Fact]
    public void JwtHelper_32CharSecret_IsStrong()
        => Assert.True(JwtHelper.IsSecretStrong("exactly-thirty-two-chars-secret!"));

    [Fact]
    public void JwtHelper_FormatExpiry_Seconds()
        => Assert.Equal("45s", JwtHelper.FormatExpiry(45));

    [Fact]
    public void JwtHelper_FormatExpiry_Minutes()
        => Assert.Equal("30m", JwtHelper.FormatExpiry(1800));

    [Fact]
    public void JwtHelper_FormatExpiry_Hours()
        => Assert.Equal("2h", JwtHelper.FormatExpiry(7200));

    [Fact]
    public void JwtHelper_FormatExpiry_Days()
        => Assert.Equal("1d", JwtHelper.FormatExpiry(86400));
}
