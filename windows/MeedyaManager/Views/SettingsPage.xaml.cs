// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings Page code-behind (WinUI 3)
//
// Populates the settings page from MmCore config defaults and the raw
// settings.json5 file (if it exists on disk).

using System;
using System.IO;
using MeedyaManager.Interop;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.ApplicationModel.DataTransfer;
using Windows.System;

namespace MeedyaManager.Views;

/// <summary>
/// Settings page (M6): displays and edits configuration values.
/// Changes can be saved back to the config file on disk.
/// </summary>
public sealed partial class SettingsPage : Page
{
    // Config file path from MmCore (or stub default)
    private string _configPath = string.Empty;

    public SettingsPage()
    {
        this.InitializeComponent();
        LoadSettings();
    }

    // ── Load ────────────────────────────────────────────────────────────────

    /// <summary>Populates controls from the MmCore config and the JSON5 file on disk.</summary>
    private void LoadSettings()
    {
        // Populate config file path
        _configPath = MmCore.Instance.ConfigPath();
        ConfigPathBox.Text = _configPath;

        // Version footer
        VersionText.Text = $"MeedyaManager core {MmCore.Instance.Version()}";

        // Default log level selection
        LogLevelBox.SelectedIndex = 2; // "info"

        // Read raw config text from disk (best-effort; silently ignore errors)
        try
        {
            if (File.Exists(_configPath))
            {
                RawConfigBox.Text = File.ReadAllText(_configPath);
            }
            else
            {
                RawConfigBox.Text = "// Config file not found — defaults are in use.\n" +
                                    "// MeedyaManager will create settings.json5 on first launch.";
            }
        }
        catch (Exception ex)
        {
            RawConfigBox.Text = $"// Could not read config: {ex.Message}";
        }
    }

    // ── Open Folder ─────────────────────────────────────────────────────────

    /// <summary>Opens the config file's parent folder in Windows Explorer.</summary>
    private async void OpenFolderButton_Click(object sender, RoutedEventArgs e)
    {
        string? dir = Path.GetDirectoryName(_configPath);
        if (dir is null) return;

        // Ensure the directory exists before attempting to open it
        Directory.CreateDirectory(dir);

        await Launcher.LaunchFolderPathAsync(dir);
    }

    // ── Copy Path ───────────────────────────────────────────────────────────

    /// <summary>Copies the config file path to the clipboard.</summary>
    private void CopyPathButton_Click(object sender, RoutedEventArgs e)
    {
        var data = new DataPackage();
        data.SetText(_configPath);
        Clipboard.SetContent(data);
    }

    // ── Save Settings ────────────────────────────────────────────────────────

    /// <summary>
    /// Serialises the current control values to JSON and writes to <see cref="_configPath"/>.
    /// Refreshes the raw config preview after a successful save.
    /// </summary>
    private void SaveButton_Click(object sender, RoutedEventArgs e)
    {
        SaveSettings();
    }

    /// <summary>Builds a JSON settings snapshot and writes it to disk.</summary>
    private void SaveSettings()
    {
        try
        {
            // Build a dictionary of current settings from the control values
            var snapshot = new System.Collections.Generic.Dictionary<string, object>
            {
                ["dry_run"]     = DryRunToggle.IsOn,
                ["recursive"]   = RecursiveToggle.IsOn,
                ["debounce_ms"] = (int)DebounceBox.Value,
                ["log_level"]   = LogLevelBox.SelectedItem?.ToString() ?? "info",
                ["redact_pii"]  = RedactPiiToggle.IsOn,
            };

            // Serialize to pretty-printed JSON (valid JSON5 superset)
            string json = System.Text.Json.JsonSerializer.Serialize(
                snapshot,
                new System.Text.Json.JsonSerializerOptions { WriteIndented = true }
            );

            // Ensure the parent directory exists before writing
            string? dir = Path.GetDirectoryName(_configPath);
            if (dir is not null)
                Directory.CreateDirectory(dir);

            // Write atomically — write to a temp file then rename
            string tempPath = _configPath + ".tmp";
            File.WriteAllText(tempPath, json, System.Text.Encoding.UTF8);
            File.Move(tempPath, _configPath, overwrite: true);

            // Refresh the raw config preview with the new content
            RawConfigBox.Text = json;
            SaveStatusText.Text = "✓ Settings saved.";
        }
        catch (Exception ex)
        {
            SaveStatusText.Text = $"Save failed: {ex.Message}";
        }
    }
}
