// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Lookup View (macOS, M6)
//
// Lets users search metadata providers and apply results to open files.
//
// Layout:
//   ┌────────────────────────────────────┬──────────────────────┐
//   │  Query + Artist fields + [Search] │  Provider checklist  │
//   │  ─────────────────────────────── │                      │
//   │  Results list (sorted by score)   │  * = stub provider   │
//   │    ● MusicBrainz  Title — Artist  │                      │
//   │    ● Spotify      ...             │                      │
//   │  ─────────────────────────────── │                      │
//   │  Detail card for selected result  │                      │
//   │  [Apply to File]  [Clear]         │                      │
//   └────────────────────────────────────┴──────────────────────┘

import SwiftUI

struct LookupView: View {

    @Environment(AppState.self) private var appState
    @State private var model = LookupModel()

    var body: some View {
        HSplitView {
            // ── Left: search + results ─────────────────────────────────────
            VStack(alignment: .leading, spacing: 0) {
                searchForm
                Divider()
                resultsList
                Divider()
                if let selected = model.selected {
                    resultDetail(selected)
                    Divider()
                }
                actionButtons
            }
            .frame(minWidth: 420)

            // ── Right: provider selector ────────────────────────────────────
            providerSelector
                .frame(minWidth: 180, maxWidth: 240)
        }
        .navigationTitle("Metadata Lookup")
    }

    // MARK: – Search form

    private var searchForm: some View {
        Form {
            Section {
                TextField("Track / show / podcast title", text: $model.query)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Search query")
                    .accessibilityHint("Enter a track title, TV show, or podcast name to search for")

                TextField("Artist (optional)", text: $model.artistHint)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("Artist hint (optional)")
                    .accessibilityHint("Narrows results to a specific artist or creator")

                HStack {
                    Spacer()
                    Button("Search") {
                        Task { await model.search() }
                    }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.return)
                    .disabled(model.isSearching || model.query.trimmingCharacters(in: .whitespaces).isEmpty)
                    .accessibilityLabel("Search providers")
                    .accessibilityHint("Queries all selected metadata providers for matching results")

                    if model.isSearching {
                        ProgressView()
                            .controlSize(.small)
                            .padding(.leading, 4)
                    }
                }
            } header: {
                Text("Search")
                    .font(.headline)
                    .padding(.bottom, 4)
            }
        }
        .formStyle(.grouped)
        .padding(.bottom, 0)
    }

    // MARK: – Results list

    @ViewBuilder
    private var resultsList: some View {
        if model.results.isEmpty && !model.isSearching {
            ContentUnavailableView(
                "No Results",
                systemImage: "magnifyingglass",
                description: Text(model.statusMessage)
            )
            .frame(maxWidth: .infinity)
            .padding()
        } else {
            List(model.results, selection: Binding(
                get: { model.selected?.id },
                set: { newID in
                    model.selected = model.results.first { $0.id == newID }
                }
            )) { result in
                ResultRow(result: result)
                    .tag(result.id)
            }
            .listStyle(.inset)
        }
    }

    // MARK: – Result detail card

    private func resultDetail(_ result: LookupResult) -> some View {
        GroupBox {
            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 12, verticalSpacing: 4) {
                if let title = result.title {
                    GridRow { Text("Title").gridColumnAlignment(.trailing).foregroundStyle(.secondary) ; Text(title) }
                }
                if let artist = result.artist {
                    GridRow { Text("Artist").gridColumnAlignment(.trailing).foregroundStyle(.secondary) ; Text(artist) }
                }
                if let album = result.album {
                    GridRow { Text("Album").gridColumnAlignment(.trailing).foregroundStyle(.secondary) ; Text(album) }
                }
                if let year = result.year {
                    GridRow { Text("Year").gridColumnAlignment(.trailing).foregroundStyle(.secondary) ; Text("\(year)") }
                }
                if let genre = result.genre {
                    GridRow { Text("Genre").gridColumnAlignment(.trailing).foregroundStyle(.secondary) ; Text(genre) }
                }
                GridRow {
                    Text("Score").gridColumnAlignment(.trailing).foregroundStyle(.secondary)
                    Text(String(format: "%.2f", result.score))
                        .foregroundStyle(result.score >= 0.8 ? .green : result.score >= 0.5 ? .orange : .red)
                }
                GridRow {
                    Text("Provider").gridColumnAlignment(.trailing).foregroundStyle(.secondary)
                    Text(result.provider)
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.quaternary, in: Capsule())
                }
            }
            .font(.callout)
        } label: {
            Text("Selected Result")
                .font(.subheadline)
                .bold()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    // MARK: – Action buttons

    private var actionButtons: some View {
        HStack {
            Text(model.statusMessage)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .accessibilityLabel("Lookup status: \(model.statusMessage)")
                .accessibilityLiveRegion(.polite)

            Spacer()

            Button("Clear") { model.clear() }
                .buttonStyle(.borderless)
                .disabled(model.results.isEmpty && model.query.isEmpty)
                .accessibilityLabel("Clear results")
                .accessibilityHint("Clears search results and resets the query fields")

            Button("Apply to File") {
                // Full apply-to-file requires the MetadataPanel to be open
                // with a file loaded. The binding is wired via AppState in a
                // subsequent patch when the FFI exposes tag write access.
                if let r = model.selected {
                    appState.metadata.applyLookupResult(
                        title: r.title,
                        artist: r.artist,
                        album: r.album,
                        year: r.year.map(String.init),
                        genre: r.genre
                    )
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(model.selected == nil)
            .accessibilityLabel("Apply result to file")
            .accessibilityHint("Copies the selected result's tags to the currently open file in the Metadata tab")
        }
        .padding(12)
    }

    // MARK: – Provider selector (right panel)

    private var providerSelector: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("Providers")
                .font(.headline)
                .padding(.horizontal, 12)
                .padding(.top, 12)
                .padding(.bottom, 6)

            Divider()

            List {
                ForEach($model.providers) { $provider in
                    Toggle(isOn: $provider.enabled) {
                        Text(provider.label)
                            .foregroundStyle(provider.isStub ? .secondary : .primary)
                            .font(provider.isStub ? .caption : .body)
                    }
                    .toggleStyle(.checkbox)
                }
            }
            .listStyle(.sidebar)

            Divider()

            Text("* = stub (no public API)")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .padding(8)
        }
    }
}

// MARK: – Result Row

private struct ResultRow: View {

    let result: LookupResult

    var body: some View {
        HStack(spacing: 8) {
            // Score circle
            Circle()
                .fill(scoreColor)
                .frame(width: 8, height: 8)

            // Provider badge
            Text(result.provider)
                .font(.caption2)
                .padding(.horizontal, 5)
                .padding(.vertical, 2)
                .background(.quaternary, in: Capsule())
                .frame(minWidth: 90, alignment: .leading)
                .lineLimit(1)

            // Title — Artist
            VStack(alignment: .leading, spacing: 1) {
                Text(result.displayTitle)
                    .font(.body)
                    .lineLimit(1)
                if !result.displaySubtitle.isEmpty {
                    Text(result.displaySubtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer()

            // Score label
            Text(String(format: "%.2f", result.score))
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 2)
    }

    private var scoreColor: Color {
        result.score >= 0.8 ? .green : result.score >= 0.5 ? .orange : .red
    }
}

#Preview {
    LookupView()
        .environment(AppState())
        .frame(width: 900, height: 620)
}
