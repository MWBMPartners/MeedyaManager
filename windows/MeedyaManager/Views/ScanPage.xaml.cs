// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Scan Page code-behind (WinUI 3)
//
// Handles folder picking, template validation, scanning via MmCore,
// and executing renames.  All I/O runs on a background thread to keep
// the UI responsive.

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Threading.Tasks;
using MeedyaManager.Interop;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace MeedyaManager.Views;

// ---------------------------------------------------------------------------
// View-model row for the results list
// ---------------------------------------------------------------------------

/// <summary>Represents one rename preview row displayed in the results list.</summary>
internal sealed class PreviewRow
{
    public string SourceName  { get; }
    public string Arrow       { get; }
    public string BadgeText   { get; }
    public Brush  BadgeColor  { get; }

    // Whether the rename can be executed (not a conflict, not unchanged)
    public bool IsExecutable { get; }

    // Raw data needed to execute the rename
    public string SourcePath      { get; }
    public string DestinationPath { get; }

    public PreviewRow(RenamePreview preview)
    {
        SourcePath      = preview.Source;
        DestinationPath = preview.Destination;
        SourceName      = Path.GetFileName(preview.Source);

        string destName = Path.GetFileName(preview.Destination);
        Arrow = $"→  {destName}";

        if (preview.Conflict)
        {
            BadgeText   = "Conflict";
            BadgeColor  = new SolidColorBrush(Colors.OrangeRed);
            IsExecutable = false;
        }
        else if (preview.Unchanged)
        {
            BadgeText   = "Unchanged";
            BadgeColor  = new SolidColorBrush(Colors.Gray);
            IsExecutable = false;
        }
        else
        {
            BadgeText   = "Rename";
            BadgeColor  = new SolidColorBrush(Colors.SteelBlue);
            IsExecutable = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Page code-behind
// ---------------------------------------------------------------------------

/// <summary>
/// Scan page: lets the user pick a folder, configure a rename template,
/// preview the results, and execute the renames.
/// </summary>
public sealed partial class ScanPage : Page
{
    /// <summary>Preview rows bound to the results ListView.</summary>
    public ObservableCollection<PreviewRow> Previews { get; } = [];

    public ScanPage()
    {
        this.InitializeComponent();
        // Initialise template field and run validation once
        TemplateBox.Text = "<Artist> - <Title>";
    }

    // ── Browse ──────────────────────────────────────────────────────────────

    /// <summary>Opens a folder picker and populates FolderBox with the result.</summary>
    private async void BrowseButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker();
        picker.SuggestedStartLocation = Windows.Storage.Pickers.PickerLocationId.MusicLibrary;
        picker.FileTypeFilter.Add("*");

        // Initialise the picker with the window handle (required on Windows 11)
        var hwnd = WindowNative.GetWindowHandle(App.MainWindow);
        InitializeWithWindow.Initialize(picker, hwnd);

        var folder = await picker.PickSingleFolderAsync();
        if (folder is not null)
        {
            FolderBox.Text = folder.Path;
        }
    }

    // ── Template validation ─────────────────────────────────────────────────

    /// <summary>Validates the template on every keystroke and updates the InfoBar.</summary>
    private void TemplateBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        string template = TemplateBox.Text.Trim();
        if (string.IsNullOrEmpty(template))
        {
            ValidationBar.IsOpen = false;
            return;
        }

        var (isValid, message) = MmCore.Instance.ValidateTemplate(template);
        if (isValid)
        {
            ValidationBar.Severity = InfoBarSeverity.Success;
            ValidationBar.Message  = "Valid template";
            ValidationBar.IsOpen   = true;
        }
        else
        {
            ValidationBar.Severity = InfoBarSeverity.Error;
            ValidationBar.Message  = message;
            ValidationBar.IsOpen   = true;
        }
    }

    // ── Scan ────────────────────────────────────────────────────────────────

    /// <summary>Runs a scan of the selected folder and populates the results list.</summary>
    private async void ScanButton_Click(object sender, RoutedEventArgs e)
    {
        string directory = FolderBox.Text.Trim();
        string template  = TemplateBox.Text.Trim();

        if (string.IsNullOrEmpty(directory))
        {
            StatusText.Text = "Please select a folder first.";
            return;
        }

        var (isValid, _) = MmCore.Instance.ValidateTemplate(template);
        if (!isValid)
        {
            StatusText.Text = "Please enter a valid template first.";
            return;
        }

        // Show progress UI
        SetScanning(true);
        Previews.Clear();
        EmptyState.Visibility  = Visibility.Collapsed;
        ResultsList.Visibility = Visibility.Collapsed;
        StatusText.Text        = "Scanning…";

        bool recursive = RecursiveToggle.IsOn;

        // Run scan on background thread to avoid blocking the UI thread
        IReadOnlyList<RenamePreview> results = await Task.Run(() =>
            MmCore.Instance.ScanDirectory(directory, template, recursive));

        // Populate the observable collection on the UI thread
        int renamed = 0, conflicts = 0, unchanged = 0;
        foreach (RenamePreview preview in results)
        {
            var row = new PreviewRow(preview);
            Previews.Add(row);
            if (row.IsExecutable)  renamed++;
            else if (preview.Conflict)  conflicts++;
            else                        unchanged++;
        }

        // Update status
        if (Previews.Count == 0)
        {
            StatusText.Text       = "No media files found.";
            EmptyState.Visibility = Visibility.Visible;
        }
        else
        {
            StatusText.Text        = $"{Previews.Count} files — {renamed} to rename, {conflicts} conflicts, {unchanged} unchanged.";
            ResultsList.Visibility = Visibility.Visible;
        }

        ExecuteButton.IsEnabled = renamed > 0;
        SetScanning(false);
    }

    // ── Execute ─────────────────────────────────────────────────────────────

    /// <summary>Executes all non-conflicting, changed renames.</summary>
    private async void ExecuteButton_Click(object sender, RoutedEventArgs e)
    {
        SetScanning(true);
        ExecuteButton.IsEnabled = false;
        StatusText.Text = "Renaming…";

        int success = 0, failed = 0;

        await Task.Run(() =>
        {
            foreach (PreviewRow row in Previews)
            {
                if (!row.IsExecutable) continue;
                try
                {
                    // Ensure the destination directory exists before renaming
                    string? destDir = Path.GetDirectoryName(row.DestinationPath);
                    if (destDir is not null) Directory.CreateDirectory(destDir);

                    File.Move(row.SourcePath, row.DestinationPath, overwrite: false);
                    success++;
                }
                catch
                {
                    failed++;
                }
            }
        });

        StatusText.Text = $"Done — {success} renamed, {failed} failed.";
        SetScanning(false);
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    /// <summary>Toggles the progress ring and disables controls during long operations.</summary>
    private void SetScanning(bool active)
    {
        ScanProgress.IsActive   = active;
        ScanButton.IsEnabled    = !active;
        BrowseButton.IsEnabled  = !active;
        RecursiveToggle.IsEnabled = !active;
    }
}
