// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rules / Template Builder View (macOS, M6)
//
// Full template builder: live validator, live preview with sample tags,
// and a tag-pill reference panel that appends tokens at the cursor.

import SwiftUI

/// Rules and template builder panel for macOS.
struct RulesView: View {

    @State private var ruleName: String = "Default Rule"
    @State private var template: String = "<Artist> - <Title>"
    @State private var sampleTags: [String: String] = [
        "artist": "Pink Floyd",
        "album":  "The Wall",
        "title":  "Comfortably Numb",
        "year":   "1979",
    ]
    @State private var previewResult: String = ""

    private let knownTags = MmCore.shared.listKnownTags()

    var body: some View {
        HSplitView {
            // ── Left: template editor + validator ─────────────────────────
            VStack(alignment: .leading, spacing: 0) {
                Form {
                    Section("Rule") {
                        // Rule name — used to identify this template in the rule list
                        TextField("Rule name", text: $ruleName)
                            .textFieldStyle(.roundedBorder)
                    }

                    Section("Rename Template") {
                        TextField("Template", text: $template)
                            .textFieldStyle(.roundedBorder)
                            .font(.system(.body, design: .monospaced))
                            .onChange(of: template) { _, new in updatePreview(new) }

                        // Validation feedback
                        ValidationFeedback(template: template)
                    }

                    Section("Live Preview") {
                        if previewResult.isEmpty {
                            Text("Enter a template above")
                                .foregroundStyle(.secondary)
                        } else {
                            Text(previewResult)
                                .font(.system(.body, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    }

                    Section("Sample Tags (for preview)") {
                        ForEach(Array(sampleTags.keys.sorted()), id: \.self) { key in
                            HStack {
                                Text(key)
                                    .frame(width: 80, alignment: .trailing)
                                    .foregroundStyle(.secondary)
                                TextField(key, text: Binding(
                                    get: { sampleTags[key] ?? "" },
                                    set: { sampleTags[key] = $0; updatePreview(template) }
                                ))
                                .textFieldStyle(.roundedBorder)
                            }
                        }
                    }
                }
                .formStyle(.grouped)

                Spacer()
            }
            .frame(minWidth: 300, idealWidth: 340)

            // ── Right: tag reference list ──────────────────────────────────
            VStack(alignment: .leading, spacing: 8) {
                Text("Available Tags")
                    .font(.headline)
                    .padding(.horizontal, 16)
                    .padding(.top, 12)

                Text("Click a tag to insert it at the cursor position.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 16)

                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 150))], spacing: 8) {
                        ForEach(knownTags, id: \.self) { tag in
                            TagPill(tag: tag) {
                                // Append the tag to the template
                                template += "<\(tag)>"
                                updatePreview(template)
                            }
                        }
                    }
                    .padding(16)
                }
            }
        }
        .navigationTitle("Rules")
        .onAppear { updatePreview(template) }
    }

    // Evaluate the template against sample tags
    private func updatePreview(_ tmpl: String) {
        guard !tmpl.trimmingCharacters(in: .whitespaces).isEmpty else {
            previewResult = ""
            return
        }

        // Build FfiTagEntry array from sample tags
        let tags = sampleTags.map { FfiTagEntry(key: $0.key, value: $0.value) }

        // Validate first — don't try to evaluate invalid templates
        let validation = MmCore.shared.validateTemplate(tmpl)
        guard validation.isValid else {
            previewResult = ""
            return
        }

        // In stub mode, do simple <Tag> substitution for preview
        var result = tmpl
        for entry in tags {
            // Replace both <Key> and <key> variants (case-insensitive)
            result = result.replacingOccurrences(
                of: "<\(entry.key)>",
                with: entry.value,
                options: [.caseInsensitive]
            )
        }
        previewResult = result
    }
}

// MARK: – Tag pill button

private struct TagPill: View {
    let tag: String
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            Text("<\(tag)>")
                .font(.system(.caption, design: .monospaced))
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
        }
        .buttonStyle(.bordered)
        .tint(.accentColor)
    }
}

// MARK: – Inline validation feedback

private struct ValidationFeedback: View {
    let template: String

    var body: some View {
        let trimmed = template.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty {
            EmptyView()
        } else {
            let result = MmCore.shared.validateTemplate(template)
            if result.isValid {
                Label("Valid", systemImage: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                    .font(.caption)
            } else {
                Label(result.message, systemImage: "xmark.circle.fill")
                    .foregroundStyle(.red)
                    .font(.caption)
            }
        }
    }
}

#Preview {
    RulesView()
        .environment(AppState())
        .frame(width: 900, height: 600)
}
