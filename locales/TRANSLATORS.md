# MeedyaManager — Translator Guide

> **(C) 2025-2026 MWBM Partners Ltd**

Thank you for your interest in translating MeedyaManager!

## ⚠️ Current status: translation infrastructure exists, but nothing is wired up yet

Before you invest time in a translation, please read this section — it will save you the
disappointment of a PR that changes no visible text.

Each platform ships the *scaffolding* for localisation, but none of the three UIs actually
look up a translated string at runtime:

- **Linux (gettext):** `mm_core::i18n::init()` (called from `mm-cli`'s `main()` and from
  `mm-gtk`'s `app::run()`) binds the GNU gettext `"meedyamanager"` text domain and sets the
  locale — but there are **zero** `gettext()` or `dgettext()` call sites anywhere in the
  workspace, and no `t!()` macro exists to wrap strings in. Every user-visible string in
  `mm-cli` and `mm-gtk` is a plain Rust string literal (for example
  `crates/mm-gtk/src/ui/scan_panel.rs:174` passes the literal `"Scan folder"` straight to
  `accessibility::set_label`). The 57 `msgid` entries in
  `locales/en_US/LC_MESSAGES/meedyamanager.po` are never looked up, and no `.mo` file is ever
  compiled or shipped — translating the `.po` file has **no runtime effect** today.
- **macOS:** `macos/MeedyaManager/Localizable.xcstrings` exists with 32 keys, but is
  referenced by **zero** Swift source files (no `String(localized:)`, no
  `NSLocalizedString`) — the SwiftUI views use literal English strings.
- **Windows:** `windows/MeedyaManager/Strings/en-US/Resources.resw` has 55 entries, but only
  **one** call site outside the resource-loading helper itself actually reads a `.resw`
  value — the rest of the WinUI 3 XAML/C# also uses literal English strings.

None of this is a translator-side problem — it is tracked as an engineering gap (see the
open i18n-wiring work; there is no dedicated GitHub issue number for it yet, so search for
"i18n" / "gettext" / "localiz" in the issue tracker before filing a new one).

**What this means in practice right now:**

- A `.po` translation, an `.xcstrings` localisation, or a `.resw` translation can be written
  and merged today, but it will sit inert until the corresponding UI code is changed to call
  `gettext()` / `String(localized:)` / read the `.resw` resource instead of using a literal
  string.
- If you want to help before that wiring lands, the most useful contribution is converting a
  literal string in the Rust/Swift/C# source to use the platform's localisation API — not
  populating `msgstr`/`localizations`/`<value>` entries that nothing reads yet.

The rest of this guide describes the **mechanics** of each format, so a translation is ready
to go the moment the wiring lands.

---

## Platform Overview

| Platform | Format | Location |
| -------- | ------ | -------- |
| **Linux CLI + GTK4** | GNU gettext `.po` / `.mo` | `locales/<lang>/LC_MESSAGES/meedyamanager.po` |
| **macOS (SwiftUI)** | Xcode `.xcstrings` | `macos/MeedyaManager/Localizable.xcstrings` |
| **Windows (WinUI 3)** | `.resw` XML | `windows/MeedyaManager/Strings/<lang>/Resources.resw` |

---

## Linux / CLI (GNU gettext)

### Prerequisites

```bash
sudo apt install gettext   # Debian/Ubuntu
brew install gettext       # macOS (for cross-compilation)
```

### Steps

1. Copy the English template:

   ```bash
   cp locales/en_US/LC_MESSAGES/meedyamanager.po \
      locales/<lang>/LC_MESSAGES/meedyamanager.po
   ```

   Replace `<lang>` with a POSIX locale code, e.g. `fr_FR`, `de_DE`, `ja_JP`.

2. Edit the `.po` file, filling in `msgstr` values for each `msgid`:

   ```po
   msgid "Scan folder"
   msgstr "Analyser le dossier"
   ```

3. Compile to a binary `.mo` file (useful for testing the mechanics — nothing in the shipped
   app reads it yet):

   ```bash
   msgfmt -o locales/<lang>/LC_MESSAGES/meedyamanager.mo \
             locales/<lang>/LC_MESSAGES/meedyamanager.po
   ```

4. `mm_core::i18n::init()` documents a `MEEDYA_LOCALE_DIR` environment variable as a
   developer override for the search path it binds gettext to
   (`crates/mm-core/src/i18n.rs`). Setting it will not currently change any displayed text,
   for the reasons above, but is useful once a string has been converted to call `gettext()`.

### Extracting strings once source is converted to call `gettext()`

There is no `t!()` macro — do not write `xgettext --keyword=t`. Once source lines are
converted to call `gettextrs::gettext(...)` directly, extract with:

```bash
xgettext --language=C --keyword=gettext \
  --output=locales/meedyamanager.pot \
  crates/mm-cli/src/**/*.rs crates/mm-gtk/src/**/*.rs
```

Then merge into each existing `.po`:

```bash
msgmerge --update locales/<lang>/LC_MESSAGES/meedyamanager.po locales/meedyamanager.pot
```

---

## macOS (Xcode `.xcstrings`)

### Steps

1. Open `macos/MeedyaManager/Localizable.xcstrings` in Xcode (or any JSON editor).

2. For each key, add a `localizations` entry for your language code:

   ```json
   "scan.button.scan" : {
     "localizations" : {
       "en" : { "stringUnit" : { "state" : "translated", "value" : "Scan" } },
       "fr" : { "stringUnit" : { "state" : "translated", "value" : "Analyser" } }
     }
   }
   ```

   Use BCP 47 language tags: `fr`, `de`, `ja`, `zh-Hans`, `pt-BR`, etc.

3. Build the app in Xcode — Xcode compiles `.xcstrings` automatically. As noted above, no
   SwiftUI view currently reads a key from this catalogue, so a build will not show your
   translation until a view is converted to `String(localized:)`.

4. Once that wiring exists, test by changing the macOS system language in
   System Settings → Language & Region.

---

## Windows (WinUI 3 `.resw`)

### Steps

1. Create the locale directory and copy the English resource file:

   ```text
   windows/MeedyaManager/Strings/<BCP47>/Resources.resw
   ```

   For example: `Strings/fr-FR/Resources.resw`

2. Open the `.resw` in Visual Studio or any XML editor.

3. Translate the `<value>` text for each `<data>` element — **leave the `name` attribute
   unchanged**:

   ```xml
   <data name="Scan.Button.Scan" xml:space="preserve">
     <value>Analyser</value>
   </data>
   ```

4. Build the project — MSBuild packages the `.resw` files into the MSIX bundle. Only one
   XAML/C# call site currently reads a `.resw` value at all, so most translated strings will
   not appear in the running app until more of the UI is converted.

5. Once that wiring exists, test by changing the Windows display language
   (Settings → Time & Language → Language).

---

## Submitting a Translation

1. Fork the repository.
2. Add your translation files following the steps above.
3. Open a Pull Request with the title: `i18n: add <Language> translation (<lang-code>)`.
4. Please mention in the PR description that you're aware the wiring is still in progress —
   it helps reviewers set expectations and avoids "why doesn't this show up" questions.

Questions? Open a GitHub Issue or email [dev@mwbm.co.uk](mailto:dev@mwbm.co.uk).
