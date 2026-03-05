// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Editor View (macOS)
//
// Displays all embedded tags of a selected media file and allows
// the user to edit and save them.

import SwiftUI

/// The Metadata editor panel for macOS.
struct MetadataView: View {

    @Environment(AppState.self) private var appState
    private var model: MetadataModel { appState.metadata }

    @State private var showFilePicker = false

    var body: some View {
        VStack(spacing: 0) {
            // ── Toolbar: file picker + audio properties ────────────────────
            HStack(spacing: 12) {
                Image(systemName: "doc.fill")
                    .foregroundStyle(.secondary)

                Text(model.filePath.map { URL(fileURLWithPath: $0).lastPathComponent } ?? "No file selected")
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundStyle(model.filePath == nil ? .secondary : .primary)

                Spacer()

                if !model.audioPropertiesText.isEmpty {
                    Text(model.audioPropertiesText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Button("Open…") { showFilePicker = true }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(.bar)

            Divider()

            // ── Tag editor ─────────────────────────────────────────────────
            if model.tags.isEmpty {
                ContentUnavailableView(
                    "No file loaded",
                    systemImage: "tag.slash",
                    description: Text("Open a media file to view and edit its metadata.")
                )
            } else {
                TagEditorList(model: model)
            }

            // ── Bottom status bar + action buttons ─────────────────────────
            Divider()

            HStack {
                Text(model.status)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Spacer()

                if model.hasUnsavedEdits {
                    Button("Revert") {
                        Task { await model.revert() }
                    }
                    .foregroundStyle(.secondary)

                    Button("Save Tags") {
                        Task { await model.saveAll() }
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
        .fileImporter(
            isPresented: $showFilePicker,
            allowedContentTypes: [.audio, .movie, .item],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                Task { await model.loadFile(url.path(percentEncoded: false)) }
            }
        }
        .disabled(model.isRunning)
        .overlay {
            if model.isRunning {
                ProgressView("Loading…")
                    .padding()
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
            }
        }
        .navigationTitle("Metadata")
    }
}

// MARK: – Tag editor list

private struct TagEditorList: View {
    @Bindable var model: MetadataModel

    var body: some View {
        List {
            ForEach($model.tags) { $tag in
                TagRow(key: tag.key, value: $tag.value) {
                    model.hasUnsavedEdits = true
                }
            }
        }
        .listStyle(.inset)
    }
}

// MARK: – Individual tag row

private struct TagRow: View {
    let key: String
    @Binding var value: String
    let onChange: () -> Void

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 16) {
            // Key label — fixed-width column
            Text(key)
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.secondary)
                .frame(width: 140, alignment: .trailing)

            // Editable value
            TextField(key, text: $value)
                .textFieldStyle(.plain)
                .onChange(of: value) { _, _ in onChange() }
        }
        .padding(.vertical, 2)
    }
}

#Preview {
    MetadataView()
        .environment(AppState())
        .frame(width: 700, height: 500)
}
