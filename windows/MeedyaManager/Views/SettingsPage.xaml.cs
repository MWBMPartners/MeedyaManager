// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
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
/// Settings page (M4 stub): displays current configuration values.
/// Full save-to-JSON5 support is planned for M6.
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
}
