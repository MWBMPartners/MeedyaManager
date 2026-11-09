// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Page code-behind (WinUI 3)
//
// Opens a media file, displays its tags in an editable list, and saves
// changes back through MmCore.  All I/O runs on a background thread.

using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using MeedyaManager.Interop;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace MeedyaManager.Views;

// ---------------------------------------------------------------------------
// Tag row view-model
// ---------------------------------------------------------------------------

/// <summary>A single editable tag row bound to the tag ListView.</summary>
internal sealed class TagRowModel : INotifyPropertyChanged
{
    private string _value = string.Empty;

    public string Key { get; init; } = string.Empty;

    public string Value
    {
        get => _value;
        set
        {
            if (_value == value) return;
            _value = value;
            OnPropertyChanged();
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged([CallerMemberName] string? name = null)
        => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}

// ---------------------------------------------------------------------------
// Page code-behind
// ---------------------------------------------------------------------------

/// <summary>
/// Metadata page: opens a media file, displays all its tags in an editable
/// grid, and saves or reverts changes via MmCore.
/// </summary>
public sealed partial class MetadataPage : Page
{
    /// <summary>Tag rows bound to the tag ListView.</summary>
    public ObservableCollection<TagRowModel> TagRows { get; } = [];

    // Original values stored for Revert
    private System.Collections.Generic.IReadOnlyList<TagEntry> _originalTags = [];

    // Currently open file path
    private string _currentPath = string.Empty;

    public MetadataPage()
    {
        this.InitializeComponent();
    }

    // ── File picker ────────────────────────────────────────────────────────

    /// <summary>Opens a file picker filtered to audio formats.</summary>
    private async void OpenFileButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FileOpenPicker();
        picker.SuggestedStartLocation = PickerLocationId.MusicLibrary;

        // Audio file type filters
        foreach (string ext in new[] { ".mp3", ".flac", ".m4a", ".aac", ".ogg", ".opus", ".wav", ".aiff" })
            picker.FileTypeFilter.Add(ext);

        // Attach picker to the window handle
        var hwnd = WindowNative.GetWindowHandle(App.MainWindow);
        InitializeWithWindow.Initialize(picker, hwnd);

        var file = await picker.PickSingleFileAsync();
        if (file is not null)
        {
            await LoadFileAsync(file.Path);
        }
    }

    // ── Load ────────────────────────────────────────────────────────────────

    /// <summary>Loads metadata and audio properties for the given path.</summary>
    private async Task LoadFileAsync(string path)
    {
        _currentPath = path;
        FilePathBox.Text = path;
        SetLoading(true);
        TagRows.Clear();
        TagList.Visibility = Visibility.Collapsed;
        EmptyState.Visibility = Visibility.Collapsed;
        StatusText.Text = "Loading…";
        AudioPropsText.Text = "—";

        // Read tags and audio properties on a background thread
        var tags = await Task.Run(() => MmCore.Instance.GetMetadata(path));

        // Build audio properties summary from extension (stub: DLL not yet linked)
        string ext = Path.GetExtension(path).TrimStart('.').ToUpperInvariant();
        AudioPropsText.Text = $"{ext} · 44100 Hz · 2ch · — (stub)";

        _originalTags = tags;
        foreach (TagEntry tag in tags)
        {
            TagRows.Add(new TagRowModel { Key = tag.Key, Value = tag.Value });
        }

        if (TagRows.Count > 0)
        {
            TagList.Visibility = Visibility.Visible;
            StatusText.Text = $"{TagRows.Count} tags loaded.";
        }
        else
        {
            EmptyState.Visibility = Visibility.Visible;
            StatusText.Text = "No tags found.";
        }

        // Load cover art: look for a "coverart" or "picture" tag URL
        // (set by the lookup provider in M6)
        string? artUrl = null;
        foreach (TagEntry tag in tags)
        {
            if (tag.Key.Equals("coverart", StringComparison.OrdinalIgnoreCase) ||
                tag.Key.Equals("picture",  StringComparison.OrdinalIgnoreCase))
            {
                artUrl = tag.Value;
                break;
            }
        }
        LoadCoverArt(artUrl);

        SaveButton.IsEnabled   = true;
        RevertButton.IsEnabled = true;
        SetLoading(false);
    }

    // ── Save ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Says why Save must refuse, or <see langword="null"/> when it is safe
    /// to write tag changes to the real file. Checked at the top of
    /// <see cref="SaveButton_Click"/> so the button being (mis-)enabled can
    /// never be the only thing standing between a click and a real write —
    /// mirrors <see cref="ScanPage.ExecuteRefusalReason"/>, added for the
    /// same reason (issue #222). Test Mode matters here specifically because,
    /// unlike the Rust engine's tag-write path, this Windows build's Test
    /// Mode is only an in-app flag (see <see cref="MmCore.SetTestMode"/>'s
    /// comment: no P/Invoke call exists yet) — it does NOT divert the write
    /// to a safe copy, so leaving it on must stop the save outright rather
    /// than let a tester believe their real file was never touched.
    /// </summary>
    /// <param name="engineAvailable">Whether mm_ffi.dll was found (<see cref="MmCore.IsEngineAvailable"/>).</param>
    /// <param name="testModeEnabled">Whether Test Mode is currently on.</param>
    /// <returns>A user-facing refusal message, or null when Save may proceed.</returns>
    internal static string? SaveRefusalReason(bool engineAvailable, bool testModeEnabled)
    {
        if (!engineAvailable)
        {
            return "Save unavailable: this build is running without the MeedyaManager engine. (issue #222)";
        }
        if (testModeEnabled)
        {
            return "Nothing was saved. Test Mode is on, and in this Windows build Test Mode does not " +
                   "make safe copies of files — turn it off in Settings to save tag changes.";
        }
        return null;
    }

    /// <summary>Writes all edited tag values back to the file.</summary>
    private async void SaveButton_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrEmpty(_currentPath)) return;

        // Refuse before any write is attempted, regardless of whether the
        // button itself is (mis-)enabled — this is the last line of defence
        // against saving for real while a tester believes Test Mode is
        // protecting them (issue #222).
        string? refusal = SaveRefusalReason(MmCore.IsEngineAvailable, MmCore.Instance.TestModeEnabled());
        if (refusal is not null)
        {
            StatusText.Text = refusal;
            return;
        }

        SetLoading(true);
        StatusText.Text = "Saving…";

        // Snapshot current rows as TagEntry list
        var tags = TagRows.Select(r => new TagEntry(r.Key, r.Value)).ToList();

        bool ok = await Task.Run(() => MmCore.Instance.WriteMetadata(_currentPath, tags));

        // WriteMetadata now returns false (rather than pretending success)
        // when the engine is missing, so distinguish that from a genuine
        // write failure — otherwise "check permissions" sends the user
        // hunting for a problem that isn't theirs (issue #222).
        StatusText.Text = ok
            ? "Saved successfully."
            : !MmCore.IsEngineAvailable
                ? "Save unavailable: this build is running without the MeedyaManager engine. (issue #222)"
                : "Save failed — check permissions.";
        SetLoading(false);
    }

    // ── Revert ──────────────────────────────────────────────────────────────

    /// <summary>Discards edits and restores the original tag values.</summary>
    private void RevertButton_Click(object sender, RoutedEventArgs e)
    {
        // Match rows by key and reset their Value to the original
        foreach (TagRowModel row in TagRows)
        {
            TagEntry? original = _originalTags.FirstOrDefault(t => t.Key == row.Key);
            if (original is not null)
                row.Value = original.Value;
        }
        StatusText.Text = "Changes reverted.";
    }

    // ── Cover Art ────────────────────────────────────────────────────────────

    /// <summary>
    /// Loads cover art from <paramref name="url"/> into the CoverArtImage control,
    /// or shows the placeholder icon when no URL is available.
    /// </summary>
    private void LoadCoverArt(string? url)
    {
        if (string.IsNullOrEmpty(url))
        {
            // Show placeholder, hide image
            CoverArtImage.Visibility       = Visibility.Collapsed;
            CoverArtPlaceholder.Visibility = Visibility.Visible;
            return;
        }

        if (!Uri.TryCreate(url, UriKind.Absolute, out Uri? uri))
        {
            CoverArtImage.Visibility       = Visibility.Collapsed;
            CoverArtPlaceholder.Visibility = Visibility.Visible;
            return;
        }

        // Load into a BitmapImage (async — WinUI 3 handles the download)
        var bmp = new Microsoft.UI.Xaml.Media.Imaging.BitmapImage(uri);
        CoverArtImage.Source           = bmp;
        CoverArtImage.Visibility       = Visibility.Visible;
        CoverArtPlaceholder.Visibility = Visibility.Collapsed;
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    private void SetLoading(bool active)
    {
        LoadProgress.IsActive    = active;
        OpenFileButton.IsEnabled = !active;
        SaveButton.IsEnabled     = !active && !string.IsNullOrEmpty(_currentPath);
        RevertButton.IsEnabled   = !active && !string.IsNullOrEmpty(_currentPath);
    }
}
