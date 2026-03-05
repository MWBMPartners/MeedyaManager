// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Main Window code-behind (WinUI 3)
//
// Configures the NavigationView and routes selection changes to the
// correct page via ContentFrame.Navigate().
// Applies Mica backdrop for the translucent Windows 11 material effect.

using System;
using MeedyaManager.Views;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MeedyaManager;

/// <summary>
/// Main application window.
/// Hosts a NavigationView with Mica backdrop and navigates between the
/// Library (Scan), Rules, Metadata, Lookup, and Settings pages.
/// </summary>
public sealed partial class MainWindow : Window
{
    /// <summary>
    /// Initializes the main window, applies Mica backdrop, and selects the default nav item.
    /// </summary>
    public MainWindow()
    {
        // Initialize XAML component tree defined in MainWindow.xaml
        this.InitializeComponent();

        // Human-readable window title shown in the title bar and taskbar
        this.Title = "MeedyaManager";

        // Apply Mica backdrop: translucent Windows 11 material effect.
        // Silently degrades on systems that do not support it (Windows 10, VM).
        this.SystemBackdrop = new MicaBackdrop();

        // Select the first navigation item (Library/Scan) by default
        NavView.SelectedItem = NavView.MenuItems[0];
    }

    /// <summary>
    /// Handles NavigationView selection changes.
    /// Routes navigation to the appropriate Page subclass based on the Tag.
    /// </summary>
    /// <param name="sender">The NavigationView that raised the event.</param>
    /// <param name="args">Event data including the selected item.</param>
    private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        // Determine which page type to navigate to based on the Tag
        Type? pageType = args.SelectedItem switch
        {
            // NavigationViewItem with Tag set in XAML
            NavigationViewItem { Tag: "Library"  } => typeof(ScanPage),
            NavigationViewItem { Tag: "Rules"    } => typeof(RulesPage),
            NavigationViewItem { Tag: "Metadata" } => typeof(MetadataPage),
            NavigationViewItem { Tag: "Lookup"   } => typeof(LookupPage),
            NavigationViewItem { Tag: "Settings" } => typeof(SettingsPage),
            _ => null,
        };

        // Navigate only if a valid page type was matched
        if (pageType is not null && ContentFrame.CurrentSourcePageType != pageType)
        {
            // Navigate with a default entrance animation (slide from right)
            ContentFrame.Navigate(pageType, null, new Microsoft.UI.Xaml.Media.Animation.EntranceNavigationTransitionInfo());
        }
    }
}
