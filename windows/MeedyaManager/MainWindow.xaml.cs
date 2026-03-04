// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)

using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace MeedyaManager;

/// <summary>
/// Main application window with NavigationView and Mica backdrop.
/// Hosts the top-level navigation between Library, Rules, Metadata, Lookup, and Settings.
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

        // Set the window title
        this.Title = "MeedyaManager";

        // Apply Mica backdrop for the Windows 11 translucent material effect.
        // Falls back gracefully on unsupported systems.
        this.SystemBackdrop = new MicaBackdrop();

        // Select the first navigation item (Library) by default
        NavView.SelectedItem = NavView.MenuItems[0];
    }

    /// <summary>
    /// Handles NavigationView selection changes.
    /// Routes navigation to the appropriate page based on the selected item's Tag.
    /// </summary>
    /// <param name="sender">The NavigationView that raised the event.</param>
    /// <param name="args">Event data including the selected item.</param>
    private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        // Get the selected NavigationViewItem
        if (args.SelectedItem is NavigationViewItem selectedItem)
        {
            // Read the Tag property to determine which section was selected
            string? tag = selectedItem.Tag as string;

            // TODO: Navigate ContentFrame to the corresponding page once view pages are implemented.
            // Example: ContentFrame.Navigate(typeof(Views.LibraryPage));
            System.Diagnostics.Debug.WriteLine($"Navigation selected: {tag}");
        }
    }
}
