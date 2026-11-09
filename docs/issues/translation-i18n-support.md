# Translation / Internationalisation (i18n) Support

> **Design spec / reference document — not a live GitHub issue.**
>
> This file used to be misnamed `issue_130_translation_support.md` and claimed to be "Issue
> #130". That was wrong: the real **#130** is a different, still-open issue — *"feat: bundle
> MediaInfo CLI as managed dependency with update checking"*. There is currently **no dedicated
> GitHub issue** for i18n/translation work; file one before starting on the scope below. The
> related issue **#129** is *"chore: Add workspace lint configuration and resolve all clippy
> warnings"* (closed) — not a "Release Hardening" issue as an earlier draft of this file claimed.

**Status: not yet implemented.** Scaffolding exists but nothing is wired up:

- `crates/mm-core/src/i18n.rs` sets up the gettext domain/locale plumbing, but **zero**
  `gettext()` call sites exist anywhere in the Rust codebase — every user-facing string is still
  a hard-coded English literal, and no `.mo` file is ever compiled from the `.po` sources under
  `locales/`.
- The macOS `Localizable.xcstrings` catalogue (32 keys) is referenced by **0** Swift files.
- The Windows `Resources.resw` catalogue (55 entries) is referenced at exactly **1** call site
  (`windows/MeedyaManager/Helpers/ResourceHelper.cs`), not wired through the UI generally.
- Real translator-facing file locations (see `locales/TRANSLATORS.md`) are `locales/<lang>/LC_MESSAGES/meedyamanager.po`,
  `macos/MeedyaManager/Localizable.xcstrings`, and `windows/MeedyaManager/Strings/<lang>/Resources.resw`
  — not the `crates/mm-cli/i18n/` or `linux/po/<locale>/` paths an earlier draft of this document
  proposed.

Treat everything below as a design proposal to scope future work against, not as a status report.

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
- Store translation files in `locales/<lang>/LC_MESSAGES/` (`.po`/`.pot` for gettext — this is
  the layout already scaffolded in the repo; a Fluent-based `.ftl` layout would replace it, not
  add to it).
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
- Store `.po`/`.mo` files in `locales/<locale>/LC_MESSAGES/` (already scaffolded; see
  `locales/TRANSLATORS.md`), not a separate `linux/po/<locale>/` tree.
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

- `docs/issues/accessibility-support.md` — Accessibility Support (VoiceOver, Narrator, AT-SPI2).
  Tracked on GitHub as **#90** (closed), not #128.
- #129 — "chore: Add workspace lint configuration and resolve all clippy warnings" (closed) — not
  a "Release Hardening" issue.
- No GitHub issue currently tracks i18n/translation work — file one before starting.
