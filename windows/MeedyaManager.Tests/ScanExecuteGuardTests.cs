// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Scan Execute guard unit tests (Windows / xUnit, issue #222)
//
// Tests the refusal logic that stops ScanPage's Execute button from renaming
// files when there is no trustworthy preview to act on: either the engine
// (mm_ffi.dll) is missing, so ScanDirectory's results cannot be real, or Test
// Mode is on, which the Rust renamer has no way to honour for a rename (it
// only ever diverts tag writes to a `_MeedyaManager` copy — see Dev_Notes.md).
//
// This file is a standalone replica of ScanPage.ExecuteRefusalReason, not a
// reference to the real WinUI 3 type, because this test project targets
// plain net8.0 so it can run without the WinUI runtime (see the existing
// tests in this project, e.g. TemplateValidationTests.cs). It runs nowhere
// today — Windows CI has never been green (issue #148) — so this only proves
// the logic is correct on the machine that eventually does build it.

using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of ScanPage.ExecuteRefusalReason (Views/ScanPage.xaml.cs)
// ---------------------------------------------------------------------------

/// <summary>
/// Pure-logic replica of the Execute refusal check added to ScanPage for
/// issue #222. Kept in lock-step with the real method by hand — a WinUI
/// project cannot be referenced from this net8.0 test project — so any
/// future edit to the real method's ordering or wording must be mirrored
/// here too.
/// </summary>
internal static class ScanExecuteGuardReplica
{
    public static string? ExecuteRefusalReason(bool engineAvailable, bool testModeEnabled)
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class ScanExecuteGuardTests
{
    [Fact]
    public void ExecuteRefusalReason_Should_RefuseWhenEngineMissing()
    {
        string? reason = ScanExecuteGuardReplica.ExecuteRefusalReason(engineAvailable: false, testModeEnabled: false);

        reason.Should().NotBeNull();
        reason.Should().Contain("engine");
        reason.Should().Contain("#222");
    }

    [Fact]
    public void ExecuteRefusalReason_Should_RefuseWhenTestModeOn()
    {
        string? reason = ScanExecuteGuardReplica.ExecuteRefusalReason(engineAvailable: true, testModeEnabled: true);

        reason.Should().NotBeNull();
        reason.Should().Contain("Test Mode");
    }

    [Fact]
    public void ExecuteRefusalReason_Should_AllowWhenEngineAvailableAndTestModeOff()
    {
        string? reason = ScanExecuteGuardReplica.ExecuteRefusalReason(engineAvailable: true, testModeEnabled: false);

        reason.Should().BeNull();
    }

    [Fact]
    public void ExecuteRefusalReason_Should_PreferEngineMissingReason_WhenBothConditionsHold()
    {
        // Engine missing takes priority over Test Mode: there is no point
        // telling the user to turn off Test Mode when the previews they'd
        // be acting on were never real to begin with.
        string? reason = ScanExecuteGuardReplica.ExecuteRefusalReason(engineAvailable: false, testModeEnabled: true);

        reason.Should().NotBeNull();
        reason.Should().Contain("engine");
        reason.Should().NotContain("Test Mode");
    }
}
