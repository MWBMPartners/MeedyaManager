// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Library / Scan View (macOS, M6)
//
// Lets the user:
//   1. Pick a source folder (file picker or drag-and-drop)
//   2. Enter a rename template
//   3. Preview computed renames
//   4. Execute approved renames

import SwiftUI
import UniformTypeIdentifiers

/// The Library / Scan panel for macOS.
struct ScanView: View {

    @Environment(AppState.self) private var appState

    // Local binding to the scan model via the environment
    private var model: ScanModel { appState.scan }

    @State private var showFolderPicker = false

    var body: some View {
        HSplitView {
            // ── Left pane: controls ────────────────────────────────────────
            VStack(alignment: .leading, spacing: 0) {
                OptionsPane(model: model, showFolderPicker: $showFolderPicker)
                Divider()
                Spacer()

                // Status bar at the bottom
                Text(model.status)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minWidth: 260, idealWidth: 280, maxWidth: 340)

            // ── Right pane: results list ───────────────────────────────────
            ResultsPane(model: model)
        }
        .fileImporter(
            isPresented: $showFolderPicker,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                // Store security-scoped bookmark path
                model.directoryPath = url.path(percentEncoded: false)
            }
        }
        .navigationTitle("Library")
    }
}

// MARK: – Options pane (left column)

private struct OptionsPane: View {
    @Bindable var model: ScanModel
    @Binding var showFolderPicker: Bool

    var body: some View {
        Form {
            // Folder picker row — also accepts drag-and-drop from Finder
            Section("Source") {
                HStack {
                    TextField("Folder path", text: $model.directoryPath)
                        .textFieldStyle(.roundedBorder)
                        // Accept dropped folders (or files — use parent directory)
                        .onDrop(of: [UTType.fileURL], isTargeted: nil) { providers in
                            guard let provider = providers.first else { return false }
                            _ = provider.loadObject(ofClass: URL.self) { url, _ in
                                guard let url else { return }
                                DispatchQueue.main.async {
                                    // If a file is dropped, use its parent folder
                                    let target = url.hasDirectoryPath ? url : url.deletingLastPathComponent()
                                    model.directoryPath = target.path(percentEncoded: false)
                                }
                            }
                            return true
                        }
                    Button("Browse…") { showFolderPicker = true }
                }
                Text("Tip: drag a folder from Finder to set the path.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            // Template entry
            Section("Rename Template") {
                TextField("<Artist> - <Title>", text: $model.template)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))

                // Inline validation feedback
                TemplateValidationBadge(template: model.template)
            }

            // Options
            Section("Options") {
                Toggle("Include sub-folders", isOn: $model.recursive)
            }

            // Scan + Execute buttons
            Section {
                HStack(spacing: 8) {
                    // Execute — only enabled when there are valid previews
                    Button("Execute Renames") {
                        Task { await model.executeRenames() }
                    }
                    .disabled(!model.canExecute || model.isRunning)
                    .foregroundStyle(.red)

                    Spacer()

                    // Scan
                    Button("Scan") {
                        Task { await model.scan() }
                    }
                    .disabled(model.directoryPath.isEmpty || model.isRunning)
                    .buttonStyle(.borderedProminent)
                }
            }
        }
        .formStyle(.grouped)
        .padding(.top, 8)
        .disabled(model.isRunning)
        // Show a progress indicator while scanning
        .overlay(alignment: .center) {
            if model.isRunning {
                ProgressView("Working…")
                    .padding()
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
            }
        }
    }
}

// MARK: – Results pane (right column)

private struct ResultsPane: View {
    let model: ScanModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Summary header
            if !model.previews.isEmpty {
                Text(model.summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)

                Divider()
            }

            if model.previews.isEmpty {
                // Empty state
                ContentUnavailableView(
                    "No files scanned",
                    systemImage: "folder.badge.questionmark",
                    description: Text("Select a folder and click Scan to preview renames.")
                )
            } else {
                // Rename preview list
                List(model.previews) { preview in
                    PreviewRow(preview: preview)
                }
                .listStyle(.plain)
            }
        }
    }
}

// MARK: – Individual preview row

private struct PreviewRow: View {
    let preview: RenamePreviewItem

    var body: some View {
        HStack(spacing: 8) {
            // Source filename
            Text(preview.sourceName)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(maxWidth: .infinity, alignment: .leading)

            // Arrow indicator
            Image(systemName: "arrow.right")
                .foregroundStyle(.secondary)
                .imageScale(.small)

            // Destination filename
            Text(preview.destinationName)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(maxWidth: .infinity, alignment: .leading)
                .foregroundStyle(preview.conflict ? .red : .primary)

            // Status badge
            Text(preview.badgeText)
                .font(.caption2)
                .fontWeight(.medium)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(badgeColor(preview).opacity(0.15), in: Capsule())
                .foregroundStyle(badgeColor(preview))
        }
        .padding(.vertical, 2)
    }

    private func badgeColor(_ p: RenamePreviewItem) -> Color {
        if p.conflict  { return .red    }
        if p.unchanged { return .gray   }
        return .green
    }
}

// MARK: – Template validation badge

private struct TemplateValidationBadge: View {
    let template: String

    var body: some View {
        let result = MmCore.shared.validateTemplate(template)
        if template.trimmingCharacters(in: .whitespaces).isEmpty {
            EmptyView()
        } else if result.isValid {
            Label("Valid template", systemImage: "checkmark.circle.fill")
                .font(.caption)
                .foregroundStyle(.green)
        } else {
            Label(result.message, systemImage: "exclamationmark.triangle.fill")
                .font(.caption)
                .foregroundStyle(.red)
        }
    }
}

#Preview {
    ScanView()
        .environment(AppState())
        .frame(width: 900, height: 600)
}
