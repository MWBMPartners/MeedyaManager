// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings Page code-behind (WinUI 3)
//
// Populates the settings page from MmCore config defaults and the raw
// settings.json5 file (if it exists on disk).  Includes test mode
// support: a journalled safe-mode that allows users to preview and
// revert file operations before committing them permanently.

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
/// Also provides a Test Mode toggle that journals file operations,
/// allowing users to commit or revert changes safely.
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

        // Synchronise the test mode toggle with the core's current state
        TestModeToggle.IsOn = MmCore.Instance.TestModeEnabled();
        RefreshTestModeUI();

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

    // ── Test Mode ──────────────────────────────────────────────────────────

    /// <summary>
    /// Handles the Test Mode toggle switch state change.
    /// When disabling test mode, prompts the user to commit or keep changes.
    /// </summary>
    private async void TestModeToggle_Toggled(object sender, RoutedEventArgs e)
    {
        if (TestModeToggle.IsOn)
        {
            // Enabling test mode — activate journalling in the core
            MmCore.Instance.SetTestMode(true);
            RefreshTestModeUI();
        }
        else
        {
            // Disabling test mode — check if there are tracked files
            int fileCount = MmCore.Instance.TestModeFileCount();

            if (fileCount > 0)
            {
                // Show a confirmation dialog: commit or keep both versions?
                var dialog = new ContentDialog
                {
                    Title = "Test Mode — Uncommitted Changes",
                    Content = $"You have {fileCount} file(s) with uncommitted test mode changes.\n\n" +
                              "Choose \"Commit\" to make the changes permanent, or " +
                              "\"Keep Both\" to retain the original and modified copies.",
                    PrimaryButtonText = "Commit",
                    SecondaryButtonText = "Keep Both",
                    CloseButtonText = "Cancel",
                    DefaultButton = ContentDialogButton.Primary,
                    XamlRoot = this.XamlRoot,
                };

                ContentDialogResult result = await dialog.ShowAsync();

                switch (result)
                {
                    case ContentDialogResult.Primary:
                        // User chose to commit — make changes permanent
                        MmCore.Instance.CommitTestModeFiles();
                        MmCore.Instance.SetTestMode(false);
                        break;

                    case ContentDialogResult.Secondary:
                        // User chose to keep both — disable without reverting
                        MmCore.Instance.SetTestMode(false);
                        break;

                    default:
                        // User cancelled — re-enable the toggle (stay in test mode)
                        TestModeToggle.IsOn = true;
                        return;
                }
            }
            else
            {
                // No tracked files — simply disable test mode
                MmCore.Instance.SetTestMode(false);
            }

            RefreshTestModeUI();
        }
    }

    /// <summary>
    /// Commits all journalled test mode changes, making them permanent.
    /// </summary>
    private void TestModeCommitButton_Click(object sender, RoutedEventArgs e)
    {
        MmCore.Instance.CommitTestModeFiles();
        RefreshTestModeUI();
    }

    /// <summary>
    /// Reverts all journalled test mode changes, restoring original files.
    /// </summary>
    private void TestModeRevertButton_Click(object sender, RoutedEventArgs e)
    {
        MmCore.Instance.RevertTestModeFiles();
        RefreshTestModeUI();
    }

    /// <summary>
    /// Updates the test mode UI elements (file count label and action buttons)
    /// to reflect the current state of the test mode journal.
    /// </summary>
    private void RefreshTestModeUI()
    {
        bool isOn = MmCore.Instance.TestModeEnabled();
        int fileCount = MmCore.Instance.TestModeFileCount();

        // Show the file count label when test mode is active
        TestModeFileCountText.Text = $"Tracked files: {fileCount}";
        TestModeFileCountText.Visibility = isOn
            ? Visibility.Visible
            : Visibility.Collapsed;

        // Show commit/revert buttons only when there are tracked files
        TestModeActionPanel.Visibility = isOn && fileCount > 0
            ? Visibility.Visible
            : Visibility.Collapsed;
    }

    // ── Save Settings ────────────────────────────────────────────────────────

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
