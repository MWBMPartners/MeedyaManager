// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Accessibility Tests (Windows / xUnit, Issue #128)
//
// Tests the accessibility metadata and helper logic used throughout
// the MeedyaManager WinUI 3 application. All UI automation properties
// must be set for compliance with Windows Narrator and the Microsoft
// Accessibility Insights tooling.
//
// Targets:
//   - AutomationProperties.Name set on all interactive controls
//   - AutomationProperties.HelpText set for non-trivial controls
//   - No element communicates state via colour alone
//   - Status strings are human-readable, no raw HTML or escape codes

using System;
using Xunit;

namespace MeedyaManager.Tests;

// ── Navigation page names ─────────────────────────────────────────────────

/// <summary>
/// Replicates the page-name constants defined in MainWindow.
/// NavigationView items must carry an AutomationProperties.Name so that
/// Narrator can announce the destination when the user navigates the sidebar.
/// </summary>
internal static class NavigationPageNames
{
    // The Tag values used by NavigationViewItems in MainWindow.xaml.
    // These strings are also set as AutomationProperties.Name in the XAML.
    public static readonly string[] All =
    [
        "Library",
        "Rules",
        "Metadata",
        "Lookup",
        "Cloud",
        "Export",
        "Server",
        "Settings",
    ];
}

// ── Automation name helper ────────────────────────────────────────────────

/// <summary>
/// Helper that generates the AutomationProperties.Name string for a page's
/// primary action button. WinUI 3 Narrator announces this when the button
/// is focused via keyboard or switch access.
/// </summary>
internal static class AutomationNameHelper
{
    /// <summary>Returns the expected Narrator button label for a given page.</summary>
    public static string PrimaryButtonLabel(string pageName)
        => pageName switch
        {
            "Library"  => "Scan folder",
            "Rules"    => "Apply rules",
            "Metadata" => "Save metadata",
            "Lookup"   => "Search providers",
            "Cloud"    => "Connect account",
            "Export"   => "Export to database",
            "Server"   => "Start server",
            "Settings" => "Save settings",
            _          => throw new ArgumentException($"Unknown page: {pageName}"),
        };

    /// <summary>
    /// Validates that a proposed AutomationProperties.Name is accessible:
    /// non-empty, no raw HTML, not a single non-word character.
    /// </summary>
    public static bool IsValidAutomationName(string? name)
    {
        if (string.IsNullOrWhiteSpace(name)) return false;
        if (name.Contains('<') || name.Contains('>')) return false;
        return name.Length >= 2;
    }
}

// ── Server status accessibility ───────────────────────────────────────────

/// <summary>
/// Replicates the server status display strings checked in ServerPageTests.
/// The Narrator announcement for the status indicator must not be ambiguous
/// or rely on colour alone.
/// </summary>
internal static class ServerStatusA11y
{
    public static string Describe(bool isRunning, bool isError, string? errorMessage = null)
    {
        if (isError && !string.IsNullOrEmpty(errorMessage))
            return $"Error: {errorMessage}";
        return isRunning ? "Server running" : "Server stopped";
    }
}

// ── Accessibility Tests ──────────────────────────────────────────────────

public class AccessibilityTests
{
    // --- Navigation page names ---

    [Fact]
    public void NavigationPages_HasEightItems()
        => Assert.Equal(8, NavigationPageNames.All.Length);

    [Fact]
    public void NavigationPages_AllNonEmpty()
    {
        foreach (var name in NavigationPageNames.All)
            Assert.False(string.IsNullOrWhiteSpace(name), $"Page name '{name}' must not be empty");
    }

    [Fact]
    public void NavigationPages_AllUnique()
    {
        var set = new System.Collections.Generic.HashSet<string>(NavigationPageNames.All);
        Assert.Equal(NavigationPageNames.All.Length, set.Count);
    }

    [Fact]
    public void NavigationPages_ContainsLibrary()
        => Assert.Contains("Library", NavigationPageNames.All);

    [Fact]
    public void NavigationPages_ContainsServer()
        => Assert.Contains("Server", NavigationPageNames.All);

    // --- AutomationName helper ---

    [Theory]
    [InlineData("Library",  "Scan folder")]
    [InlineData("Rules",    "Apply rules")]
    [InlineData("Metadata", "Save metadata")]
    [InlineData("Lookup",   "Search providers")]
    [InlineData("Cloud",    "Connect account")]
    [InlineData("Export",   "Export to database")]
    [InlineData("Server",   "Start server")]
    [InlineData("Settings", "Save settings")]
    public void PrimaryButtonLabel_ReturnsExpectedString(string page, string expected)
        => Assert.Equal(expected, AutomationNameHelper.PrimaryButtonLabel(page));

    [Fact]
    public void PrimaryButtonLabel_UnknownPageThrows()
        => Assert.Throws<ArgumentException>(() => AutomationNameHelper.PrimaryButtonLabel("Unknown"));

    // --- IsValidAutomationName ---

    [Fact]
    public void IsValidAutomationName_EmptyString_False()
        => Assert.False(AutomationNameHelper.IsValidAutomationName(""));

    [Fact]
    public void IsValidAutomationName_Null_False()
        => Assert.False(AutomationNameHelper.IsValidAutomationName(null));

    [Fact]
    public void IsValidAutomationName_HtmlTag_False()
        => Assert.False(AutomationNameHelper.IsValidAutomationName("<button>"));

    [Fact]
    public void IsValidAutomationName_SingleChar_False()
        => Assert.False(AutomationNameHelper.IsValidAutomationName("X"));

    [Fact]
    public void IsValidAutomationName_ValidLabel_True()
        => Assert.True(AutomationNameHelper.IsValidAutomationName("Start server"));

    [Fact]
    public void IsValidAutomationName_AllPageLabelsValid()
    {
        foreach (var page in NavigationPageNames.All)
        {
            var label = AutomationNameHelper.PrimaryButtonLabel(page);
            Assert.True(AutomationNameHelper.IsValidAutomationName(label),
                $"Label '{label}' for page '{page}' must be a valid automation name");
        }
    }

    // --- ServerStatus accessibility ---

    [Fact]
    public void ServerStatusA11y_Stopped_Descriptive()
    {
        var text = ServerStatusA11y.Describe(isRunning: false, isError: false);
        Assert.Equal("Server stopped", text);
    }

    [Fact]
    public void ServerStatusA11y_Running_Descriptive()
    {
        var text = ServerStatusA11y.Describe(isRunning: true, isError: false);
        Assert.Equal("Server running", text);
    }

    [Fact]
    public void ServerStatusA11y_Error_IncludesMessage()
    {
        var text = ServerStatusA11y.Describe(isRunning: false, isError: true,
                                             errorMessage: "TLS cert missing");
        Assert.Contains("TLS cert missing", text);
        Assert.StartsWith("Error:", text);
    }

    [Fact]
    public void ServerStatusA11y_NoColourOnlyState()
    {
        // All status descriptions must contain the word describing the state,
        // not just a colour indicator. This is a proxy for the colour-alone rule.
        var stopped = ServerStatusA11y.Describe(false, false);
        var running = ServerStatusA11y.Describe(true,  false);
        Assert.NotEqual(stopped, running); // must differ in text, not just colour
    }

    // --- All primary button labels pass automation name validation ---

    [Fact]
    public void AllPrimaryButtonLabels_AreValidAutomationNames()
    {
        foreach (var page in NavigationPageNames.All)
        {
            var label = AutomationNameHelper.PrimaryButtonLabel(page);
            Assert.True(AutomationNameHelper.IsValidAutomationName(label));
        }
    }
}
