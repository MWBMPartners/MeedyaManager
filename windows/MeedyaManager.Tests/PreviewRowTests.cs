// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — PreviewRow Logic Unit Tests (xUnit)
//
// Tests badge text, executability, and display name computation for the
// rename preview row view-model.  No WinUI dependency — pure C# logic.

using System.IO;
using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of the production PreviewRow (without WinUI Brush dependency)
// ---------------------------------------------------------------------------

internal sealed class PreviewRow
{
    public string  SourceName       { get; }
    public string  Arrow            { get; }
    public string  BadgeText        { get; }
    public bool    IsExecutable     { get; }
    public string  SourcePath       { get; }
    public string  DestinationPath  { get; }
    public bool    IsConflict       { get; }
    public bool    IsUnchanged      { get; }

    public PreviewRow(
        string source,
        string destination,
        bool conflict,
        bool unchanged)
    {
        SourcePath      = source;
        DestinationPath = destination;
        IsConflict      = conflict;
        IsUnchanged     = unchanged;
        SourceName      = Path.GetFileName(source);
        string destName = Path.GetFileName(destination);
        Arrow           = $"→  {destName}";

        if (conflict)       { BadgeText = "Conflict";  IsExecutable = false; }
        else if (unchanged) { BadgeText = "Unchanged"; IsExecutable = false; }
        else                { BadgeText = "Rename";    IsExecutable = true;  }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class PreviewRowTests
{
    private static PreviewRow MakeRename()
        => new(@"C:\Music\track01.mp3",
               @"C:\Music\Pink Floyd - Comfortably Numb.mp3",
               conflict: false, unchanged: false);

    private static PreviewRow MakeConflict()
        => new(@"C:\Music\track01.mp3",
               @"C:\Music\track01.mp3",
               conflict: true, unchanged: false);

    private static PreviewRow MakeUnchanged()
        => new(@"C:\Music\track01.mp3",
               @"C:\Music\track01.mp3",
               conflict: false, unchanged: true);

    [Fact]
    public void SourceName_Should_ReturnBasename()
        => MakeRename().SourceName.Should().Be("track01.mp3");

    [Fact]
    public void Arrow_Should_ContainDestinationBasename()
        => MakeRename().Arrow.Should().Contain("Pink Floyd - Comfortably Numb.mp3");

    [Fact]
    public void BadgeText_Should_Be_Rename_ForNormalRename()
        => MakeRename().BadgeText.Should().Be("Rename");

    [Fact]
    public void BadgeText_Should_Be_Conflict_ForConflict()
        => MakeConflict().BadgeText.Should().Be("Conflict");

    [Fact]
    public void BadgeText_Should_Be_Unchanged_ForUnchanged()
        => MakeUnchanged().BadgeText.Should().Be("Unchanged");

    [Fact]
    public void IsExecutable_Should_BeTrue_ForRename()
        => MakeRename().IsExecutable.Should().BeTrue();

    [Fact]
    public void IsExecutable_Should_BeFalse_ForConflict()
        => MakeConflict().IsExecutable.Should().BeFalse();

    [Fact]
    public void IsExecutable_Should_BeFalse_ForUnchanged()
        => MakeUnchanged().IsExecutable.Should().BeFalse();

    [Fact]
    public void SourcePath_Should_BeStored()
        => MakeRename().SourcePath.Should().Be(@"C:\Music\track01.mp3");

    [Fact]
    public void DestinationPath_Should_BeStored()
        => MakeRename().DestinationPath.Should().Be(@"C:\Music\Pink Floyd - Comfortably Numb.mp3");
}
