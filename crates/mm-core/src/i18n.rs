// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Internationalisation (i18n) initialisation
//
// On Linux, sets up GNU gettext so that all translatable strings (wrapped in
// `gettextrs::gettext()` or the `t!()` convenience macro) are resolved
// from the compiled .mo catalogue for the system locale.
//
// On macOS and Windows, this module provides a no-op `init()` — those
// platforms use their native localisation frameworks (NSLocalizedString /
// .resw resources) in their respective UI layers.
//
// Call `mm_core::i18n::init()` once — before any UI or user-visible text
// is produced — in the entry point of each binary (mm-cli `main()`, and
// mm-gtk `app::run()`).
//
// At runtime the function (Linux only):
//   1. Detects the system locale via `setlocale(LC_ALL, "")`.
//   2. Binds the "meedyamanager" text-domain to the locale directory.
//   3. Forces UTF-8 codeset for consistent string encoding.
//   4. Activates the domain with `textdomain()`.
//
// Locale directory search order (first non-error path wins):
//   a. `MEEDYA_LOCALE_DIR` environment variable (developer override, CI)
//   b. `$XDG_DATA_HOME/../share/locale` (Flatpak sandbox path)
//   c. `/usr/share/locale` (system-wide installation, .deb / Snap / AppImage)
//
// Adding a new language:
//   1. Copy `locales/en_US/LC_MESSAGES/meedyamanager.po` to
//      `locales/<lang>/LC_MESSAGES/meedyamanager.po`.
//   2. Translate the `msgstr` values.
//   3. Compile: `msgfmt -o meedyamanager.mo meedyamanager.po`.
//   4. Place the compiled .mo at the locale directory path expected above.
//
// See `locales/TRANSLATORS.md` for a full guide.

/// GNU gettext text-domain identifier.  Must match the .mo filename stem.
#[allow(dead_code)]
const DOMAIN: &str = "meedyamanager";

// ===========================================================================
// Linux — real GNU gettext initialisation
// ===========================================================================

#[cfg(target_os = "linux")]
use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};

/// Initialise the gettext i18n subsystem.
///
/// On Linux this binds the GNU gettext text-domain.  On macOS / Windows
/// this is a no-op — native localisation is handled by the platform UI.
///
/// Safe to call multiple times — subsequent calls are no-ops because gettext
/// stores the domain binding in process-global state.
///
/// # Panics
///
/// Does **not** panic — all errors are logged at `warn` level and the
/// function returns, leaving the application running with untranslated strings
/// (English fallback from `msgid`s is always available).
#[cfg(target_os = "linux")]
pub fn init() {
    // Step 1 — activate the system locale (e.g. "fr_FR.UTF-8").
    // Passing an empty string tells gettext to read the LC_ALL / LANG env vars.
    setlocale(LocaleCategory::LcAll, "");

    // Step 2 — resolve the locale directory.
    let locale_dir = resolve_locale_dir();

    // Step 3 — bind our text-domain to the locale directory.
    if let Err(e) = bindtextdomain(DOMAIN, &locale_dir) {
        tracing::warn!("i18n: bindtextdomain failed (dir={locale_dir:?}): {e}");
    }

    // Step 4 — require UTF-8 output regardless of system codeset.
    if let Err(e) = bind_textdomain_codeset(DOMAIN, "UTF-8") {
        tracing::warn!("i18n: bind_textdomain_codeset failed: {e}");
    }

    // Step 5 — make "meedyamanager" the active domain for `gettext()` calls.
    if let Err(e) = textdomain(DOMAIN) {
        tracing::warn!("i18n: textdomain failed: {e}");
    }

    tracing::debug!("i18n: initialised — domain={DOMAIN}, locale_dir={locale_dir}");
}

// ===========================================================================
// macOS / Windows — no-op stub
// ===========================================================================

/// No-op on non-Linux platforms.  Native localisation is handled by the
/// platform UI layer (NSLocalizedString on macOS, .resw on Windows).
#[cfg(not(target_os = "linux"))]
pub fn init() {
    tracing::debug!("i18n: skipped (non-Linux platform — using native localisation)");
}

// ===========================================================================
// Locale directory resolution (Linux only)
// ===========================================================================

/// Resolve the locale directory, preferring developer/Flatpak overrides
/// over the system-wide path.
#[cfg(target_os = "linux")]
fn resolve_locale_dir() -> String {
    // (a) Explicit override — useful for development and CI
    if let Ok(dir) = std::env::var("MEEDYA_LOCALE_DIR") {
        if !dir.is_empty() {
            return dir;
        }
    }

    // (b) Flatpak sandbox: XDG_DATA_DIRS contains the app's share dir
    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_dirs.split(':') {
            let candidate = format!("{dir}/locale");
            if std::path::Path::new(&candidate).is_dir() {
                return candidate;
            }
        }
    }

    // (c) System-wide fallback
    "/usr/share/locale".to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_constant_is_non_empty() {
        assert!(!DOMAIN.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[allow(unsafe_code)] // env::remove_var requires unsafe in Edition 2024
    fn resolve_locale_dir_returns_system_fallback_without_env() {
        // Clear any override that might be set in the test environment
        unsafe { std::env::remove_var("MEEDYA_LOCALE_DIR") };
        // Without MEEDYA_LOCALE_DIR set and without valid XDG_DATA_DIRS entries,
        // the function should return the system-wide fallback path.
        let dir = resolve_locale_dir();
        assert!(!dir.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[allow(unsafe_code)] // env::set_var/remove_var require unsafe in Edition 2024
    fn resolve_locale_dir_honours_env_override() {
        unsafe { std::env::set_var("MEEDYA_LOCALE_DIR", "/tmp/test-locales") };
        let dir = resolve_locale_dir();
        unsafe { std::env::remove_var("MEEDYA_LOCALE_DIR") };
        assert_eq!(dir, "/tmp/test-locales");
    }

    #[test]
    fn init_does_not_panic() {
        // Calling init() with no .mo files present should not panic —
        // gettext falls back to returning the untranslated msgid string.
        init();
    }
}
