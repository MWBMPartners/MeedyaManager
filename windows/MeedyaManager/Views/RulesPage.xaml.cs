// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rules Page code-behind (WinUI 3)
//
// Wires the template validator and tag pill buttons.
// Tag pills insert "<TagName>" at the current cursor position.

using System.Collections.Generic;
using MeedyaManager.Interop;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MeedyaManager.Views;

/// <summary>
/// Rules page (M6): full template builder with live validator, live preview,
/// sample tag editor, and tag pill reference grid.
/// </summary>
public sealed partial class RulesPage : Page
{
    // Sample tags used to compute the live preview
    private static readonly Dictionary<string, string> SampleTags = new()
    {
        ["Artist"] = "Pink Floyd",
        ["Album"]  = "The Wall",
        ["Title"]  = "Comfortably Numb",
        ["Year"]   = "1979",
    };

    public RulesPage()
    {
        this.InitializeComponent();

        // Populate tag pills from the known-tag registry
        IReadOnlyList<string> tags = MmCore.Instance.ListKnownTags();
        var pillItems = new List<string>();
        foreach (string tag in tags)
            pillItems.Add($"<{tag}>");

        TagPills.ItemsSource = pillItems;

        // Trigger initial validation with the default (empty) template
        UpdatePreview(string.Empty);
    }

    // ── Template validator ──────────────────────────────────────────────────

    /// <summary>Validates the template on every keystroke and updates the preview.</summary>
    private void TemplateEntry_TextChanged(object sender, TextChangedEventArgs e)
    {
        string template = TemplateEntry.Text;
        UpdatePreview(template);
    }

    /// <summary>Validates <paramref name="template"/> and recomputes the live preview.</summary>
    private void UpdatePreview(string template)
    {
        string trimmed = template.Trim();

        if (string.IsNullOrEmpty(trimmed))
        {
            ValidationBar.IsOpen = false;
            PreviewText.Text = "Enter a template above";
            PreviewText.Foreground = (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["TextFillColorSecondaryBrush"];
            return;
        }

        var (isValid, message) = MmCore.Instance.ValidateTemplate(trimmed);

        if (isValid)
        {
            ValidationBar.Severity = InfoBarSeverity.Success;
            ValidationBar.Message  = "Valid template";
            ValidationBar.IsOpen   = true;

            // Compute preview by substituting sample tags (case-insensitive)
            string result = trimmed;
            foreach (var (key, value) in SampleTags)
            {
                result = System.Text.RegularExpressions.Regex.Replace(
                    result,
                    System.Text.RegularExpressions.Regex.Escape($"<{key}>"),
                    value,
                    System.Text.RegularExpressions.RegexOptions.IgnoreCase
                );
            }
            PreviewText.Text = result;
            PreviewText.Foreground = (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["TextFillColorPrimaryBrush"];
        }
        else
        {
            ValidationBar.Severity = InfoBarSeverity.Error;
            ValidationBar.Message  = message;
            ValidationBar.IsOpen   = true;
            PreviewText.Text = string.Empty;
        }
    }

    // ── Tag pills ───────────────────────────────────────────────────────────

    /// <summary>
    /// Inserts the clicked tag text at the current cursor position in the template entry.
    /// </summary>
    private void TagPill_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Content: string tagText })
        {
            // WinUI 3 TextBox: insert at SelectionStart
            int pos = TemplateEntry.SelectionStart;
            string current = TemplateEntry.Text;
            TemplateEntry.Text = current.Insert(pos, tagText);
            // Move cursor to after the inserted tag
            TemplateEntry.SelectionStart = pos + tagText.Length;
        }
    }
}
