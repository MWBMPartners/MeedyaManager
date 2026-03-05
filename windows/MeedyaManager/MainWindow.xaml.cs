// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Main Window code-behind (WinUI 3)
//
// Configures the NavigationView and routes selection changes to the
// correct page via ContentFrame.Navigate().
// Applies Mica backdrop for the translucent Windows 11 material effect.
// Runs a background update check on startup (M8) and shows UpdateInfoBar
// when a newer release is available on GitHub.

using System;
using System.Threading.Tasks;
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

        // Kick off a background update check (M8).
        // Uses Task.Run so the UI is never blocked. The InfoBar is made visible
        // on the UI thread via DispatcherQueue when an update is found.
        _ = CheckForUpdatesAsync();
    }

    /// <summary>
    /// Performs an asynchronous update check against the GitHub Releases API
    /// (via mm-update P/Invoke, wired in M9+). For M8 this is a stub that
    /// always reports no update — it exercises the UI path without a network call.
    /// </summary>
    private async Task CheckForUpdatesAsync()
    {
        // Simulate the latency of a network call so the UI stays responsive
        await Task.Delay(millisecondsDelay: 2000).ConfigureAwait(false);

        // M8 stub: production code will call mm_update_check() via P/Invoke
        // and compare the returned version to the assembly's own version string.
        bool updateAvailable = false;   // always false until M9 wires the FFI
        string latestVersion  = string.Empty;

        if (updateAvailable)
        {
            // Marshal back to the UI thread before touching XAML elements
            this.DispatcherQueue.TryEnqueue(() =>
            {
                UpdateInfoBar.Message =
                    $"MeedyaManager {latestVersion} is available. Download it from GitHub.";
                UpdateInfoBar.IsOpen = true;
            });
        }
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
            NavigationViewItem { Tag: "Cloud"    } => typeof(CloudPage),
            NavigationViewItem { Tag: "Export"   } => typeof(ExportPage),
            NavigationViewItem { Tag: "Server"   } => typeof(ServerPage),
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
