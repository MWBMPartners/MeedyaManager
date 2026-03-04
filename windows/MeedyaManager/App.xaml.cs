// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)

using Microsoft.UI.Xaml;

namespace MeedyaManager;

/// <summary>
/// WinUI 3 application entry point.
/// Creates and activates the main window on launch.
/// </summary>
public partial class App : Application
{
    /// <summary>
    /// Reference to the main application window.
    /// </summary>
    private Window? _mainWindow;

    /// <summary>
    /// Initializes the application and its XAML components.
    /// </summary>
    public App()
    {
        // Initialize XAML component tree defined in App.xaml
        this.InitializeComponent();
    }

    /// <summary>
    /// Invoked when the application is launched.
    /// Creates and activates the main window.
    /// </summary>
    /// <param name="args">Launch activation arguments.</param>
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Create the main application window
        _mainWindow = new MainWindow();

        // Activate (show) the window
        _mainWindow.Activate();
    }
}
