// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — User-Agent String Builder
//
// All HTTP requests made by MeedyaManager (metadata providers, update checker,
// cloud storage) use a consistent, descriptive User-Agent header that identifies:
//
//   - Application name and version   ("MeedyaManager/1.2.0")
//   - Operating system and CPU arch  ("macOS; Apple Silicon")
//
// Format: `MeedyaManager/<version> (<platform>)`
//
// Examples:
//   MeedyaManager/1.2.0 (macOS; Apple Silicon)
//   MeedyaManager/1.2.0 (Windows; x64)
//   MeedyaManager/1.2.0 (Windows; ARM64)
//   MeedyaManager/1.2.0 (Linux; x86_64)
//   MeedyaManager/1.2.0 (Linux; ARM64)       ← includes Raspberry Pi 4/5 (64-bit)
//   MeedyaManager/1.2.0 (Linux; ARM)         ← includes Raspberry Pi OS 32-bit
//
// The platform string is resolved at compile time using `std::env::consts` so
// there is zero runtime overhead and the string is inlined into the binary.
//
// Some third-party APIs (notably MusicBrainz — see crates/mm-providers/src/
// musicbrainz.rs) require the User-Agent to also carry a contact address so
// the API operator can reach us if our traffic misbehaves. For those callers,
// use `build_user_agent_with_contact()` instead, which produces:
//
//   Format: `<build_user_agent()> ( <contact> )`
//
// Example:
//   MeedyaManager/1.2.0 (macOS; Apple Silicon) ( support@mwbmpartners.ltd )
//
// The contact segment defaults to a compiled-in support email + homepage URL
// (`DEFAULT_CONTACT_EMAIL` / `DEFAULT_CONTACT_URL`), but can be overridden at
// runtime by setting the `MUSICBRAINZ_CONTACT_EMAIL` environment variable —
// useful for self-hosters who want MusicBrainz to contact *them* directly
// rather than MWBM Partners Ltd.
//
// Public API:
//   build_user_agent() → String              — full UA string for this build
//   build_user_agent_with_contact() → String — UA string + contact segment
//   contact_string() → String                — just the contact segment
//   contact_from(Option<&str>) → String      — pure helper (unit-testable)

// ---------------------------------------------------------------------------
// Platform constants (compile-time, zero overhead)
// ---------------------------------------------------------------------------

/// Operating system identifier from the Rust standard library.
/// Values: "linux", "macos", "windows", "freebsd", etc.
const OS: &str = std::env::consts::OS;

/// CPU architecture identifier from the Rust standard library.
/// Values: "x86_64", "aarch64", "arm", "x86", "riscv64", etc.
const ARCH: &str = std::env::consts::ARCH;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the MeedyaManager User-Agent string for this platform and version.
///
/// The version is read from `CARGO_PKG_VERSION` at compile time, so it always
/// matches the workspace version in `Cargo.toml`.
///
/// # Examples
///
/// ```
/// let ua = mm_core::useragent::build_user_agent();
/// assert!(ua.starts_with("MeedyaManager/"));
/// assert!(ua.contains('('));
/// ```
pub fn build_user_agent() -> String {
    // Application name and version — version injected at compile time
    let version = env!("CARGO_PKG_VERSION");
    // Platform descriptor — OS + architecture detail
    let platform = platform_string();
    format!("MeedyaManager/{version} ({platform})")
}

// ---------------------------------------------------------------------------
// Contact-bearing User-Agent (for APIs requiring a contact address)
// ---------------------------------------------------------------------------

/// Default contact e-mail baked into the binary when no runtime override is
/// present. Used to populate the contact segment of `build_user_agent_with_contact()`.
pub const DEFAULT_CONTACT_EMAIL: &str = "support@mwbmpartners.ltd";

/// Default contact homepage URL baked into the binary when no runtime override
/// is present. Paired with `DEFAULT_CONTACT_EMAIL` in the default contact string.
pub const DEFAULT_CONTACT_URL: &str = "https://www.mwbmpartners.ltd";

/// Pure helper that resolves the contact segment from an optional environment
/// value. Kept free of `std::env` access so it is trivially unit-testable
/// without mutating process-wide state (edition 2024 makes `env::set_var`
/// `unsafe`, and tests must never touch it).
///
/// Behaviour:
///   - `Some(value)` where `value` is non-empty after trimming whitespace →
///     the trimmed value is used verbatim as the contact string.
///   - `None`, `Some("")`, or an all-whitespace value → falls back to the
///     compiled-in default `"<DEFAULT_CONTACT_EMAIL> <DEFAULT_CONTACT_URL>"`.
fn contact_from(env_value: Option<&str>) -> String {
    // Trim the candidate (if any) and treat an empty result as "not set".
    match env_value.map(str::trim) {
        // A real, non-empty override — use it exactly as provided (trimmed).
        Some(trimmed) if !trimmed.is_empty() => trimmed.to_owned(),
        // Absent, empty, or whitespace-only — fall back to the compiled default.
        _ => format!("{DEFAULT_CONTACT_EMAIL} {DEFAULT_CONTACT_URL}"),
    }
}

/// Resolve the contact string for this run, reading the `MUSICBRAINZ_CONTACT_EMAIL`
/// environment variable at call time and delegating the fallback logic to the
/// pure `contact_from()` helper.
///
/// This is a thin, side-effecting wrapper — all interesting logic lives in
/// `contact_from()` so it can be unit-tested without touching the environment.
pub fn contact_string() -> String {
    // Read the runtime override, if any. `std::env::var` returns `Err` when
    // the variable is unset, which we collapse to `None` via `.ok()`.
    let override_value = std::env::var("MUSICBRAINZ_CONTACT_EMAIL").ok();
    contact_from(override_value.as_deref())
}

/// Build the full contact-bearing User-Agent string used by providers that
/// require a way to reach the application operator (e.g. MusicBrainz).
///
/// Format: `<build_user_agent()> ( <contact_string()> )`
pub fn build_user_agent_with_contact() -> String {
    // Start from the standard UA string, then append the parenthesised
    // contact segment with a leading space, matching MusicBrainz's documented
    // convention of "AppName/Version ( contact-info )".
    format!("{} ( {} )", build_user_agent(), contact_string())
}

// ---------------------------------------------------------------------------
// Internal — platform string resolution
// ---------------------------------------------------------------------------

/// Resolve the human-readable platform descriptor for the current OS + arch.
///
/// All values are compile-time constants — no runtime inspection required.
fn platform_string() -> &'static str {
    match (OS, ARCH) {
        // ── macOS ──────────────────────────────────────────────────────────
        // Apple Silicon: M1/M2/M3/M4 — arm64 (aarch64)
        ("macos", "aarch64") => "macOS; Apple Silicon",
        // Intel Mac — x86_64
        ("macos", "x86_64") => "macOS; Intel",
        // Future Mac architectures
        ("macos", _) => "macOS",

        // ── Windows ────────────────────────────────────────────────────────
        // Standard 64-bit Windows (most common)
        ("windows", "x86_64") => "Windows; x64",
        // Windows on ARM (Snapdragon X Elite, Surface Pro X, etc.)
        ("windows", "aarch64") => "Windows; ARM64",
        // 32-bit Windows (uncommon but possible)
        ("windows", "x86") => "Windows; x86",
        // Other Windows architectures
        ("windows", _) => "Windows",

        // ── Linux ──────────────────────────────────────────────────────────
        // 64-bit Intel/AMD Linux (servers, workstations, most desktops)
        ("linux", "x86_64") => "Linux; x86_64",
        // 64-bit ARM Linux — includes Raspberry Pi 4/5 (64-bit OS),
        // AWS Graviton, Ampere Altra, NVIDIA Jetson, Apple M-series VMs
        ("linux", "aarch64") => "Linux; ARM64",
        // 32-bit ARM Linux — includes Raspberry Pi OS 32-bit, older Pi models
        ("linux", "arm") => "Linux; ARM",
        // RISC-V 64-bit (HiFive Unleashed, Starfive VisionFive 2, etc.)
        ("linux", "riscv64") => "Linux; RISC-V",
        // Other Linux architectures (s390x, powerpc64, mips, etc.)
        ("linux", _) => "Linux",

        // ── FreeBSD / OpenBSD / NetBSD ─────────────────────────────────────
        ("freebsd", _) => "FreeBSD",
        ("openbsd", _) => "OpenBSD",
        ("netbsd", _) => "NetBSD",

        // ── Unknown / unsupported ─────────────────────────────────────────
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_has_correct_prefix() {
        let ua = build_user_agent();
        assert!(
            ua.starts_with("MeedyaManager/"),
            "UA must start with 'MeedyaManager/': {ua}"
        );
    }

    #[test]
    fn user_agent_contains_version() {
        let ua = build_user_agent();
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            ua.contains(version),
            "UA must contain the crate version '{version}': {ua}"
        );
    }

    #[test]
    fn user_agent_has_platform_parens() {
        let ua = build_user_agent();
        // Must contain the parenthesised platform string: "MeedyaManager/1.x.y (OS...)"
        assert!(ua.contains('('), "UA must contain an opening paren: {ua}");
        assert!(ua.contains(')'), "UA must contain a closing paren: {ua}");
    }

    #[test]
    fn user_agent_no_empty_platform() {
        let ua = build_user_agent();
        // The platform section must not be empty parens "()"
        assert!(
            !ua.contains("()"),
            "UA must not have empty platform parens: {ua}"
        );
    }

    #[test]
    fn platform_string_is_non_empty() {
        assert!(
            !platform_string().is_empty(),
            "platform_string() must not be empty"
        );
    }

    #[test]
    fn user_agent_format_valid() {
        // Format: "MeedyaManager/X.Y.Z (Platform Details)"
        let ua = build_user_agent();
        // Starts with name
        assert!(ua.starts_with("MeedyaManager/"));
        // Contains a space before the platform
        assert!(ua.contains(" ("));
        // Ends with closing paren
        assert!(ua.ends_with(')'), "UA must end with ')': {ua}");
    }

    // -----------------------------------------------------------------------
    // contact_from() — pure fallback logic (no env access, safe under edition
    // 2024's `unsafe fn env::set_var`)
    // -----------------------------------------------------------------------

    #[test]
    fn contact_from_none_uses_default() {
        // No override at all — must fall back to the compiled-in default,
        // which contains both the default email and default URL.
        let contact = contact_from(None);
        assert!(
            contact.contains(DEFAULT_CONTACT_EMAIL),
            "default contact must contain the default email: {contact}"
        );
        assert!(
            contact.contains(DEFAULT_CONTACT_URL),
            "default contact must contain the default URL: {contact}"
        );
    }

    #[test]
    fn contact_from_empty_string_uses_default() {
        // An empty override string is treated as "not set".
        let contact = contact_from(Some(""));
        assert!(contact.contains(DEFAULT_CONTACT_EMAIL));
        assert!(contact.contains(DEFAULT_CONTACT_URL));
    }

    #[test]
    fn contact_from_whitespace_only_uses_default() {
        // Whitespace-only override is also treated as "not set" after trimming.
        let contact = contact_from(Some("  "));
        assert!(contact.contains(DEFAULT_CONTACT_EMAIL));
        assert!(contact.contains(DEFAULT_CONTACT_URL));
    }

    #[test]
    fn contact_from_some_value_used_verbatim() {
        // A real override is returned exactly as the trimmed value — no
        // defaults mixed in.
        let contact = contact_from(Some("me@example.com"));
        assert_eq!(contact, "me@example.com");
    }

    // -----------------------------------------------------------------------
    // build_user_agent_with_contact()
    // -----------------------------------------------------------------------

    #[test]
    fn user_agent_with_contact_starts_with_base_ua() {
        let full = build_user_agent_with_contact();
        let base = build_user_agent();
        assert!(
            full.starts_with(&base),
            "contact UA must start with the base UA: {full}"
        );
    }

    #[test]
    fn user_agent_with_contact_contains_separator() {
        let full = build_user_agent_with_contact();
        assert!(
            full.contains(" ( "),
            "contact UA must contain the ' ( ' separator: {full}"
        );
    }

    #[test]
    fn user_agent_with_contact_ends_with_closing_paren() {
        let full = build_user_agent_with_contact();
        assert!(full.ends_with(')'), "contact UA must end with ')': {full}");
    }
}
