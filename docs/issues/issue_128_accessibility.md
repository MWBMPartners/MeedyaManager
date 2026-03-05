# Issue #128 — Accessibility Support

**Title:** Implement accessibility support across all three platforms

**Labels:** `type:feature`, `platform:all`, `priority:P1`, `milestone:M10`

**Milestone:** M10 — Secure Media Server + Public Release

**Assignee:** TBD

---

## Summary

MeedyaManager must be accessible to users who rely on assistive technologies.
This issue tracks the full accessibility implementation across the three native
UI platforms: macOS (SwiftUI), Windows (WinUI 3 / C#), and Linux (GTK4).

---

## Platform Requirements

### macOS — VoiceOver & Accessibility

Apple requires accessibility compliance for App Store submission. SwiftUI
provides built-in accessibility APIs via the `Accessibility` framework.

**Required changes:**

- Add `.accessibilityLabel()` to all interactive elements (buttons, pickers,
  text fields, sliders, toggles)
- Add `.accessibilityHint()` for non-obvious controls (e.g., the schema preview
  button, dry-run toggle)
- Add `.accessibilityValue()` for dynamic state (e.g., export progress, provider
  status, sync state)
- Add `.accessibilityIdentifier()` to all test-relevant elements
- Use `.accessibilityHeading()` on section titles in settings and preferences
- Use `.accessibilityElement(children: .combine)` for composite rows in lists
- Ensure `TabView` tabs have descriptive accessibility labels
- Use `.accessibilityAddTraits(.isButton)` where `onTapGesture` replaces Button
- Support `DynamicType` — all text must scale with user's preferred size
- Test with VoiceOver enabled on macOS 15+

**Target:** All views in `macos/MeedyaManager/Views/`

---

### Windows — Narrator & UIA

WinUI 3 uses the **UI Automation (UIA)** framework which Narrator and third-party
assistive tools (NVDA, JAWS) consume.

**Required changes:**

- Set `AutomationProperties.Name` on all interactive controls (Buttons, ComboBoxes,
  TextBoxes, ToggleSwitches) in XAML
- Set `AutomationProperties.HelpText` for non-obvious controls
- Set `AutomationProperties.LabeledBy` to link labels to their input fields
- Use `AutomationProperties.LiveSetting="Polite"` on status TextBlocks that update
  dynamically (export status, update InfoBar, sync log)
- Ensure all NavigationView items have descriptive names
- Ensure `TabIndex` is logical (matches visual reading order)
- Verify heading levels with `AutomationProperties.HeadingLevel`
- Ensure all icons/glyphs have text alternatives via `ToolTipService.ToolTip`
- Support High Contrast mode — test with all four Windows HC themes
- Verify focus indicators are visible (WinUI 3 default focus rect may be
  insufficient in some custom styles)

**Target:** All pages in `windows/MeedyaManager/Views/`

---

### Linux — AT-SPI2 & GTK4 Accessibility

GTK4 has built-in AT-SPI2 support through the `gtk-atspi` subsystem. When the
`accessibility` feature is enabled in GTK4, widgets automatically expose their
role, name, and state via AT-SPI2 to assistive tools (Orca screen reader).

**Required changes:**

- Set `widget.set_tooltip_text()` on all buttons and controls
- Set accessible names via `widget.update_property()` with `AccessibleProperty::Label`
  for unlabelled widgets (icon-only buttons)
- Use `widget.update_property()` with `AccessibleProperty::Description` for hints
- Ensure `AdwPreferencesGroup` title and subtitle describe the group purpose
- Use `gtk::Label::new()` with `set_mnemonic_widget()` to associate labels with inputs
- Ensure keyboard navigation works in all panels (Tab/Shift+Tab, arrow keys)
- Set `widget.update_state()` with `AccessibleState::Busy` during async operations
- Add accessible names to `ComboBoxText` entries (not just the combo itself)
- Test with Orca screen reader on GNOME
- Verify `high-contrast` theme works correctly (`adw-dark` + system HC)

**Target:** All panels in `crates/mm-gtk/src/ui/`

---

## Acceptance Criteria

- [ ] All interactive controls on all three platforms have accessible names and hints
- [ ] Dynamic status messages (export progress, sync log, update notification) are
      announced to screen readers without user action
- [ ] Keyboard-only navigation works fully on all three platforms
- [ ] VoiceOver audit passes on macOS (no "unlabelled" warnings)
- [ ] Narrator audit passes on Windows (Accessibility Insights for Windows: no errors)
- [ ] Orca navigates all GTK4 panels without silent areas
- [ ] High-contrast themes are supported on all platforms
- [ ] Dynamic Type / large text rendering works on macOS and Windows
- [ ] `#[cfg(test)]` accessibility identifiers are set on key UI elements
- [ ] CI includes a basic accessibility smoke test (can be an automated axe-like check)

---

## Tests to Add

| Platform | Test File | Description |
| -------- | --------- | ----------- |
| macOS | `macos/MeedyaManagerTests/AccessibilityTests.swift` | XCUITest: verify accessibility labels, VoiceOver traversal |
| Windows | `windows/MeedyaManager.Tests/AccessibilityTests.cs` | xUnit + Accessibility Insights API: UIA tree, label coverage |
| Linux (Rust) | `crates/mm-gtk/src/ui/accessibility_tests.rs` | Widget label presence, tooltip coverage |

---

## References

- [Apple Accessibility Programming Guide for iOS (applies to macOS)](https://developer.apple.com/accessibility/)
- [WinUI 3 Accessibility — Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-overview)
- [GTK4 Accessibility — GNOME Developer Docs](https://docs.gtk.org/gtk4/accessibility.html)
- [WCAG 2.1 Guidelines](https://www.w3.org/TR/WCAG21/)
- [Accessibility Insights for Windows](https://accessibilityinsights.io/docs/windows/overview/)

---

> *Created: 2026-03-05 | Planned for M10 alongside v1.0.0 public release*
