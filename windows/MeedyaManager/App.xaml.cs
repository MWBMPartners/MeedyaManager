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
    /// Exposed publicly so that file/folder pickers can retrieve the HWND.
    /// </summary>
    public static Window? MainWindow { get; private set; }

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
        // Create the main application window and store for cross-class HWND access
        MainWindow = new MainWindow();

        // Activate (show) the window
        MainWindow.Activate();
    }
}
