// (C) 2025-2026 MWBM Partners Ltd
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
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage;
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

    // ── Drag-and-drop ────────────────────────────────────────────────────────

    /// <summary>Accepts folder drops on the FolderBox text field.</summary>
    private void FolderBox_DragOver(object sender, DragEventArgs e)
    {
        // Only accept items that contain storage items (files/folders)
        if (e.DataView.Contains(StandardDataFormats.StorageItems))
        {
            e.AcceptedOperation = DataPackageOperation.Copy;
            e.DragUIOverride.Caption = "Set as source folder";
        }
        else
        {
            e.AcceptedOperation = DataPackageOperation.None;
        }
    }

    /// <summary>
    /// Handles a drop onto FolderBox.
    /// If a folder is dropped, its path is used directly.
    /// If a file is dropped, its parent directory is used.
    /// </summary>
    private async void FolderBox_Drop(object sender, DragEventArgs e)
    {
        if (!e.DataView.Contains(StandardDataFormats.StorageItems)) return;

        var items = await e.DataView.GetStorageItemsAsync();
        if (items.Count == 0) return;

        IStorageItem first = items[0];
        string path = first is StorageFolder folder
            ? folder.Path
            : Path.GetDirectoryName(first.Path) ?? first.Path;

        FolderBox.Text = path;
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

    /// <summary>
    /// Says why Execute must refuse, or <see langword="null"/> when it is safe
    /// to rename files for real. Checked both when Execute is enabled/disabled
    /// after a scan and again at the top of <see cref="ExecuteButton_Click"/>,
    /// so the button being (mis-)enabled can never be the only thing standing
    /// between a click and a real file move (issue #222).
    /// </summary>
    /// <param name="engineAvailable">Whether mm_ffi.dll was found (<see cref="MmCore.IsEngineAvailable"/>).</param>
    /// <param name="testModeEnabled">Whether Test Mode is currently on.</param>
    /// <returns>A user-facing refusal message, or null when Execute may proceed.</returns>
    internal static string? ExecuteRefusalReason(bool engineAvailable, bool testModeEnabled)
    {
        if (!engineAvailable)
        {
            return "Nothing was renamed. This build does not include the MeedyaManager engine, " +
                   "so there are no trustworthy previews to apply. (issue #222)";
        }
        if (testModeEnabled)
        {
            return "Nothing was renamed. Test Mode is on, and renames are not staged by Test Mode " +
                   "— turn it off in Settings to rename files for real.";
        }
        return null;
    }

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

        // Without the engine, ScanDirectory now returns an empty list rather
        // than fabricating previews (see MmCore, issue #222) — but that reads
        // to the user exactly like "no media files found" in an empty real
        // folder. Say the true reason plainly instead, and never let the
        // Execute button reach an enabled state on a fabricated scan.
        if (!MmCore.IsEngineAvailable)
        {
            StatusText.Text = "This build does not include the MeedyaManager engine, " +
                               "so scanning is unavailable. (issue #222)";
            EmptyState.Visibility  = Visibility.Visible;
            ResultsList.Visibility = Visibility.Collapsed;
            ExecuteButton.IsEnabled = false;
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

        // Even with real previews in hand, Execute must still stay off when
        // there is a standing reason to refuse (engine missing, Test Mode
        // on) — this is the same check ExecuteButton_Click makes at its own
        // top, kept here too so the button's own enabled state never lies
        // about whether a click would do anything (issue #222).
        ExecuteButton.IsEnabled =
            renamed > 0 &&
            ExecuteRefusalReason(MmCore.IsEngineAvailable, MmCore.Instance.TestModeEnabled()) is null;
        SetScanning(false);
    }

    // ── Execute ─────────────────────────────────────────────────────────────

    /// <summary>Executes all non-conflicting, changed renames.</summary>
    private async void ExecuteButton_Click(object sender, RoutedEventArgs e)
    {
        // Refuse regardless of whether the button itself is (mis-)enabled —
        // this is the last line of defence against the data-loss bug in
        // issue #222, so it must not depend on any other code path having
        // already got IsEnabled right.
        string? refusal = ExecuteRefusalReason(MmCore.IsEngineAvailable, MmCore.Instance.TestModeEnabled());
        if (refusal is not null)
        {
            StatusText.Text = refusal;
            ExecuteButton.IsEnabled = false;
            return;
        }

        SetScanning(true);
        ExecuteButton.IsEnabled = false;
        StatusText.Text = "Renaming…";

        int success = 0, failed = 0;

        // File.Move stays here, guarded by the refusal check above, rather
        // than being deleted the way the macOS fix for the same bug (#222)
        // deleted its file-moving loop outright: the mm-ffi C header exports
        // no execute/rename function at all, only mm_ffi_scan_directory for
        // previews, so there is no Rust-side execute path to hand this off
        // to on Windows. Reaching this point already means the engine is
        // present and Test Mode is off, so the previews above came from a
        // real scan.
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
