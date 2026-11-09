// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Cloud Storage Monitor Page code-behind (WinUI 3, M7)
//
// Builds provider rows at runtime. Connect/Disconnect is disabled in this
// alpha — mm-cloud does not yet make a real network call for any provider.
// See issue #205.

using System;
using System.Collections.Generic;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
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
    /// Short status string ("Not Connected", "Coming Soon", etc.)
    public string Status      { get; set; } = "Not Connected";
    /// `true` for providers not yet implemented (MEGA, iCloud).
    public bool   IsStub      { get; init; }
}

// ---------------------------------------------------------------------------
// CloudPage
// ---------------------------------------------------------------------------

/// Cloud Storage Monitor page.
///
/// Displays five cloud provider rows with a Connect/Disconnect button and a
/// scrollable event log. Connect/Disconnect is disabled in this alpha — the
/// real OAuth and mm-cloud FFI calls have not been wired up yet. See #205.
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
        AppendLog("Cloud page ready. This is a preview only — Connect is disabled in this alpha (issue #205).");
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

            // Connect / Disconnect button (or Coming Soon label for stubs) —
            // disabled in this alpha, since mm-cloud does not yet make a real
            // network call for any provider. See the InfoBar above and
            // issue #205.
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
                    IsEnabled         = false,
                };
                AutomationProperties.SetName(btn, $"Connect {provider.Label} (disabled — not functional in this alpha)");
                AutomationProperties.SetHelpText(btn, $"Connecting {provider.Label} is disabled in this alpha; mm-cloud does not yet make a real network call. See issue #205.");
                Grid.SetColumn(btn, 2);
                grid.Children.Add(btn);
            }

            ProviderList.Items.Add(new ListViewItem { Content = grid, IsTabStop = false });
        }
    }

    // Connecting and disconnecting a provider are disabled in this alpha —
    // mm-cloud does not yet make a real network call for any provider, so
    // there is no toggle handler wired up above. See issue #205.

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
