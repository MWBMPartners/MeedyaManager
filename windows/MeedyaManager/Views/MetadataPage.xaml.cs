// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
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

        SaveButton.IsEnabled   = true;
        RevertButton.IsEnabled = true;
        SetLoading(false);
    }

    // ── Save ────────────────────────────────────────────────────────────────

    /// <summary>Writes all edited tag values back to the file.</summary>
    private async void SaveButton_Click(object sender, RoutedEventArgs e)
    {
        if (string.IsNullOrEmpty(_currentPath)) return;

        SetLoading(true);
        StatusText.Text = "Saving…";

        // Snapshot current rows as TagEntry list
        var tags = TagRows.Select(r => new TagEntry(r.Key, r.Value)).ToList();

        bool ok = await Task.Run(() => MmCore.Instance.WriteMetadata(_currentPath, tags));

        StatusText.Text = ok ? "Saved successfully." : "Save failed — check permissions.";
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

    // ── Helpers ─────────────────────────────────────────────────────────────

    private void SetLoading(bool active)
    {
        LoadProgress.IsActive    = active;
        OpenFileButton.IsEnabled = !active;
        SaveButton.IsEnabled     = !active && !string.IsNullOrEmpty(_currentPath);
        RevertButton.IsEnabled   = !active && !string.IsNullOrEmpty(_currentPath);
    }
}
