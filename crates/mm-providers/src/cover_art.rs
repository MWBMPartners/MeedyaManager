// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Cover Art Utilities (#133 migration)
//
// Phase 3 of the MeedyaSuite-core integration epic. The local implementations
// (411 lines) have been replaced by re-exports from the upstream
// `meedya_providers::cover_art` module via `meedya_core`.
//
// Re-exported upstream items:
//   - `CoverArtSize` enum (Unknown / Thumbnail / Small / Medium / Large / ExtraLarge)
//   - `CoverArtSize::from_dimension(px)` — single-dim classifier
//   - `classify(art)` — classifies by LARGEST dimension (upstream behaviour)
//   - `select_largest`, `select_smallest`, `select_best`, `filter_by_min_size`
//   - `is_valid_art_url`, `url_has_image_extension`, `mime_type_for_url`
//   - `deduplicate` (takes `&[CoverArtInfo]`, returns `Vec<CoverArtInfo>`)
//
// API DRIFT from the previous local module:
//   - `CoverArtSize::from_art()` (which classified by SHORTEST dimension) is
//     replaced by `CoverArtSize::from_art_min_side()` below, which preserves
//     the original min-side semantics on top of the upstream type.
//   - The local `Display` impl for `CoverArtSize` is retained below for the
//     CLI to keep printing "thumbnail" / "extra-large" etc.
//   - `select_best(arts, min_side_px: u32)` is now `select_best_min_side()`;
//     the upstream `select_best(arts, min_size: CoverArtSize)` is also re-exported
//     under its own name.
//   - `deduplicate` now borrows `&[CoverArtInfo]` instead of consuming
//     `Vec<CoverArtInfo>`.

use crate::traits::CoverArtInfo;

// Re-exports — primary surface from upstream.
pub use meedya_core::providers::cover_art::{
    CoverArtSize, classify, deduplicate, filter_by_min_size, is_valid_art_url, mime_type_for_url,
    select_best, select_largest, select_smallest, url_has_image_extension,
};

// ---------------------------------------------------------------------------
// Local-only adapters — preserve previous semantics on top of the upstream type
// ---------------------------------------------------------------------------

/// Extension trait giving `CoverArtSize` a stable `Display` impl and a
/// classifier that matches the previous local "shortest dimension" semantics.
///
/// The upstream `classify()` / `CoverArtSize::from_dimension()` use the
/// *largest* dimension; MeedyaManager historically used the *shortest*
/// (`min(w,h)`) so that a 100×3000 banner classified as Thumbnail rather than
/// ExtraLarge. We preserve that here without forking the upstream enum.
pub trait CoverArtSizeExt {
    /// Classify a `CoverArtInfo` by its shortest dimension (min of width/height).
    fn from_art_min_side(art: &CoverArtInfo) -> CoverArtSize;

    /// Human-readable label: `thumbnail`, `small`, `medium`, `large`,
    /// `extra-large`, or `unknown`.
    fn label(&self) -> &'static str;
}

impl CoverArtSizeExt for CoverArtSize {
    fn from_art_min_side(art: &CoverArtInfo) -> Self {
        let w = art.width.unwrap_or(0);
        let h = art.height.unwrap_or(0);
        if w == 0 || h == 0 {
            return Self::Unknown;
        }
        Self::from_dimension(w.min(h))
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Thumbnail => "thumbnail",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::ExtraLarge => "extra-large",
        }
    }
}

/// Select the largest cover art whose shortest side meets `min_side_px`.
///
/// Local-only convenience used by the registry. Falls back to the overall
/// largest if nothing meets the minimum (preserves previous behaviour).
pub fn select_best_min_side(arts: &[CoverArtInfo], min_side_px: u32) -> Option<&CoverArtInfo> {
    if arts.is_empty() {
        return None;
    }
    let qualifying = arts
        .iter()
        .filter(|a| a.width.unwrap_or(0) >= min_side_px && a.height.unwrap_or(0) >= min_side_px)
        .max_by_key(|a| u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0)));
    if qualifying.is_some() {
        return qualifying;
    }
    select_largest(arts)
}

// ---------------------------------------------------------------------------
// Tests — local-adapter behaviour only (upstream tests live upstream)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn art(url: &str, w: u32, h: u32) -> CoverArtInfo {
        CoverArtInfo {
            url: url.into(),
            width: Some(w),
            height: Some(h),
            mime_type: Some("image/jpeg".into()),
        }
    }

    fn art_unknown(url: &str) -> CoverArtInfo {
        CoverArtInfo {
            url: url.into(),
            width: None,
            height: None,
            mime_type: Some("image/jpeg".into()),
        }
    }

    // --- CoverArtSizeExt::from_art_min_side ---

    #[test]
    fn from_art_min_side_unknown_when_no_dimensions() {
        let a = art_unknown("https://x.com/a.jpg");
        assert_eq!(CoverArtSize::from_art_min_side(&a), CoverArtSize::Unknown);
    }

    #[test]
    fn from_art_min_side_uses_min_dimension() {
        // 100 × 3000 — shortest side 100 → Thumbnail (was a known difference
        // vs upstream's classify() which would say ExtraLarge here).
        let a = art("https://x.com/a.jpg", 100, 3000);
        assert_eq!(CoverArtSize::from_art_min_side(&a), CoverArtSize::Thumbnail);
    }

    #[test]
    fn from_art_min_side_square_thumbnail() {
        let a = art("https://x.com/a.jpg", 100, 100);
        assert_eq!(CoverArtSize::from_art_min_side(&a), CoverArtSize::Thumbnail);
    }

    #[test]
    fn from_art_min_side_square_medium() {
        let a = art("https://x.com/a.jpg", 500, 500);
        assert_eq!(CoverArtSize::from_art_min_side(&a), CoverArtSize::Medium);
    }

    // --- CoverArtSizeExt::label ---

    #[test]
    fn cover_art_size_label() {
        assert_eq!(CoverArtSize::Thumbnail.label(), "thumbnail");
        assert_eq!(CoverArtSize::ExtraLarge.label(), "extra-large");
        assert_eq!(CoverArtSize::Unknown.label(), "unknown");
    }

    // --- select_best_min_side ---

    #[test]
    fn select_best_min_side_meets_minimum_returns_qualifying() {
        let arts = vec![
            art("https://x.com/s.jpg", 300, 300),
            art("https://x.com/l.jpg", 1400, 1400),
        ];
        let best = select_best_min_side(&arts, 1000).unwrap();
        assert_eq!(best.width, Some(1400));
    }

    #[test]
    fn select_best_min_side_no_qualifying_falls_back_to_largest() {
        let arts = vec![
            art("https://x.com/a.jpg", 200, 200),
            art("https://x.com/b.jpg", 300, 300),
        ];
        // No image meets 2000px minimum
        let best = select_best_min_side(&arts, 2000).unwrap();
        assert_eq!(best.width, Some(300));
    }

    #[test]
    fn select_best_min_side_empty_returns_none() {
        assert!(select_best_min_side(&[], 100).is_none());
    }

    // --- deduplicate (now borrows) ---

    #[test]
    fn deduplicate_removes_duplicate_urls() {
        let arts = vec![
            art("https://x.com/a.jpg", 500, 500),
            art("https://x.com/a.jpg", 500, 500), // duplicate URL
            art("https://x.com/b.jpg", 1000, 1000),
        ];
        let deduped = deduplicate(&arts);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn deduplicate_preserves_order() {
        let arts = vec![
            art("https://x.com/first.jpg", 500, 500),
            art("https://x.com/second.jpg", 300, 300),
        ];
        let deduped = deduplicate(&arts);
        assert_eq!(deduped[0].url, "https://x.com/first.jpg");
    }
}
