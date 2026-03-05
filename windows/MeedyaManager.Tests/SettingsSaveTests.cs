// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings Save Logic Unit Tests (xUnit)
//
// Tests the JSON serialization logic used in SettingsPage.SaveSettings().
// No WinUI dependency — pure System.Text.Json usage.

using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Xunit;
using FluentAssertions;

namespace MeedyaManager.Tests;

// ---------------------------------------------------------------------------
// Settings serialisation helper (mirrors SettingsPage.SaveSettings logic)
// ---------------------------------------------------------------------------

internal static class SettingsSerialiser
{
    private static readonly JsonSerializerOptions PrettyOptions = new()
    {
        WriteIndented = true,
    };

    /// <summary>Serialises a settings snapshot to a pretty-printed JSON string.</summary>
    public static string Serialise(
        bool dryRun,
        bool recursive,
        int  debounceMs,
        string logLevel,
        bool redactPii)
    {
        var snapshot = new Dictionary<string, object>
        {
            ["dry_run"]     = dryRun,
            ["recursive"]   = recursive,
            ["debounce_ms"] = debounceMs,
            ["log_level"]   = logLevel,
            ["redact_pii"]  = redactPii,
        };
        return JsonSerializer.Serialize(snapshot, PrettyOptions);
    }

    /// <summary>Writes settings to a temp file and reads them back.</summary>
    public static JsonNode? RoundTrip(string json, string path)
    {
        File.WriteAllText(path, json);
        string readBack = File.ReadAllText(path);
        return JsonNode.Parse(readBack);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

public class SettingsSaveTests
{
    [Fact]
    public void Serialise_Should_ProduceValidJson()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "info", true);
        JsonNode? node = JsonNode.Parse(json);
        node.Should().NotBeNull();
    }

    [Fact]
    public void Serialise_Should_ContainDryRun()
    {
        string json = SettingsSerialiser.Serialise(true, false, 200, "debug", false);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["dry_run"]!.GetValue<bool>().Should().BeTrue();
    }

    [Fact]
    public void Serialise_Should_ContainRecursive()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "info", true);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["recursive"]!.GetValue<bool>().Should().BeTrue();
    }

    [Fact]
    public void Serialise_Should_ContainDebounceMs()
    {
        string json = SettingsSerialiser.Serialise(false, true, 750, "info", true);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["debounce_ms"]!.GetValue<int>().Should().Be(750);
    }

    [Fact]
    public void Serialise_Should_ContainLogLevel()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "warn", true);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["log_level"]!.GetValue<string>().Should().Be("warn");
    }

    [Fact]
    public void Serialise_Should_ContainRedactPii()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "info", false);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["redact_pii"]!.GetValue<bool>().Should().BeFalse();
    }

    [Fact]
    public void Serialise_Should_ContainAllFiveKeys()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "info", true);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc.Should().ContainKey("dry_run");
        doc.Should().ContainKey("recursive");
        doc.Should().ContainKey("debounce_ms");
        doc.Should().ContainKey("log_level");
        doc.Should().ContainKey("redact_pii");
    }

    [Fact]
    public void RoundTrip_Should_PreserveAllValues()
    {
        string path = Path.GetTempFileName();
        try
        {
            string json = SettingsSerialiser.Serialise(true, false, 1000, "trace", false);
            JsonNode? node = SettingsSerialiser.RoundTrip(json, path);
            var doc = node!.AsObject();
            doc["dry_run"]!.GetValue<bool>().Should().BeTrue();
            doc["recursive"]!.GetValue<bool>().Should().BeFalse();
            doc["debounce_ms"]!.GetValue<int>().Should().Be(1000);
            doc["log_level"]!.GetValue<string>().Should().Be("trace");
            doc["redact_pii"]!.GetValue<bool>().Should().BeFalse();
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Theory]
    [InlineData("error")]
    [InlineData("warn")]
    [InlineData("info")]
    [InlineData("debug")]
    [InlineData("trace")]
    public void ValidLogLevel_Should_RoundTripCorrectly(string level)
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, level, true);
        var doc = JsonNode.Parse(json)!.AsObject();
        doc["log_level"]!.GetValue<string>().Should().Be(level);
    }

    [Fact]
    public void Serialise_Should_ProducePrettyPrintedOutput()
    {
        string json = SettingsSerialiser.Serialise(false, true, 500, "info", true);
        // Pretty-printed JSON should have newlines
        json.Should().Contain("\n");
    }
}
