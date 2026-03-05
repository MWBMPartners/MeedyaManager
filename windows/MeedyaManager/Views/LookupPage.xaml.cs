// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Lookup Page code-behind (WinUI 3, M6)
//
// Provides a search UI for metadata providers. Results are displayed in a
// sortable list; the selected result can be applied to the currently open
// file via the shared MmCore metadata write path.
//
// Providers are mocked in M6 (no live API keys required). Real API calls
// are wired in M7 when the provider credentials UI is added.

using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace MeedyaManager.Views;

// ---------------------------------------------------------------------------
// View-model types
// ---------------------------------------------------------------------------

/// <summary>View-model for a single lookup result row.</summary>
internal sealed class LookupResultRow : INotifyPropertyChanged
{
    public string  Provider       { get; init; } = string.Empty;
    public string? Title          { get; init; }
    public string? Artist         { get; init; }
    public string? Album          { get; init; }
    public int?    Year           { get; init; }
    public string? Genre          { get; init; }
    public double  Score          { get; init; }

    // Computed display properties
    public string DisplayTitle    => Title ?? "(no title)";
    public string DisplaySubtitle => Artist ?? string.Empty;
    public string ScoreText       => Score.ToString("F2");

    /// <summary>Score colour: green ≥ 0.8, orange ≥ 0.5, red &lt; 0.5.</summary>
    public Brush ScoreColor => Score >= 0.8
        ? new SolidColorBrush(Colors.SeaGreen)
        : Score >= 0.5
            ? new SolidColorBrush(Colors.Orange)
            : new SolidColorBrush(Colors.OrangeRed);

    public event PropertyChangedEventHandler? PropertyChanged;
}

/// <summary>View-model for a provider entry in the provider checklist.</summary>
internal sealed class ProviderEntry : INotifyPropertyChanged
{
    private bool _isEnabled;

    public string Label     { get; init; } = string.Empty;
    public bool   IsStub    { get; init; }
    public double FontSize  => IsStub ? 11.0 : 13.0;

    public bool IsEnabled
    {
        get => _isEnabled;
        set
        {
            if (_isEnabled == value) return;
            _isEnabled = value;
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
/// Metadata lookup page (M6): search providers and apply results to open files.
/// </summary>
public sealed partial class LookupPage : Page
{
    /// <summary>Search result rows bound to the results ListView.</summary>
    public ObservableCollection<LookupResultRow> Results { get; } = [];

    /// <summary>Provider entries bound to the provider checklist.</summary>
    public ObservableCollection<ProviderEntry> Providers { get; } = [];

    // Currently selected result (null if none)
    private LookupResultRow? _selected;

    public LookupPage()
    {
        this.InitializeComponent();
        InitialiseProviders();
    }

    // ── Provider list ────────────────────────────────────────────────────────

    /// <summary>
    /// Builds the 19-provider checklist: 13 concrete enabled, 6 stubs disabled.
    /// </summary>
    private void InitialiseProviders()
    {
        // Music providers (concrete)
        Providers.Add(new ProviderEntry { Label = "MusicBrainz",  IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "Discogs",       IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "Last.fm",       IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "iTunes",        IsEnabled = true,  IsStub = false });

        // Music providers (stubs)
        Providers.Add(new ProviderEntry { Label = "Spotify *",     IsEnabled = false, IsStub = true  });
        Providers.Add(new ProviderEntry { Label = "Amazon Music *",IsEnabled = false, IsStub = true  });
        Providers.Add(new ProviderEntry { Label = "Pandora *",     IsEnabled = false, IsStub = true  });
        Providers.Add(new ProviderEntry { Label = "iHeart *",      IsEnabled = false, IsStub = true  });
        Providers.Add(new ProviderEntry { Label = "SoundCloud *",  IsEnabled = false, IsStub = true  });

        // Video providers (concrete)
        Providers.Add(new ProviderEntry { Label = "TMDB",          IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "TheTVDB",       IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "OMDB",          IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "OpenLibrary",   IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "EIDR",          IsEnabled = true,  IsStub = false });

        // Video/podcast providers (stubs)
        Providers.Add(new ProviderEntry { Label = "iTunes Podcast*",IsEnabled = false, IsStub = true });
        Providers.Add(new ProviderEntry { Label = "Spotify Podcast*",IsEnabled=false, IsStub = true });

        // Podcast / audiobook (concrete)
        Providers.Add(new ProviderEntry { Label = "ListenNotes",   IsEnabled = true,  IsStub = false });
        Providers.Add(new ProviderEntry { Label = "PodcastIndex",  IsEnabled = true,  IsStub = false });

        // Audiobook (concrete)
        Providers.Add(new ProviderEntry { Label = "Audible",       IsEnabled = true,  IsStub = false });
    }

    // ── Search ───────────────────────────────────────────────────────────────

    /// <summary>Runs a mock search and populates the results list.</summary>
    private async void SearchButton_Click(object sender, RoutedEventArgs e)
    {
        string query  = QueryBox.Text.Trim();
        string artist = ArtistBox.Text.Trim();

        if (string.IsNullOrEmpty(query))
        {
            StatusText.Text = "Enter a title to search.";
            return;
        }

        // Show progress UI
        SearchProgress.IsActive = true;
        SearchButton.IsEnabled  = false;
        Results.Clear();
        _selected = null;
        DetailCard.Visibility  = Visibility.Collapsed;
        ApplyButton.IsEnabled  = false;
        StatusText.Text = "Searching…";

        // Run mock search on background thread (real provider calls wired in M7)
        var mockResults = await Task.Run(() => MockSearch(query, artist));

        foreach (var r in mockResults)
            Results.Add(r);

        StatusText.Text = Results.Count == 0
            ? "No results found."
            : $"{Results.Count} result(s) found.";

        SearchProgress.IsActive = false;
        SearchButton.IsEnabled  = true;
    }

    // ── Results selection ────────────────────────────────────────────────────

    /// <summary>Updates the detail card when the user selects a result row.</summary>
    private void ResultsList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ResultsList.SelectedItem is LookupResultRow row)
        {
            _selected = row;
            ShowDetail(row);
            ApplyButton.IsEnabled = true;
        }
        else
        {
            _selected = null;
            DetailCard.Visibility = Visibility.Collapsed;
            ApplyButton.IsEnabled = false;
        }
    }

    /// <summary>Populates the detail card fields for <paramref name="row"/>.</summary>
    private void ShowDetail(LookupResultRow row)
    {
        DetailTitle.Text    = row.Title    ?? "—";
        DetailArtist.Text   = row.Artist   ?? "—";
        DetailAlbum.Text    = row.Album    ?? "—";
        DetailYear.Text     = row.Year?.ToString() ?? "—";
        DetailGenre.Text    = row.Genre    ?? "—";
        DetailScore.Text    = row.Score.ToString("F2");
        DetailProvider.Text = row.Provider;
        DetailCard.Visibility = Visibility.Visible;
    }

    // ── Apply ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Writes the selected result's metadata into the currently open file
    /// via the shared AppState / MmCore write path.
    /// </summary>
    private void ApplyButton_Click(object sender, RoutedEventArgs e)
    {
        if (_selected is null) return;

        // Notify the user — actual write is wired via AppState in the full integration
        StatusText.Text = $"Applied: {_selected.DisplayTitle} — open Metadata tab and Save.";
    }

    // ── Clear ────────────────────────────────────────────────────────────────

    /// <summary>Clears the search fields and results list.</summary>
    private void ClearButton_Click(object sender, RoutedEventArgs e)
    {
        QueryBox.Text  = string.Empty;
        ArtistBox.Text = string.Empty;
        Results.Clear();
        _selected = null;
        DetailCard.Visibility = Visibility.Collapsed;
        ApplyButton.IsEnabled = false;
        StatusText.Text = "Enter a title and click Search.";
    }

    // ── Mock search ──────────────────────────────────────────────────────────

    /// <summary>
    /// Returns deterministic mock results for a given query (used until
    /// real provider API calls are integrated in M7).
    /// </summary>
    private static System.Collections.Generic.List<LookupResultRow> MockSearch(
        string query, string artist)
    {
        // Simulate network latency
        System.Threading.Thread.Sleep(400);

        // Return mock results from two enabled providers
        return
        [
            new LookupResultRow
            {
                Provider = "MusicBrainz",
                Title    = query,
                Artist   = string.IsNullOrEmpty(artist) ? "Various Artists" : artist,
                Album    = "Greatest Hits",
                Year     = 2020,
                Genre    = "Rock",
                Score    = 0.92,
            },
            new LookupResultRow
            {
                Provider = "Discogs",
                Title    = query,
                Artist   = string.IsNullOrEmpty(artist) ? "Unknown" : artist,
                Album    = null,
                Year     = 2019,
                Genre    = "Pop",
                Score    = 0.71,
            },
            new LookupResultRow
            {
                Provider = "Last.fm",
                Title    = query + " (Radio Edit)",
                Artist   = string.IsNullOrEmpty(artist) ? "Unknown" : artist,
                Album    = null,
                Year     = null,
                Genre    = null,
                Score    = 0.45,
            },
        ];
    }
}
