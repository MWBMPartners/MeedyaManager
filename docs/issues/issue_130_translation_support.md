# Issue #130 — Translation / Internationalisation (i18n) Support

**Milestone:** Post-Release (v1.1.0)
**Labels:** `enhancement`, `i18n`, `accessibility`, `ux`
**Priority:** Medium
**Status:** Open

---

## Summary

Add internationalisation (i18n) and localisation (l10n) support to MeedyaManager
across all three platforms so the application can be translated into additional
languages beyond English.

---

## Motivation

MeedyaManager is a cross-platform desktop application used worldwide. Providing
translations lowers the barrier for non-English-speaking users and is a
prerequisite for App Store approval in many regions. It also complements the
accessibility work (#128) in making the application inclusive.

---

## Scope

### Rust CLI (`mm-cli`)

- Introduce a localisation crate (e.g. `fluent` or `gettext-rs`) to externalise
  all user-facing strings in the CLI output.
- Store translation files in `crates/mm-cli/i18n/` (`.ftl` for Fluent,
  or `.po`/`.pot` for gettext).
- Detect the locale from `LANG`/`LC_ALL` environment variables at runtime.
- Provide a `--lang <LOCALE>` global flag to override the detected locale.
- Fall back to English (`en-US`) when the requested locale is unavailable.

### macOS SwiftUI

- Use `NSLocalizedString` / `String(localized:)` (Swift 5.7+) for all UI strings.
- Store `.strings` / `.xcstrings` files in `macos/MeedyaManager/Resources/en.lproj/`
  (and additional language subfolders as translations are contributed).
- Enable the `CFBundleLocalizations` key in `Info.plist`.
- Support Dynamic Type and right-to-left (RTL) layouts for Arabic/Hebrew locales.

### Windows WinUI 3

- Use WinUI 3 / WinAppSDK resource strings (`resw` files) via
  `ResourceLoader.GetForCurrentView().GetString("Key")`.
- Store strings in `windows/MeedyaManager/Strings/<locale>/Resources.resw`.
- Respect the user's preferred language set in Windows Settings.
- Support RTL layouts for Arabic/Hebrew using `FlowDirection="RightToLeft"`.

### Linux GTK4

- Use `gettextrs` crate for string externalisation.
- Store `.po`/`.mo` files in `linux/po/<locale>/`.
- Initialise `textdomain` in `mm-gtk/src/main.rs`.
- Follow GNOME internationalisation guidelines.

---

## Translation Infrastructure

- Extract English source strings into a single `.pot` template (or `.xcstrings`
  catalogue on macOS) for community translation.
- Host translations on Weblate or Transifex for community contributions.
- CI check: fail the build if a new user-facing string is added without an
  English base entry.
- Minimum target locales for v1.1.0: `en-US` (base), `fr-FR`, `de-DE`, `es-ES`,
  `ja-JP`, `zh-Hans`.

---

## Acceptance Criteria

- [ ] All CLI output strings are externalised (no hard-coded English in `println!` / `eprintln!`)
- [ ] macOS SwiftUI uses `String(localized:)` for all visible text
- [ ] Windows WinUI 3 uses `ResourceLoader` for all visible text
- [ ] Linux GTK4 uses `gettext!()` / `i18n!()` macro for all visible text
- [ ] English `.pot` / `.xcstrings` / `.resw` base files are committed to the repo
- [ ] CI enforces that no unlocalisable string is introduced
- [ ] `--lang` flag works in the CLI
- [ ] RTL layout is tested manually for a sample Arabic locale
- [ ] `docs/changelog.md` and `docs/roadmap.md` updated
- [ ] This issue closed after v1.1.0 release

---

## Implementation Notes

- Rust: `fluent-bundle` + `fluent-langneg` is the recommended modern choice;
  `gettext-rs` is simpler but requires system `libintl` on some platforms.
- macOS: `String(localized:)` is available from Swift 5.7 / macOS 13; for
  macOS 15 targeting we have no minimum-version concern.
- Windows: Pseudo-locale testing (`qps-ploc`) can be enabled in
  Windows Settings to catch truncation issues.
- GTK4: `glib::g_dgettext` and the `gettextrs` crate integrate well with
  the existing `gtk4-rs` setup.

---

## Related Issues

- #128 — Accessibility Support (VoiceOver, Narrator, AT-SPI2)
- #129 — Release Hardening (binary optimisation profiles)
