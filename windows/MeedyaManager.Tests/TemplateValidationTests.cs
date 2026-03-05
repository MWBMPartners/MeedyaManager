// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Template Validation Logic Unit Tests (xUnit)
//
// Tests the rename template substitution logic used in RulesPage and
// ScanPage. No WinUI dependency — pure C# string processing.

using System.Collections.Generic;
using System.Text.RegularExpressions;
using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Template substitution replica (mirrors RulesPage code-behind logic)
// ---------------------------------------------------------------------------

internal static class TemplateLogic
{
    private static readonly Dictionary<string, string> SampleTags = new()
    {
        ["Artist"] = "Pink Floyd",
        ["Album"]  = "The Wall",
        ["Title"]  = "Comfortably Numb",
        ["Year"]   = "1979",
        ["Genre"]  = "Rock",
    };

    /// <summary>
    /// Substitutes &lt;Tag&gt; placeholders in <paramref name="template"/>
    /// using the sample tag dictionary. Returns an empty string for an
    /// empty or whitespace-only template.
    /// </summary>
    public static string ComputePreview(string template)
    {
        string trimmed = template.Trim();
        if (string.IsNullOrEmpty(trimmed)) return string.Empty;

        string result = trimmed;
        foreach (var (key, value) in SampleTags)
        {
            result = Regex.Replace(
                result,
                Regex.Escape($"<{key}>"),
                value,
                RegexOptions.IgnoreCase
            );
        }
        return result;
    }

    /// <summary>
    /// Returns true when the template contains at least one valid &lt;Tag&gt;
    /// placeholder (i.e. it is non-empty and has at least one token).
    /// </summary>
    public static (bool IsValid, string Message) ValidateTemplate(string template)
    {
        string trimmed = template.Trim();
        if (string.IsNullOrEmpty(trimmed))
            return (false, "Template is empty.");

        bool hasToken = Regex.IsMatch(trimmed, @"<\w+>");
        if (!hasToken)
            return (false, "Template must contain at least one <Tag> placeholder.");

        return (true, "Valid template.");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class TemplateValidationTests
{
    // ── ComputePreview ───────────────────────────────────────────────────────

    [Fact]
    public void ComputePreview_Should_SubstituteArtistAndTitle()
    {
        string result = TemplateLogic.ComputePreview("<Artist> - <Title>");
        result.Should().Be("Pink Floyd - Comfortably Numb");
    }

    [Fact]
    public void ComputePreview_Should_SubstituteYear()
    {
        string result = TemplateLogic.ComputePreview("<Year>");
        result.Should().Be("1979");
    }

    [Fact]
    public void ComputePreview_Should_ReturnEmpty_ForEmptyTemplate()
    {
        TemplateLogic.ComputePreview("").Should().BeEmpty();
    }

    [Fact]
    public void ComputePreview_Should_ReturnEmpty_ForWhitespaceTemplate()
    {
        TemplateLogic.ComputePreview("   ").Should().BeEmpty();
    }

    [Fact]
    public void ComputePreview_Should_BeCaseInsensitive()
    {
        string result = TemplateLogic.ComputePreview("<artist> - <TITLE>");
        result.Should().Be("Pink Floyd - Comfortably Numb");
    }

    [Fact]
    public void ComputePreview_Should_LeaveUnknownTagsIntact()
    {
        string result = TemplateLogic.ComputePreview("<Artist> - <TrackNumber>");
        // <TrackNumber> is not in SampleTags, so it remains verbatim
        result.Should().Be("Pink Floyd - <TrackNumber>");
    }

    [Fact]
    public void ComputePreview_Should_SubstituteMultipleTags()
    {
        string result = TemplateLogic.ComputePreview("<Artist>/<Album>/<Title>");
        result.Should().Be("Pink Floyd/The Wall/Comfortably Numb");
    }

    // ── ValidateTemplate ─────────────────────────────────────────────────────

    [Fact]
    public void ValidateTemplate_Should_ReturnValid_ForTagTemplate()
    {
        var (isValid, _) = TemplateLogic.ValidateTemplate("<Artist> - <Title>");
        isValid.Should().BeTrue();
    }

    [Fact]
    public void ValidateTemplate_Should_ReturnInvalid_ForEmpty()
    {
        var (isValid, msg) = TemplateLogic.ValidateTemplate("");
        isValid.Should().BeFalse();
        msg.Should().Contain("empty");
    }

    [Fact]
    public void ValidateTemplate_Should_ReturnInvalid_ForNoPlaceholders()
    {
        var (isValid, msg) = TemplateLogic.ValidateTemplate("just a static name");
        isValid.Should().BeFalse();
        msg.Should().Contain("placeholder");
    }

    [Fact]
    public void ValidateTemplate_Should_ReturnValid_ForSingleTag()
    {
        var (isValid, _) = TemplateLogic.ValidateTemplate("<Title>");
        isValid.Should().BeTrue();
    }

    [Fact]
    public void ValidateTemplate_Message_Should_SayValid_OnSuccess()
    {
        var (_, msg) = TemplateLogic.ValidateTemplate("<Artist>");
        msg.Should().Be("Valid template.");
    }
}
