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
// Public API:
//   build_user_agent() → String     — full UA string for this build

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
}
