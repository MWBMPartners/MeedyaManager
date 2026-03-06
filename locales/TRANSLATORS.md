# MeedyaManager — Translator Guide

> **(C) 2025-2026 MWBM Partners Ltd**

Thank you for contributing a translation to MeedyaManager!
This guide covers all three platform targets.

---

## Platform Overview

| Platform | Format | Location |
|----------|--------|----------|
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

3. Compile to a binary `.mo` file:
   ```bash
   msgfmt -o locales/<lang>/LC_MESSAGES/meedyamanager.mo \
             locales/<lang>/LC_MESSAGES/meedyamanager.po
   ```

4. Test by running the app with the locale set:
   ```bash
   LANG=fr_FR.UTF-8 MEEDYA_LOCALE_DIR=locales ./target/debug/meedya scan /tmp
   ```

### Extracting new strings

When new strings are added to Rust source files, extract them with:
```bash
xgettext --language=C --keyword=gettext --keyword=t \
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

3. Build the app in Xcode — Xcode compiles `.xcstrings` automatically.

4. Test by changing the macOS system language in System Settings → Language & Region.

---

## Windows (WinUI 3 `.resw`)

### Steps

1. Create the locale directory and copy the English resource file:
   ```
   windows/MeedyaManager/Strings/<BCP47>/Resources.resw
   ```
   For example: `Strings/fr-FR/Resources.resw`

2. Open the `.resw` in Visual Studio or any XML editor.

3. Translate the `<value>` text for each `<data>` element — **leave the `name` attribute unchanged**:
   ```xml
   <data name="Scan.Button.Scan" xml:space="preserve">
     <value>Analyser</value>
   </data>
   ```

4. Build the project — MSBuild packages the `.resw` files into the MSIX bundle.

5. Test by changing the Windows display language (Settings → Time & Language → Language).

---

## Submitting a Translation

1. Fork the repository.
2. Add your translation files following the steps above.
3. Open a Pull Request with the title: `i18n: add <Language> translation (<lang-code>)`.

Questions? Open a GitHub Issue or email [dev@mwbm.co.uk](mailto:dev@mwbm.co.uk).
