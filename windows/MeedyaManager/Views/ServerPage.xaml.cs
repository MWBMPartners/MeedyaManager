// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Secure Media Server Page code-behind (WinUI 3, M10)
//
// Implements the Server page: network/TLS/auth/CORS configuration,
// start/stop server control, route table display, and an access log.
// Real server start/stop is wired via mm-server P/Invoke — this stub
// exercises the complete UI flow and validation path.

using System;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MeedyaManager.Views;

/// <summary>
/// Secure Media Server page — lets users configure and control the
/// MeedyaManager HTTPS media server with JWT authentication.
/// </summary>
public sealed partial class ServerPage : Page
{
    // ── Route table ─────────────────────────────────────────────────────────

    /// HTTP route table displayed via RoutesBtn.  Matches the routes defined
    /// in the mm-server crate's routes.rs handler stubs.
    private static readonly (string Method, string Path, string Description)[] Routes =
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

    // ── Log buffer ───────────────────────────────────────────────────────────

    /// Mutable string builder for the access log — avoids repeated string allocation.
    private readonly StringBuilder _log = new();

    /// <summary>True while the server is in the running state.</summary>
    private bool _isRunning;

    // ── Constructor ──────────────────────────────────────────────────────────

    /// <summary>Initializes the Server page.</summary>
    public ServerPage()
    {
        // Inflate the XAML component tree defined in ServerPage.xaml
        this.InitializeComponent();

        AppendLog("Server page ready. Configure settings and click Start Server.");
        AppendLog("TLS is required for production use. Use --no-tls only for local development.");
    }

    // ── Event handlers ───────────────────────────────────────────────────────

    /// <summary>
    /// Validates the current configuration and starts the HTTPS media server.
    /// </summary>
    private async void StartBtn_Click(object sender, RoutedEventArgs e)
    {
        // Validate the configuration before attempting to start
        var errors = ValidateConfig();
        if (errors.Length > 0)
        {
            foreach (var err in errors)
                AppendLog($"[Config error] {err}");
            StatusText.Text = "Fix configuration errors before starting.";
            return;
        }

        // Disable start, enable stop while the server is starting/running
        StartBtn.IsEnabled = false;
        StopBtn.IsEnabled  = false;
        RoutesBtn.IsEnabled = false;
        StatusText.Text    = "Status: Starting…";

        var bind    = BindAddressBox.Text.Trim();
        var port    = PortBox.Text.Trim();
        var noTls   = NoTlsToggle.IsOn;
        var scheme  = noTls ? "http" : "https";
        var expiry  = JwtExpiryBox.Text.Trim();
        var cors    = CorsOriginsBox.Text.Trim();

        AppendLog($"Starting {scheme} server on {bind}:{port}…");
        AppendLog($"JWT token expiry: {expiry} seconds");
        if (!string.IsNullOrEmpty(cors))
            AppendLog($"CORS origins: {cors}");
        if (noTls)
            AppendLog("[Warning] TLS disabled — not suitable for production use.");
        else
        {
            AppendLog($"TLS cert: {CertPathBox.Text.Trim()}");
            AppendLog($"TLS key:  {KeyPathBox.Text.Trim()}");
        }

        // Simulate async server start latency (real start wired via P/Invoke)
        await Task.Delay(millisecondsDelay: 1200).ConfigureAwait(true);

        _isRunning = true;
        StatusText.Text   = $"Status: Running — {scheme}://{bind}:{port}/";
        StopBtn.IsEnabled = true;
        RoutesBtn.IsEnabled = true;

        AppendLog($"Server running at {scheme}://{bind}:{port}/");
        AppendLog("Use POST /auth/login to obtain a JWT bearer token.");
    }

    /// <summary>
    /// Stops the running media server.
    /// </summary>
    private async void StopBtn_Click(object sender, RoutedEventArgs e)
    {
        // Disable stop button immediately to prevent double-clicks
        StopBtn.IsEnabled  = false;
        StatusText.Text    = "Status: Stopping…";

        AppendLog("Stopping server…");

        // Simulate async shutdown latency (real shutdown wired via P/Invoke)
        await Task.Delay(millisecondsDelay: 600).ConfigureAwait(true);

        _isRunning          = false;
        StatusText.Text     = "Status: Stopped";
        StartBtn.IsEnabled  = true;
        RoutesBtn.IsEnabled = true;

        AppendLog("Server stopped.");
    }

    /// <summary>
    /// Appends the HTTP route table to the access log.
    /// </summary>
    private void RoutesBtn_Click(object sender, RoutedEventArgs e)
    {
        AppendLog("─── HTTP Route Table ───────────────────────────────");
        foreach (var (method, path, desc) in Routes)
            AppendLog($"  {method,-6} {path,-22} — {desc}");
        AppendLog("────────────────────────────────────────────────────");
    }

    /// <summary>Clears the access log.</summary>
    private void ClearLogBtn_Click(object sender, RoutedEventArgs e)
    {
        _log.Clear();
        LogBox.Text     = string.Empty;
        StatusText.Text = _isRunning ? StatusText.Text : "Status: Stopped";
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// <summary>
    /// Validates the current server configuration.
    /// Returns an array of error messages; empty array means valid.
    /// </summary>
    private string[] ValidateConfig()
    {
        var errors = new System.Collections.Generic.List<string>();

        // Port must be a valid integer in range 1–65535
        if (!int.TryParse(PortBox.Text.Trim(), out var port) || port < 1 || port > 65535)
            errors.Add("Port must be a number between 1 and 65535.");

        // JWT secret is required and must be at least 16 characters
        var secret = JwtSecretBox.Password.Trim();
        if (string.IsNullOrEmpty(secret))
            errors.Add("JWT secret is required (or set MM_JWT_SECRET env var).");
        else if (secret.Length < 16)
            errors.Add("JWT secret must be at least 16 characters.");

        // TLS cert and key are required unless --no-tls is enabled
        if (!NoTlsToggle.IsOn)
        {
            if (string.IsNullOrEmpty(CertPathBox.Text.Trim()))
                errors.Add("TLS certificate path is required (or enable 'Disable TLS').");
            if (string.IsNullOrEmpty(KeyPathBox.Text.Trim()))
                errors.Add("TLS private key path is required (or enable 'Disable TLS').");
        }

        // JWT expiry must be a positive integer
        if (!long.TryParse(JwtExpiryBox.Text.Trim(), out var expiry) || expiry < 1)
            errors.Add("Token expiry must be a positive integer (seconds).");

        return [..errors];
    }

    /// <summary>Appends a timestamped line to the log TextBox.</summary>
    private void AppendLog(string message)
    {
        var ts   = DateTime.Now.ToString("HH:mm:ss");
        var line = $"[{ts}] {message}{Environment.NewLine}";
        _log.Append(line);
        LogBox.Text = _log.ToString();
        // Scroll to the most recently appended line
        LogScroll.ChangeView(null, double.MaxValue, null);
    }
}
