// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Cloud Storage Monitor Page code-behind (WinUI 3, M7)
//
// Builds provider rows at runtime, handles connect/disconnect toggles, and
// appends timestamped entries to the on-screen event log.

using System;
using System.Collections.Generic;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace MeedyaManager.Views;

// ---------------------------------------------------------------------------
// ProviderRow view-model (pure C# — no WinUI dependency)
// ---------------------------------------------------------------------------

/// View-model for a single cloud provider row in the Cloud page list.
internal sealed class CloudProviderRow
{
    /// Internal identifier matching the mm-cloud provider name.
    public string Id         { get; init; } = string.Empty;
    /// Display name shown to the user.
    public string Label      { get; init; } = string.Empty;
    /// Whether the user is authenticated with this provider.
    public bool   IsConnected { get; set; }
    /// Short status string ("Not Connected", "Synced", "Coming Soon", etc.)
    public string Status      { get; set; } = "Not Connected";
    /// `true` for providers not yet implemented (MEGA, iCloud).
    public bool   IsStub      { get; init; }
}

// ---------------------------------------------------------------------------
// CloudPage
// ---------------------------------------------------------------------------

/// Cloud Storage Monitor page.
///
/// Displays five cloud provider rows with Connect/Disconnect buttons and a
/// scrollable event log.  Provider state changes are simulated for M7;
/// live OAuth and mm-cloud FFI calls will be wired up in M8.
public sealed partial class CloudPage : Page
{
    // The ordered list of cloud provider descriptors.
    private readonly List<CloudProviderRow> _providers =
    [
        new() { Id = "onedrive",    Label = "OneDrive",     IsStub = false, Status = "Not Connected" },
        new() { Id = "googledrive", Label = "Google Drive", IsStub = false, Status = "Not Connected" },
        new() { Id = "dropbox",     Label = "Dropbox",      IsStub = false, Status = "Not Connected" },
        new() { Id = "mega",        Label = "MEGA",         IsStub = true,  Status = "Coming Soon"   },
        new() { Id = "icloud",      Label = "iCloud Drive", IsStub = true,  Status = "macOS only"    },
    ];

    public CloudPage()
    {
        this.InitializeComponent();
        // Build the provider rows after the XAML tree is ready.
        this.Loaded += (_, _) => BuildProviderRows();
    }

    // ── Row builder ──────────────────────────────────────────────────────────

    /// Populates the ProviderList with one row per cloud provider.
    private void BuildProviderRows()
    {
        foreach (var provider in _providers)
        {
            // Root grid for one row: [Name | Status | Button]
            var grid = new Grid
            {
                ColumnSpacing = 12,
                Padding       = new Thickness(0, 6, 0, 6),
            };
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(140) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

            // Provider name
            var nameLabel = new TextBlock
            {
                Text              = provider.Label,
                VerticalAlignment = VerticalAlignment.Center,
                FontWeight        = Microsoft.UI.Text.FontWeights.SemiBold,
            };
            Grid.SetColumn(nameLabel, 0);
            grid.Children.Add(nameLabel);

            // Status label
            var statusLabel = new TextBlock
            {
                Text              = provider.Status,
                VerticalAlignment = VerticalAlignment.Center,
                Foreground        = Application.Current.Resources["TextFillColorSecondaryBrush"] as Brush,
            };
            Grid.SetColumn(statusLabel, 1);
            grid.Children.Add(statusLabel);

            // Connect / Disconnect button (or Coming Soon label for stubs)
            if (provider.IsStub)
            {
                var stubLabel = new TextBlock
                {
                    Text              = "—",
                    VerticalAlignment = VerticalAlignment.Center,
                    HorizontalAlignment = HorizontalAlignment.Right,
                    Foreground        = Application.Current.Resources["TextFillColorDisabledBrush"] as Brush,
                };
                Grid.SetColumn(stubLabel, 2);
                grid.Children.Add(stubLabel);
            }
            else
            {
                var btn = new Button
                {
                    Content           = "Connect",
                    VerticalAlignment = VerticalAlignment.Center,
                    Tag               = provider.Id,
                };
                btn.Click += (_, _) => ToggleProvider(provider, btn, statusLabel);
                Grid.SetColumn(btn, 2);
                grid.Children.Add(btn);
            }

            ProviderList.Items.Add(new ListViewItem { Content = grid, IsTabStop = false });
        }
    }

    // ── Connect / disconnect toggle ──────────────────────────────────────────

    /// Toggles the connection state of the given provider and updates the UI.
    private void ToggleProvider(CloudProviderRow provider, Button btn, TextBlock statusLabel)
    {
        provider.IsConnected = !provider.IsConnected;

        if (provider.IsConnected)
        {
            provider.Status      = "Syncing\u2026";
            statusLabel.Text     = provider.Status;
            btn.Content          = "Disconnect";
            AppendLog($"[{provider.Label}] Connecting\u2026");
            // Simulate a successful sync (production calls mm-cloud FFI).
            _ = SimulatedSyncAsync(provider, statusLabel);
        }
        else
        {
            provider.Status      = "Not Connected";
            statusLabel.Text     = provider.Status;
            btn.Content          = "Connect";
            AppendLog($"[{provider.Label}] Disconnected");
        }
    }

    /// Simulates a brief sync delay then marks the provider as Synced.
    private async System.Threading.Tasks.Task SimulatedSyncAsync(
        CloudProviderRow provider, TextBlock statusLabel)
    {
        await System.Threading.Tasks.Task.Delay(1200);
        if (!provider.IsConnected) return; // user disconnected during delay
        provider.Status   = "Synced";
        statusLabel.Text  = provider.Status;
        AppendLog($"[{provider.Label}] Connected \u2014 watching /Music");
    }

    // ── Event log ────────────────────────────────────────────────────────────

    /// Appends a timestamped entry to the event log TextBox.
    private void AppendLog(string message)
    {
        string ts   = DateTime.Now.ToString("HH:mm:ss");
        string line = $"[{ts}] {message}\r\n";
        EventLogBox.Text += line;
        // Auto-scroll to the newest entry.
        EventLogBox.SelectionStart  = EventLogBox.Text.Length;
        EventLogBox.SelectionLength = 0;
    }

    /// Clears the event log when the Clear button is clicked.
    private void ClearLogButton_Click(object sender, RoutedEventArgs e)
    {
        EventLogBox.Text = string.Empty;
    }
}
