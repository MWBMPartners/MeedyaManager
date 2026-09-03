// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Secure Media Server Page code-behind (WinUI 3, M10)
//
// Implements the Server page: network/TLS/auth/CORS configuration, route
// table display, and an access log. Start/Stop are disabled in this alpha —
// mm-server does not yet build a real HTTP router. See issue #205.

using System;
using System.Text;
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

    // ── Constructor ──────────────────────────────────────────────────────────

    /// <summary>Initializes the Server page.</summary>
    public ServerPage()
    {
        // Inflate the XAML component tree defined in ServerPage.xaml
        this.InitializeComponent();

        AppendLog("Server page ready. This is a preview only — Start/Stop are disabled in this alpha (issue #205).");
        AppendLog("TLS is required for production use. Use --no-tls only for local development.");
    }

    // ── Event handlers ───────────────────────────────────────────────────────
    //
    // Starting and stopping the server are disabled in this alpha — mm-server
    // does not yet build a real HTTP router, so StartBtn/StopBtn have no Click
    // handler wired up (see ServerPage.xaml). There is nothing genuine for
    // either button to do. See the InfoBar above and issue #205.

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
        StatusText.Text = "Status: Stopped";
    }

    // ── Private helpers ──────────────────────────────────────────────────────

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
