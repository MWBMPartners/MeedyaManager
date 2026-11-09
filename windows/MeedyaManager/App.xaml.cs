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

        // Tell the user plainly when this build has no engine at all, before
        // the pre-release check below — a build with no engine can also be
        // a non-pre-release build (e.g. a stable version tag with the DLL
        // missing from packaging), so this must not be folded into that
        // check (issue #222).
        ShowEngineMissingNoticeAsync();

        // Check for pre-release version and show warning if applicable
        CheckPreReleaseAndEnableTestMode();
    }

    // -----------------------------------------------------------------
    // Engine-missing notice (issue #222)
    // -----------------------------------------------------------------

    /// <summary>
    /// Shows a one-time dialog when mm_ffi.dll was not found beside the
    /// executable, so the user knows scanning, renaming and tag saving are
    /// all unavailable rather than discovering it silently, page by page.
    /// </summary>
    private async void ShowEngineMissingNoticeAsync()
    {
        if (MmCore.IsEngineAvailable)
            return;

        // Wait briefly for the XAML tree to finish loading before showing a
        // dialog, matching the pattern used by CheckPreReleaseAndEnableTestMode
        // below (a XamlRoot is not guaranteed to exist immediately after
        // Activate returns).
        if (MainWindow?.Content is not FrameworkElement rootElement)
            return;

        await System.Threading.Tasks.Task.Delay(200);

        var dialog = new ContentDialog
        {
            Title = "Running without the engine",
            Content = "This build of MeedyaManager does not include mm_ffi.dll, the " +
                      "MeedyaManager engine. Scanning, renaming and saving tags are all " +
                      "unavailable until a build with the engine is installed. " +
                      "(issue #222)",
            CloseButtonText = "OK",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = rootElement.XamlRoot,
        };

        await dialog.ShowAsync();
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
            // Corrected wording (issue #222 follow-up): the previous text
            // promised that saved tag edits would be diverted to a
            // safety-copy file elsewhere, using MeedyaManager's copy-file
            // naming pattern. That was false on Windows — Test Mode here is
            // only a flag (see MmCore.SetTestMode's comment: there is no
            // P/Invoke call yet), and neither MetadataPage nor ScanPage
            // checked it before MetadataPage's Save guard was added. A
            // tester who believed the old sentence and saved tags was
            // rewriting the real file.
            Content = $"You are running MeedyaManager {version}.\n\n" +
                      "This is a pre-release build. Test Mode has been automatically " +
                      "enabled to protect your files. While Test Mode is on, " +
                      "MeedyaManager will not rename files or save tag changes. In this " +
                      "Windows build, Test Mode is a safety switch only; it does not make " +
                      "copies.\n\n" +
                      "You can disable Test Mode at any time in Settings.",
            CloseButtonText = "OK",
            DefaultButton = ContentDialogButton.Close,
            XamlRoot = rootElement.XamlRoot,
        };

        await dialog.ShowAsync();
    }
}
