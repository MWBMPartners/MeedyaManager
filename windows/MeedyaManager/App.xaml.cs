// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — WinUI 3 Application Entry Point
//
// Creates and activates the main window on launch.  On first launch of a
// pre-release build (version string contains '-'), shows a warning dialog
// and auto-enables test mode so that file operations are journalled and
// safely reversible.

using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using MeedyaManager.Interop;

namespace MeedyaManager;

/// <summary>
/// WinUI 3 application entry point.
/// Creates and activates the main window on launch.
/// Detects pre-release builds and auto-enables test mode with a notice.
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
    /// After activation, checks for pre-release builds and shows a notice.
    /// </summary>
    /// <param name="args">Launch activation arguments.</param>
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // Create the main application window and store for cross-class HWND access
        MainWindow = new MainWindow();

        // Activate (show) the window
        MainWindow.Activate();

        // Check for pre-release version and show warning if applicable
        CheckPreReleaseAndEnableTestMode();
    }

    // -----------------------------------------------------------------
    // Pre-release detection
    // -----------------------------------------------------------------

    /// <summary>
    /// Detects whether this is a pre-release build by checking whether the
    /// version string contains a hyphen (e.g. "2.0.0-alpha.5").  If so,
    /// auto-enables test mode and shows a one-time warning dialog.
    /// </summary>
    private async void CheckPreReleaseAndEnableTestMode()
    {
        // Retrieve the version string from MmCore (falls back to stub version)
        string version = MmCore.Instance.Version();

        // Pre-release versions contain a hyphen per semver (e.g. "2.0.0-beta.1")
        if (!version.Contains('-'))
            return;

        // Skip if test mode is already enabled (user or previous session)
        if (MmCore.Instance.TestModeEnabled())
            return;

        // Auto-enable test mode so file operations are journalled
        MmCore.Instance.SetTestMode(true);

        // Wait briefly for the XAML tree to finish loading before showing a dialog
        if (MainWindow?.Content is not FrameworkElement rootElement)
            return;

        // Ensure the XamlRoot is available (may need a layout pass)
        await System.Threading.Tasks.Task.Delay(200);

        // Build and show the pre-release warning dialog
        var dialog = new ContentDialog
        {
            Title = "Pre-Release Build Detected",
            Content = $"You are running MeedyaManager {version}.\n\n" +
                      "This is a pre-release build. Test Mode has been automatically " +
                      "enabled to protect your files. All file operations (renames, " +
                      "tag writes) will be journalled and can be committed or reverted " +
                      "from the Settings page.\n\n" +
                      "You can disable Test Mode at any time in Settings.",
            CloseButtonText = "OK",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = rootElement.XamlRoot,
        };

        await dialog.ShowAsync();
    }
}
