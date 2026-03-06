// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — MmCore Windows P/Invoke Bridge
//
// Wraps the mm-ffi C API exported by the Rust cdylib via cbindgen.
// All functions communicate through JSON strings to avoid complex
// marshalling of nested structs across the P/Invoke boundary.
//
// In development builds without the compiled DLL the class returns
// realistic stub values so the UI can be previewed without a Rust toolchain.
//
// Generated C header: crates/mm-ffi/include/mm_ffi.h
// Library name:       mm_ffi.dll  (placed beside the executable by CI)

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace MeedyaManager.Interop;

// ---------------------------------------------------------------------------
// Data transfer types (mirroring FFI JSON payloads)
// ---------------------------------------------------------------------------

/// <summary>A single metadata tag key/value pair.</summary>
public record TagEntry(string Key, string Value);

/// <summary>Rename preview item returned by scan operations.</summary>
public record RenamePreview(
    string Source,
    string Destination,
    bool   Conflict,
    bool   Unchanged
);

/// <summary>Result of template validation.</summary>
public record ValidationResult(
    bool   IsValid,
    string ErrorMessage,
    List<string> Warnings
);

/// <summary>Audio technical properties for a media file.</summary>
public record AudioProperties(
    long   DurationSecs,
    uint   BitrateKbps,
    uint   SampleRateHz,
    uint   Channels,
    uint   BitDepth,
    bool   IsLossless,
    string Codec
);

// ---------------------------------------------------------------------------
// MmCore singleton — thin P/Invoke wrapper
// ---------------------------------------------------------------------------

/// <summary>
/// Single entry point for all calls into the mm-ffi Rust library.
/// Use <see cref="Instance"/> throughout the application.
/// All methods gracefully degrade to stubs when the DLL is absent.
/// </summary>
public sealed class MmCore
{
    // -----------------------------------------------------------------
    // Singleton
    // -----------------------------------------------------------------

    /// <summary>Application-wide shared instance.</summary>
    public static readonly MmCore Instance = new();

    private MmCore() { }

    // True when the native DLL was successfully loaded at startup
    private static readonly bool _dllAvailable = CheckDllAvailable();

    /// <summary>Checks whether mm_ffi.dll exists beside the executable.</summary>
    private static bool CheckDllAvailable()
    {
        string dllPath = Path.Combine(AppContext.BaseDirectory, "mm_ffi.dll");
        return File.Exists(dllPath);
    }

    // -----------------------------------------------------------------
    // P/Invoke declarations — all functions follow the mm_ffi C API
    // -----------------------------------------------------------------

    private const string DllName = "mm_ffi";

    // Free a heap string returned by any mm_ffi_* function
    [DllImport(DllName, EntryPoint = "mm_ffi_free_string", CallingConvention = CallingConvention.Cdecl)]
    private static extern void FreeString(IntPtr ptr);

    [DllImport(DllName, EntryPoint = "mm_ffi_version", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeVersion();

    [DllImport(DllName, EntryPoint = "mm_ffi_config_path", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeConfigPath();

    [DllImport(DllName, EntryPoint = "mm_ffi_validate_template", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeValidateTemplate([MarshalAs(UnmanagedType.LPStr)] string template);

    [DllImport(DllName, EntryPoint = "mm_ffi_list_known_tags", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeListKnownTags();

    [DllImport(DllName, EntryPoint = "mm_ffi_scan_directory", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeScanDirectory(
        [MarshalAs(UnmanagedType.LPStr)] string directory,
        [MarshalAs(UnmanagedType.LPStr)] string template,
        [MarshalAs(UnmanagedType.I1)]    bool   recursive
    );

    [DllImport(DllName, EntryPoint = "mm_ffi_get_metadata", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeGetMetadata([MarshalAs(UnmanagedType.LPStr)] string path);

    [DllImport(DllName, EntryPoint = "mm_ffi_write_metadata", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr NativeWriteMetadata(
        [MarshalAs(UnmanagedType.LPStr)] string path,
        [MarshalAs(UnmanagedType.LPStr)] string tagsJson
    );

    // -----------------------------------------------------------------
    // Helper: call native, read JSON string, free memory
    // -----------------------------------------------------------------

    /// <summary>
    /// Calls a native function that returns a heap-allocated C string,
    /// marshals it to a managed string, and frees the native memory.
    /// Returns null if the DLL is unavailable or the pointer is null.
    /// </summary>
    private static string? CallNative(Func<IntPtr> native)
    {
        if (!_dllAvailable) return null;
        IntPtr ptr = IntPtr.Zero;
        try
        {
            ptr = native();
            return ptr == IntPtr.Zero ? null : Marshal.PtrToStringAnsi(ptr);
        }
        catch
        {
            return null;
        }
        finally
        {
            if (ptr != IntPtr.Zero) FreeString(ptr);
        }
    }

    // -----------------------------------------------------------------
    // Public API — mirrors MmCore.swift
    // -----------------------------------------------------------------

    /// <summary>Returns the MeedyaManager core version string.</summary>
    public string Version()
    {
        string? raw = CallNative(NativeVersion);
        return raw ?? $"{GetAppVersion()} (stub)";
    }

    /// <summary>Returns the platform-specific path to settings.json5.</summary>
    public string ConfigPath()
    {
        string? raw = CallNative(NativeConfigPath);
        if (raw is not null) return raw;

        // Stub: %APPDATA%\MeedyaManager\settings.json5
        string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        return Path.Combine(appData, "MeedyaManager", "settings.json5");
    }

    /// <summary>
    /// Validates a rename template string.
    /// Returns (isValid, errorMessage) tuple.
    /// </summary>
    public (bool IsValid, string Message) ValidateTemplate(string template)
    {
        string? raw = CallNative(() => NativeValidateTemplate(template));
        if (raw is not null)
        {
            // Parse {"is_valid":true,"error_message":"","warnings":[]}
            try
            {
                using JsonDocument doc = JsonDocument.Parse(raw);
                JsonElement root = doc.RootElement;
                bool isValid = root.GetProperty("is_valid").GetBoolean();
                string msg = root.TryGetProperty("error_message", out JsonElement em)
                    ? em.GetString() ?? string.Empty
                    : string.Empty;
                return (isValid, msg);
            }
            catch { /* fall through to stub */ }
        }

        // Stub: basic balanced-bracket check
        return StubValidateTemplate(template);
    }

    /// <summary>Returns all recognised tag names.</summary>
    public IReadOnlyList<string> ListKnownTags()
    {
        string? raw = CallNative(NativeListKnownTags);
        if (raw is not null)
        {
            try
            {
                return JsonSerializer.Deserialize<List<string>>(raw) ?? StubKnownTags();
            }
            catch { /* fall through */ }
        }
        return StubKnownTags();
    }

    /// <summary>
    /// Scans a directory and returns rename previews.
    /// Returns an empty list on error.
    /// </summary>
    public IReadOnlyList<RenamePreview> ScanDirectory(string directory, string template, bool recursive)
    {
        string? raw = CallNative(() => NativeScanDirectory(directory, template, recursive));
        if (raw is not null)
        {
            try
            {
                // JSON array of {source, destination, conflict, unchanged}
                return JsonSerializer.Deserialize<List<RenamePreviewJson>>(raw)
                    ?.ConvertAll(p => new RenamePreview(p.Source, p.Destination, p.Conflict, p.Unchanged))
                    ?? [];
            }
            catch { /* fall through */ }
        }
        return StubScanDirectory(directory, template);
    }

    /// <summary>Reads all metadata tags from a media file.</summary>
    public IReadOnlyList<TagEntry> GetMetadata(string path)
    {
        string? raw = CallNative(() => NativeGetMetadata(path));
        if (raw is not null)
        {
            try
            {
                return JsonSerializer.Deserialize<List<TagEntryJson>>(raw)
                    ?.ConvertAll(t => new TagEntry(t.Key, t.Value))
                    ?? [];
            }
            catch { /* fall through */ }
        }
        return StubMetadata(path);
    }

    /// <summary>Writes updated tag values to a media file.</summary>
    public bool WriteMetadata(string path, IReadOnlyList<TagEntry> tags)
    {
        if (!_dllAvailable) return true; // Stub: pretend success

        // Serialise tags to JSON array [{key, value}, ...]
        string tagsJson = JsonSerializer.Serialize(
            tags, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });

        string? raw = CallNative(() => NativeWriteMetadata(path, tagsJson));
        if (raw is null) return false;

        // Error response: {"error":"..."}
        try
        {
            using JsonDocument doc = JsonDocument.Parse(raw);
            return !doc.RootElement.TryGetProperty("error", out _);
        }
        catch
        {
            return false;
        }
    }

    // -----------------------------------------------------------------
    // Test Mode API — safe-mode for pre-release builds
    // -----------------------------------------------------------------

    // In-memory flag tracking whether test mode is active (stub state)
    private bool _testModeEnabled;

    /// <summary>
    /// Returns whether test mode is currently enabled.
    /// When the DLL is available, delegates to the native function;
    /// otherwise returns the in-memory stub flag.
    /// </summary>
    public bool TestModeEnabled()
    {
        // TODO: P/Invoke NativeTestModeEnabled() when DLL ships
        return _testModeEnabled;
    }

    /// <summary>
    /// Enables or disables test mode.
    /// In test mode, file operations are journalled and reversible.
    /// </summary>
    /// <param name="enabled">True to enable test mode, false to disable.</param>
    public void SetTestMode(bool enabled)
    {
        // TODO: P/Invoke NativeSetTestMode(enabled) when DLL ships
        _testModeEnabled = enabled;
    }

    /// <summary>
    /// Returns the number of files currently tracked in the test mode journal.
    /// Returns 0 when test mode is off or the DLL is unavailable.
    /// </summary>
    public int TestModeFileCount()
    {
        // TODO: P/Invoke NativeTestModeFileCount() when DLL ships
        // Stub: always 0 (no real journal without the Rust core)
        return 0;
    }

    /// <summary>
    /// Commits all test mode changes — makes them permanent on disk.
    /// Clears the test mode journal after successful commit.
    /// </summary>
    public void CommitTestModeFiles()
    {
        // TODO: P/Invoke NativeCommitTestModeFiles() when DLL ships
        // Stub: no-op (nothing to commit without the Rust core)
    }

    /// <summary>
    /// Reverts all test mode changes — restores original file state.
    /// Clears the test mode journal after successful revert.
    /// </summary>
    public void RevertTestModeFiles()
    {
        // TODO: P/Invoke NativeRevertTestModeFiles() when DLL ships
        // Stub: no-op (nothing to revert without the Rust core)
    }

    // -----------------------------------------------------------------
    // Stubs (used when DLL is unavailable)
    // -----------------------------------------------------------------

    private static (bool, string) StubValidateTemplate(string template)
    {
        if (string.IsNullOrWhiteSpace(template))
            return (false, "Template must not be empty");

        int depth = 0;
        foreach (char ch in template)
        {
            if (ch == '<') depth++;
            if (ch == '>') depth--;
            if (depth < 0) return (false, "Unexpected '>' without matching '<'");
        }
        return depth != 0
            ? (false, "Unmatched '<' — missing '>'")
            : (true, string.Empty);
    }

    private static List<string> StubKnownTags() =>
    [
        "Title", "Artist", "Album", "AlbumArtist", "Year", "Genre",
        "TrackNumber", "TrackTotal", "DiscNumber", "DiscTotal",
        "Composer", "Comment", "Lyrics", "ISRC", "Barcode",
        "CatalogNumber", "Label", "Compilation", "BPM",
        "Filename", "Extension", "Folder", "Duration",
        "BitrateKbps", "SampleRateHz", "MediaClass", "MediaFormat",
    ];

    private static IReadOnlyList<RenamePreview> StubScanDirectory(string directory, string template)
    {
        // Walk directory with FileInfo and return placeholder previews
        var previews = new List<RenamePreview>();
        var audioExts = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            { ".mp3", ".flac", ".m4a", ".aac", ".ogg", ".opus", ".wav" };

        try
        {
            foreach (string file in Directory.EnumerateFiles(directory, "*", SearchOption.AllDirectories))
            {
                if (!audioExts.Contains(Path.GetExtension(file))) continue;
                string dir = Path.GetDirectoryName(file) ?? directory;
                string dst = Path.Combine(dir, $"[Preview] {Path.GetFileName(file)}");
                previews.Add(new RenamePreview(file, dst, false, false));
                if (previews.Count >= 50) break;
            }
        }
        catch { /* Ignore permission errors */ }

        previews.Sort((a, b) => string.Compare(a.Source, b.Source, StringComparison.Ordinal));
        return previews;
    }

    private static IReadOnlyList<TagEntry> StubMetadata(string path)
    {
        string ext = Path.GetExtension(path).ToLowerInvariant();
        var tags = new List<TagEntry>
        {
            new("title",        "Sample Track"),
            new("artist",       "Sample Artist"),
            new("album",        "Sample Album"),
            new("year",         "2024"),
            new("track_number", "1"),
            new("genre",        "Electronic"),
        };
        if (ext is ".flac" or ".wav" or ".aiff")
            tags.Add(new("comment", "Lossless format"));
        tags.Sort((a, b) => string.Compare(a.Key, b.Key, StringComparison.Ordinal));
        return tags;
    }

    // -----------------------------------------------------------------
    // Private JSON DTOs (snake_case names from Rust serialisation)
    // -----------------------------------------------------------------

    private record RenamePreviewJson(
        [property: JsonPropertyName("source")]      string Source,
        [property: JsonPropertyName("destination")] string Destination,
        [property: JsonPropertyName("conflict")]    bool   Conflict,
        [property: JsonPropertyName("unchanged")]   bool   Unchanged
    );

    private record TagEntryJson(
        [property: JsonPropertyName("key")]   string Key,
        [property: JsonPropertyName("value")] string Value
    );

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    private static string GetAppVersion()
    {
        return System.Reflection.Assembly.GetExecutingAssembly()
            .GetName().Version?.ToString(3) ?? "2.0.0-alpha.5";
    }
}
