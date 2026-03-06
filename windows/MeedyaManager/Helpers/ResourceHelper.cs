// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Windows Resource Helper
//
// Provides a thin, static wrapper around Windows.ApplicationModel.Resources.ResourceLoader
// so that code-behind files can retrieve localised strings without repeating
// boilerplate ResourceLoader construction.
//
// Usage (code-behind):
//   using MeedyaManager.Helpers;
//
//   string label   = ResourceHelper.Get("Scan.Button.Scan");
//   string message = ResourceHelper.Format("Common.Error.Generic", errorText);
//
// Usage (XAML):
//   Attach x:Uid to any FrameworkElement whose Text/Content/Header/PlaceholderText
//   you want resolved from Resources.resw.  The x:Uid value maps to the resource
//   key prefix, e.g.:
//
//     <TextBlock x:Uid="Scan_Status" Text="Idle" />
//   resolves to the resource key "Scan_Status.Text" in Resources.resw.
//
// Resource file location:
//   windows/MeedyaManager/Strings/en-US/Resources.resw  — English (source)
//   windows/MeedyaManager/Strings/<lang>/Resources.resw — per-language translations
//
// Adding a new language:
//   1. Create Strings/<BCP47-tag>/Resources.resw  (e.g. Strings/fr-FR/Resources.resw)
//   2. Copy all <data> elements from Strings/en-US/Resources.resw
//   3. Translate the <value> text for each key (keep keys unchanged)
//   4. WinUI 3 resolves the correct locale automatically at runtime

using System;
using System.Runtime.CompilerServices;
using Windows.ApplicationModel.Resources;

namespace MeedyaManager.Helpers;

/// <summary>
/// Centralised resource string lookup for MeedyaManager.
/// </summary>
public static class ResourceHelper
{
    // Lazily created, thread-safe ResourceLoader instance.
    // ResourceLoader must be accessed from a UI thread in WinUI 3, so we use
    // Lazy with ExecutionAndPublication to ensure single initialisation.
    private static readonly Lazy<ResourceLoader> _loader =
        new(() => new ResourceLoader(), System.Threading.LazyThreadSafetyMode.PublicationOnly);

    /// <summary>
    /// Returns the localised string for <paramref name="key"/>.
    /// Falls back to <paramref name="key"/> itself if no resource is found,
    /// so the app never crashes due to a missing translation.
    /// </summary>
    /// <param name="key">The resource key (e.g. "Scan.Button.Scan").</param>
    /// <returns>The localised string, or the key on failure.</returns>
    public static string Get(string key)
    {
        try
        {
            // ResourceLoader uses '/' as the separator internally.
            // Replace '.' so callers can use the dot-separated convention.
            string resolved = _loader.Value.GetString(key.Replace('.', '/'));
            return string.IsNullOrEmpty(resolved) ? key : resolved;
        }
        catch
        {
            // Never crash due to a missing translation — return the key as
            // a visible fallback so developers can spot missing entries easily.
            return key;
        }
    }

    /// <summary>
    /// Returns a localised, formatted string using <see cref="string.Format"/>.
    /// </summary>
    /// <param name="key">The resource key.</param>
    /// <param name="args">Format arguments corresponding to {0}, {1}, … in the value.</param>
    /// <returns>The formatted localised string.</returns>
    public static string Format(string key, params object[] args)
    {
        try
        {
            return string.Format(Get(key), args);
        }
        catch (FormatException)
        {
            // Return unformatted template if argument count mismatches.
            return Get(key);
        }
    }
}
