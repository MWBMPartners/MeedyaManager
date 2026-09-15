// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Save guard unit tests (Windows / xUnit, issue #222)
//
// Tests the refusal logic that stops MetadataPage's Save button from writing
// tag changes to the real file when there is no honest way to say the write
// is safe: either the engine (mm_ffi.dll) is missing, so there is no real
// write path at all, or Test Mode is on — and in this Windows build Test
// Mode is only an in-app flag (see MmCore.SetTestMode's comment: no
// P/Invoke call exists yet), so it does NOT divert the write to a safe copy
// the way App.xaml.cs's pre-release dialog used to claim. A tester who
// believed that claim and pressed Save was rewriting the real file.
//
// This file is a standalone replica of MetadataPage.SaveRefusalReason, not a
// reference to the real WinUI 3 type, because this test project targets
// plain net8.0 so it can run without the WinUI runtime (see the existing
// tests in this project, e.g. TemplateValidationTests.cs, and its sibling
// ScanExecuteGuardTests.cs, which tests the equivalent guard on the Execute
// button). It runs nowhere today — Windows CI has never been green (issue
// #148) — so this only proves the logic is correct on the machine that
// eventually does build it.

using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Replica of MetadataPage.SaveRefusalReason (Views/MetadataPage.xaml.cs)
// ---------------------------------------------------------------------------

/// <summary>
/// Pure-logic replica of the Save refusal check added to MetadataPage for
/// issue #222. Kept in lock-step with the real method by hand — a WinUI
/// project cannot be referenced from this net8.0 test project — so any
/// future edit to the real method's ordering or wording must be mirrored
/// here too.
/// </summary>
internal static class MetadataSaveGuardReplica
{
    public static string? SaveRefusalReason(bool engineAvailable, bool testModeEnabled)
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class MetadataSaveGuardTests
{
    [Fact]
    public void SaveRefusalReason_Should_RefuseWhenEngineMissing()
    {
        string? reason = MetadataSaveGuardReplica.SaveRefusalReason(engineAvailable: false, testModeEnabled: false);

        reason.Should().NotBeNull();
        reason.Should().Contain("engine");
        reason.Should().Contain("#222");
    }

    [Fact]
    public void SaveRefusalReason_Should_RefuseWhenTestModeOn()
    {
        string? reason = MetadataSaveGuardReplica.SaveRefusalReason(engineAvailable: true, testModeEnabled: true);

        reason.Should().NotBeNull();
        reason.Should().Contain("Test Mode");
    }

    [Fact]
    public void SaveRefusalReason_Should_AllowWhenEngineAvailableAndTestModeOff()
    {
        string? reason = MetadataSaveGuardReplica.SaveRefusalReason(engineAvailable: true, testModeEnabled: false);

        reason.Should().BeNull();
    }

    [Fact]
    public void SaveRefusalReason_Should_PreferEngineMissingReason_WhenBothConditionsHold()
    {
        // Engine missing takes priority over Test Mode: there is no point
        // telling the user to turn off Test Mode when there was never a real
        // write path to begin with.
        string? reason = MetadataSaveGuardReplica.SaveRefusalReason(engineAvailable: false, testModeEnabled: true);

        reason.Should().NotBeNull();
        reason.Should().Contain("engine");
        reason.Should().NotContain("Test Mode");
    }
}
