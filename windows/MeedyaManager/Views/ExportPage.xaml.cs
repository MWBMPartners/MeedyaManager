// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export Page code-behind (WinUI 3, M9)
//
// Implements the Export page: backend picker, DSN entry, schema preview,
// and simulated export (real DB writes wired via mm-export P/Invoke in M9+).

using System;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MeedyaManager.Views;

/// <summary>
/// Database Export page — lets users configure and run a media library export.
/// </summary>
public sealed partial class ExportPage : Page
{
    // ── DSN hint strings per backend ────────────────────────────────────────

    /// Maps backend tag → example DSN shown as hint text.
    private static readonly (string Tag, string Hint)[] BackendHints =
    [
        ("sqlite",   "sqlite:///C:/Users/You/library.db"),
        ("mysql",    "mysql://user:pass@localhost/meedya"),
        ("mariadb",  "mysql://user:pass@localhost/meedya"),
        ("postgres", "postgres://user:pass@localhost/meedya"),
        ("mssql",    "server=tcp:host,1433;database=meedya;user=sa;password=P"),
    ];

    // ── Log buffer ───────────────────────────────────────────────────────────

    /// Mutable string builder for the export log.
    private readonly StringBuilder _log = new();

    // ── Constructor ──────────────────────────────────────────────────────────

    /// <summary>Initializes the Export page.</summary>
    public ExportPage()
    {
        this.InitializeComponent();
        AppendLog("Export page ready. Configure a connection string and click Export Library.");
    }

    // ── Event handlers ───────────────────────────────────────────────────────

    /// <summary>
    /// Updates the DSN hint text when the user changes the backend selection.
    /// </summary>
    private void BackendCombo_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (BackendCombo.SelectedItem is not ComboBoxItem item) return;
        var tag = item.Tag?.ToString() ?? "sqlite";

        foreach (var (hintTag, hint) in BackendHints)
        {
            if (hintTag == tag)
            {
                DsnHintText.Text    = $"Example: {hint}";
                DsnBox.PlaceholderText = hint;
                break;
            }
        }
    }

    /// <summary>
    /// Shows the schema DDL that would be created for the selected backend.
    /// </summary>
    private void SchemaBtn_Click(object sender, RoutedEventArgs e)
    {
        var backend = GetSelectedBackend();
        var prefix  = PrefixBox.Text.Trim();
        if (string.IsNullOrEmpty(prefix)) prefix = "mm_";

        AppendLog($"--- Schema DDL preview ({backend}) ---");
        AppendLog($"CREATE TABLE IF NOT EXISTS {prefix}files   ( … );");
        AppendLog($"CREATE TABLE IF NOT EXISTS {prefix}tags    ( … );");
        AppendLog($"CREATE TABLE IF NOT EXISTS {prefix}history ( … );");
        AppendLog("Full DDL available via: meedya export --show-schema --db <DSN>");

        StatusText.Text = "Schema DDL appended to log.";
    }

    /// <summary>
    /// Initiates the export operation.  For M9 the database write is simulated.
    /// </summary>
    private async void ExportBtn_Click(object sender, RoutedEventArgs e)
    {
        var dsn = DsnBox.Text.Trim();
        if (string.IsNullOrEmpty(dsn))
        {
            StatusText.Text = "⚠ Please enter a connection string before exporting.";
            return;
        }

        ExportBtn.IsEnabled = false;
        SchemaBtn.IsEnabled = false;

        var backend = GetSelectedBackend();
        var prefix  = PrefixBox.Text.Trim();
        if (string.IsNullOrEmpty(prefix)) prefix = "mm_";
        var dryRun  = DryRunToggle.IsOn;

        AppendLog($"Starting export to {backend}…");
        AppendLog($"DSN length: {dsn.Length} chars");
        AppendLog($"Table prefix: {prefix}");
        AppendLog($"Dry run: {dryRun}");

        StatusText.Text = "Exporting…";

        // Simulate async export latency (real export wired via P/Invoke in M9+)
        await Task.Delay(millisecondsDelay: 1200).ConfigureAwait(true);

        AppendLog("Export complete (stub — no DB writes in M9).");
        StatusText.Text = dryRun
            ? "Dry-run complete. No rows written."
            : "Export finished: 0 inserted, 0 updated, 0 skipped.";

        ExportBtn.IsEnabled = true;
        SchemaBtn.IsEnabled = true;
    }

    /// <summary>Clears the export log.</summary>
    private void ClearLogBtn_Click(object sender, RoutedEventArgs e)
    {
        _log.Clear();
        LogBox.Text     = string.Empty;
        StatusText.Text = string.Empty;
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Returns the tag string of the currently selected backend.
    private string GetSelectedBackend()
    {
        if (BackendCombo.SelectedItem is ComboBoxItem item)
            return item.Tag?.ToString() ?? "sqlite";
        return "sqlite";
    }

    /// Appends a timestamped line to the log TextBox.
    private void AppendLog(string message)
    {
        var ts   = DateTime.Now.ToString("HH:mm:ss");
        var line = $"[{ts}] {message}{Environment.NewLine}";
        _log.Append(line);
        LogBox.Text = _log.ToString();
        // Scroll to end
        LogScroll.ChangeView(null, double.MaxValue, null);
    }
}
