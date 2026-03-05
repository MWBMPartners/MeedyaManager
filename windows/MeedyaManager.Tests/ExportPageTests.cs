// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — ExportPage unit tests (Windows / xUnit, M9)
//
// Tests pure logic extracted from ExportPage.xaml.cs without requiring a
// WinUI runtime (no XAML components are constructed — only standalone helpers).

using System;
using Xunit;

namespace MeedyaManager.Tests;

// ── Backend hint map replicated for testing ──────────────────────────────────

/// <summary>Pure-logic replica of the backend hint table from ExportPage.</summary>
internal static class ExportBackendHints
{
    public static readonly (string Tag, string Hint)[] All =
    [
        ("sqlite",   "sqlite:///C:/Users/You/library.db"),
        ("mysql",    "mysql://user:pass@localhost/meedya"),
        ("mariadb",  "mysql://user:pass@localhost/meedya"),
        ("postgres", "postgres://user:pass@localhost/meedya"),
        ("mssql",    "server=tcp:host,1433;database=meedya;user=sa;password=P"),
    ];

    public static string? GetHint(string tag)
    {
        foreach (var (t, h) in All)
            if (t == tag) return h;
        return null;
    }
}

// ── DSN detection helper replicated for testing ──────────────────────────────

internal static class ExportDsnHelper
{
    /// <summary>Returns true when the DSN looks non-empty and plausible.</summary>
    public static bool IsValidDsn(string? dsn)
        => !string.IsNullOrWhiteSpace(dsn);

    /// <summary>Redacts credentials from a DSN for display.</summary>
    public static string RedactDsn(string dsn)
    {
        if (dsn.StartsWith("sqlite", StringComparison.OrdinalIgnoreCase) ||
            dsn.StartsWith("server=", StringComparison.OrdinalIgnoreCase))
        {
            return dsn[..Math.Min(40, dsn.Length)] + "…";
        }
        var at = dsn.IndexOf('@');
        var scheme = dsn.IndexOf("://", StringComparison.Ordinal);
        if (at > 0 && scheme > 0)
        {
            return dsn[..(scheme + 3)] + "***@" + dsn[(at + 1)..];
        }
        return dsn[..Math.Min(30, dsn.Length)] + "…";
    }
}

// ── Export stats replica ─────────────────────────────────────────────────────

internal record ExportStatsReplica(long Inserted, long Updated, long Skipped, long Errors)
{
    public long Total     => Inserted + Updated + Skipped + Errors;
    public long Persisted => Inserted + Updated;
    public bool IsClean   => Errors == 0;
}

// ── Tests ────────────────────────────────────────────────────────────────────

public class ExportPageTests
{
    // --- Backend hints ---

    [Fact]
    public void BackendHints_HasFiveEntries()
        => Assert.Equal(5, ExportBackendHints.All.Length);

    [Fact]
    public void BackendHints_SqliteIsFirst()
        => Assert.Equal("sqlite", ExportBackendHints.All[0].Tag);

    [Fact]
    public void BackendHints_AllTagsNonEmpty()
    {
        foreach (var (tag, _) in ExportBackendHints.All)
            Assert.False(string.IsNullOrEmpty(tag));
    }

    [Fact]
    public void BackendHints_AllHintsNonEmpty()
    {
        foreach (var (_, hint) in ExportBackendHints.All)
            Assert.False(string.IsNullOrEmpty(hint));
    }

    [Fact]
    public void BackendHints_GetHint_SqliteReturnsPath()
    {
        var hint = ExportBackendHints.GetHint("sqlite");
        Assert.NotNull(hint);
        Assert.Contains("sqlite", hint, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void BackendHints_GetHint_UnknownReturnsNull()
        => Assert.Null(ExportBackendHints.GetHint("oracle"));

    // --- DSN validation ---

    [Fact]
    public void IsValidDsn_EmptyString_False()
        => Assert.False(ExportDsnHelper.IsValidDsn(""));

    [Fact]
    public void IsValidDsn_WhitespaceOnly_False()
        => Assert.False(ExportDsnHelper.IsValidDsn("   "));

    [Fact]
    public void IsValidDsn_ValidSqliteDsn_True()
        => Assert.True(ExportDsnHelper.IsValidDsn("sqlite:///library.db"));

    [Fact]
    public void IsValidDsn_ValidPostgresDsn_True()
        => Assert.True(ExportDsnHelper.IsValidDsn("postgres://u:p@h/db"));

    // --- DSN redaction ---

    [Fact]
    public void RedactDsn_HidesPassword()
    {
        var result = ExportDsnHelper.RedactDsn("postgres://admin:secret123@db.host/meedya");
        Assert.DoesNotContain("secret123", result);
        Assert.Contains("db.host", result);
    }

    [Fact]
    public void RedactDsn_SqliteTruncates()
    {
        var result = ExportDsnHelper.RedactDsn("sqlite:///very/long/path/to/library.db");
        Assert.EndsWith("…", result);
    }

    // --- ExportStats replica ---

    [Fact]
    public void ExportStats_TotalAndPersisted()
    {
        var s = new ExportStatsReplica(10, 5, 3, 2);
        Assert.Equal(20, s.Total);
        Assert.Equal(15, s.Persisted);
        Assert.False(s.IsClean);
    }

    [Fact]
    public void ExportStats_CleanRun()
    {
        var s = new ExportStatsReplica(7, 2, 0, 0);
        Assert.Equal(9, s.Total);
        Assert.True(s.IsClean);
    }
}
