// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — LookupResultRow Logic Unit Tests (xUnit)
//
// Tests computed display properties of the lookup result row view-model.

using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of the production LookupResultRow (without WinUI Brush dependency)
// ---------------------------------------------------------------------------

internal sealed class LookupResultRow
{
    public string  Provider       { get; init; } = string.Empty;
    public string? Title          { get; init; }
    public string? Artist         { get; init; }
    public string? Album          { get; init; }
    public int?    Year           { get; init; }
    public string? Genre          { get; init; }
    public double  Score          { get; init; }

    public string DisplayTitle    => Title ?? "(no title)";
    public string DisplaySubtitle => Artist ?? string.Empty;
    public string ScoreText       => Score.ToString("F2");

    /// <summary>Score category for colour coding.</summary>
    public string ScoreCategory   => Score >= 0.8 ? "High" : Score >= 0.5 ? "Medium" : "Low";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class LookupResultRowTests
{
    private static LookupResultRow Make(
        string? title  = "Comfortably Numb",
        string? artist = "Pink Floyd",
        int?    year   = 1979,
        double  score  = 0.92)
        => new()
        {
            Provider = "MusicBrainz",
            Title    = title,
            Artist   = artist,
            Year     = year,
            Score    = score,
        };

    [Fact]
    public void DisplayTitle_Should_ReturnTitle_WhenSet()
        => Make(title: "Bohemian Rhapsody").DisplayTitle.Should().Be("Bohemian Rhapsody");

    [Fact]
    public void DisplayTitle_Should_Fallback_WhenNull()
        => Make(title: null).DisplayTitle.Should().Be("(no title)");

    [Fact]
    public void DisplaySubtitle_Should_ReturnArtist_WhenSet()
        => Make(artist: "Pink Floyd").DisplaySubtitle.Should().Be("Pink Floyd");

    [Fact]
    public void DisplaySubtitle_Should_BeEmpty_WhenArtistNull()
        => Make(artist: null).DisplaySubtitle.Should().BeEmpty();

    [Fact]
    public void ScoreText_Should_FormatToTwoDecimalPlaces()
        => Make(score: 0.95).ScoreText.Should().Be("0.95");

    [Fact]
    public void ScoreCategory_Should_BeHigh_WhenScoreAbove0_8()
        => Make(score: 0.85).ScoreCategory.Should().Be("High");

    [Fact]
    public void ScoreCategory_Should_BeMedium_WhenScoreBetween0_5_And_0_8()
        => Make(score: 0.65).ScoreCategory.Should().Be("Medium");

    [Fact]
    public void ScoreCategory_Should_BeLow_WhenScoreBelow0_5()
        => Make(score: 0.35).ScoreCategory.Should().Be("Low");

    [Fact]
    public void ScoreCategory_BoundaryAt_0_8_IsHigh()
        => Make(score: 0.8).ScoreCategory.Should().Be("High");

    [Fact]
    public void ScoreCategory_BoundaryAt_0_5_IsMedium()
        => Make(score: 0.5).ScoreCategory.Should().Be("Medium");

    [Fact]
    public void Provider_Should_BeStored()
        => Make().Provider.Should().Be("MusicBrainz");

    [Fact]
    public void Year_Should_BeStored()
        => Make(year: 1979).Year.Should().Be(1979);
}
