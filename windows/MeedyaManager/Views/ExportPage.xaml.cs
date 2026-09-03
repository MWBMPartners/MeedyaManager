// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export Page code-behind (WinUI 3, M9)
//
// Implements the Export page: backend picker, DSN entry, and schema preview.
// Running an export is disabled in this alpha — mm-export does not yet open
// a real database connection. See issue #205.

using System;
using System.Text;
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
        AppendLog("Export page ready. This is a preview only — Export Library is disabled in this alpha (issue #205).");
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

    // Running an export is disabled in this alpha — mm-export does not yet
    // open a real database connection, so ExportBtn has no Click handler
    // wired up (see ExportPage.xaml). There is nothing genuine for it to do.
    // See the InfoBar above and issue #205.

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
