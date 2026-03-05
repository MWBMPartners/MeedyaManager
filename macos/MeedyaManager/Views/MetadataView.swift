// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Editor View (macOS, M6)
//
// Displays all embedded tags of a selected media file and allows
// the user to edit and save them. Includes cover art display (M6).

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

            // ── Content area: cover art (left) + tag editor (right) ────────
            if model.tags.isEmpty {
                ContentUnavailableView(
                    "No file loaded",
                    systemImage: "tag.slash",
                    description: Text("Open a media file to view and edit its metadata.")
                )
            } else {
                HSplitView {
                    // Cover art panel — shown when a URL is available
                    CoverArtPanel(coverArtUrl: model.coverArtUrl)
                        .frame(minWidth: 160, idealWidth: 180, maxWidth: 200)

                    // Editable tag list
                    TagEditorList(model: model)
                }
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

// MARK: – Cover art panel

/// Shows embedded cover art (or a placeholder) on the left side of the metadata panel.
private struct CoverArtPanel: View {
    let coverArtUrl: String?

    var body: some View {
        VStack(spacing: 8) {
            if let urlString = coverArtUrl, let url = URL(string: urlString) {
                // Network-loaded cover art with placeholder while loading
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .empty:
                        ProgressView()
                            .frame(width: 160, height: 160)
                    case .success(let image):
                        image
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(width: 160, height: 160)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    case .failure:
                        CoverArtPlaceholder()
                    @unknown default:
                        CoverArtPlaceholder()
                    }
                }
            } else {
                // No art URL — show placeholder icon
                CoverArtPlaceholder()
            }

            Text("Cover Art")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// Placeholder icon displayed when no cover art is available.
private struct CoverArtPlaceholder: View {
    var body: some View {
        RoundedRectangle(cornerRadius: 8)
            .fill(.quaternary)
            .frame(width: 160, height: 160)
            .overlay {
                Image(systemName: "music.note")
                    .font(.system(size: 48))
                    .foregroundStyle(.secondary)
            }
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
