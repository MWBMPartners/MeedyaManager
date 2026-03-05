// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — ProviderEntry and LookupPage Logic Unit Tests (xUnit)
//
// Tests provider list initialisation, enabled/disabled defaults, and
// toggle behaviour for the Lookup page provider checklist.

using System.Collections.Generic;
using System.Linq;
using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of the production ProviderEntry (without WinUI dependency)
// ---------------------------------------------------------------------------

internal sealed class ProviderEntry
{
    public string Label     { get; init; } = string.Empty;
    public bool   IsStub    { get; init; }
    public bool   IsEnabled { get; set; }
    public double FontSize  => IsStub ? 11.0 : 13.0;
}

/// <summary>Pure logic extracted from LookupPage for testability.</summary>
internal static class LookupPageLogic
{
    public static List<ProviderEntry> DefaultProviders() =>
    [
        new() { Label = "MusicBrainz",      IsEnabled = true,  IsStub = false },
        new() { Label = "Discogs",           IsEnabled = true,  IsStub = false },
        new() { Label = "Last.fm",           IsEnabled = true,  IsStub = false },
        new() { Label = "iTunes",            IsEnabled = true,  IsStub = false },
        new() { Label = "Spotify *",         IsEnabled = false, IsStub = true  },
        new() { Label = "Amazon Music *",    IsEnabled = false, IsStub = true  },
        new() { Label = "Pandora *",         IsEnabled = false, IsStub = true  },
        new() { Label = "iHeart *",          IsEnabled = false, IsStub = true  },
        new() { Label = "SoundCloud *",      IsEnabled = false, IsStub = true  },
        new() { Label = "TMDB",              IsEnabled = true,  IsStub = false },
        new() { Label = "TheTVDB",           IsEnabled = true,  IsStub = false },
        new() { Label = "OMDB",              IsEnabled = true,  IsStub = false },
        new() { Label = "OpenLibrary",       IsEnabled = true,  IsStub = false },
        new() { Label = "EIDR",              IsEnabled = true,  IsStub = false },
        new() { Label = "iTunes Podcast*",   IsEnabled = false, IsStub = true  },
        new() { Label = "Spotify Podcast*",  IsEnabled = false, IsStub = true  },
        new() { Label = "ListenNotes",       IsEnabled = true,  IsStub = false },
        new() { Label = "PodcastIndex",      IsEnabled = true,  IsStub = false },
        new() { Label = "Audible",           IsEnabled = true,  IsStub = false },
    ];
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class ProviderEntryTests
{
    [Fact]
    public void DefaultProviders_Should_Have19Entries()
    {
        var providers = LookupPageLogic.DefaultProviders();
        providers.Should().HaveCount(19);
    }

    [Fact]
    public void DefaultProviders_Should_Have13EnabledConcrete()
    {
        var providers = LookupPageLogic.DefaultProviders();
        providers.Count(p => p.IsEnabled && !p.IsStub).Should().Be(13);
    }

    [Fact]
    public void DefaultProviders_Should_Have6DisabledStubs()
    {
        var providers = LookupPageLogic.DefaultProviders();
        providers.Count(p => p.IsStub).Should().Be(6);
    }

    [Fact]
    public void AllStubs_Should_BeDisabledByDefault()
    {
        var stubs = LookupPageLogic.DefaultProviders().Where(p => p.IsStub);
        stubs.Should().OnlyContain(p => !p.IsEnabled);
    }

    [Fact]
    public void AllConcrete_Should_BeEnabledByDefault()
    {
        var concrete = LookupPageLogic.DefaultProviders().Where(p => !p.IsStub);
        concrete.Should().OnlyContain(p => p.IsEnabled);
    }

    [Fact]
    public void ProviderEntry_FontSize_Should_Be11_ForStub()
    {
        var stub = new ProviderEntry { Label = "X *", IsStub = true };
        stub.FontSize.Should().Be(11.0);
    }

    [Fact]
    public void ProviderEntry_FontSize_Should_Be13_ForConcrete()
    {
        var concrete = new ProviderEntry { Label = "MusicBrainz", IsStub = false };
        concrete.FontSize.Should().Be(13.0);
    }

    [Fact]
    public void MusicBrainz_Should_BeInList()
    {
        var providers = LookupPageLogic.DefaultProviders();
        providers.Should().Contain(p => p.Label == "MusicBrainz");
    }

    [Fact]
    public void AllLabels_Should_BeUnique()
    {
        var labels = LookupPageLogic.DefaultProviders().Select(p => p.Label).ToList();
        labels.Should().OnlyHaveUniqueItems();
    }

    [Fact]
    public void Toggle_Should_DisableEnabledProvider()
    {
        var p = new ProviderEntry { Label = "MusicBrainz", IsEnabled = true, IsStub = false };
        p.IsEnabled = !p.IsEnabled;
        p.IsEnabled.Should().BeFalse();
    }

    [Fact]
    public void Toggle_Should_EnableDisabledProvider()
    {
        var p = new ProviderEntry { Label = "Spotify *", IsEnabled = false, IsStub = true };
        p.IsEnabled = !p.IsEnabled;
        p.IsEnabled.Should().BeTrue();
    }

    [Fact]
    public void TotalEnabled_Should_MatchCountOfNonStubs()
    {
        var providers = LookupPageLogic.DefaultProviders();
        int enabledCount = providers.Count(p => p.IsEnabled);
        int concreteCount = providers.Count(p => !p.IsStub);
        enabledCount.Should().Be(concreteCount);
    }
}
