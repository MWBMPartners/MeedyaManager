// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Engine Missing Banner (issue #222)
//
// Shown wherever the app would otherwise be showing, or letting the user act
// on, results made up by a stub because the Rust engine (mm-ffi) is not
// linked into this build. `Package.swift`'s `MM_FFI_AVAILABLE` flag is
// commented out there and defined nowhere else, so this is the state of
// every build today (see issue #66 for the real fix — actually linking the
// XCFramework).
//
// Before this fix, an unlinked build invented plausible-looking data (fake
// scan previews, fake "Sample Track" metadata) with nothing on screen to say
// so. `ScanModel` then treated the invented previews as safe to act on,
// which is how real music files ended up renamed to meaningless
// "Preview"-prefixed names — see the P0 write-up on issue #222. Every screen
// that could reach that situation now shows this banner instead of the
// fabricated data.
//
// Visual pattern deliberately copied from `CloudView.swift`'s
// `AlphaPreviewBanner` (the M7 "nothing here is real yet" notice), so the
// whole app reads as one family of honesty banners rather than several
// different designs saying the same kind of thing.

import SwiftUI

/// A reusable, prominent notice banner (orange, warning triangle).
///
/// Kept `internal` rather than `private` because more than one file needs
/// it: `ScanView.swift` uses it directly for its Test Mode notice, and
/// `EngineMissingBanner` below wraps it for the engine-missing case.
struct NoticeBanner: View {
    let text: String

    var body: some View {
        Label(text, systemImage: "exclamationmark.triangle.fill")
            .font(.callout)
            .foregroundStyle(.orange)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.orange.opacity(0.15))
            .clipShape(RoundedRectangle(cornerRadius: 8))
            // One combined VoiceOver element, rather than the triangle icon
            // and the text being announced as two separate items.
            .accessibilityElement(children: .combine)
    }
}

/// Persistent notice that this build cannot do anything trustworthy in this
/// screen, because the Rust engine (mm-ffi) is not linked in.
///
/// Used at the top of both `ScanView.swift` (scanning and renaming) and
/// `MetadataView.swift` (tag reading and writing) — the two screens whose
/// stub fallbacks used to fabricate data that looked real.
struct EngineMissingBanner: View {
    var body: some View {
        NoticeBanner(
            text: "This build is running without the MeedyaManager engine. Scanning, metadata and renaming are unavailable, and nothing on disk will be changed. (issue #222)"
        )
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 12) {
        EngineMissingBanner()
        NoticeBanner(text: "Example of the general-purpose banner with different text.")
    }
    .padding()
    .frame(width: 480)
}
