// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Cover Art Utilities
//
// Helpers for selecting, validating, and describing cover art returned by
// metadata providers. Providers attach `CoverArtInfo` structs to `ProviderResult`;
// the functions in this module help callers choose the best image and verify URLs.
//
// MIGRATION NOTE (#132): the upstream `CoverArtInfo` uses `Option<u32>` for
// `width`/`height` and has no `pixel_count()` / `has_dimensions()` methods.
// We provide local equivalents as free functions / inline expressions so this
// module's selectors continue to work.

use crate::traits::CoverArtInfo;

// ---------------------------------------------------------------------------
// Helpers — inline replacements for the removed CoverArtInfo methods
// ---------------------------------------------------------------------------

/// Returns `width * height` for a `CoverArtInfo`, treating absent dimensions as zero.
///
/// Replaces the local-only `CoverArtInfo::pixel_count()` method that no longer
/// exists on the upstream type.
fn pixel_count(a: &CoverArtInfo) -> u64 {
    u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0))
}

/// Returns the width (zero if unknown).
fn width(a: &CoverArtInfo) -> u32 {
    a.width.unwrap_or(0)
}

/// Returns the height (zero if unknown).
fn height(a: &CoverArtInfo) -> u32 {
    a.height.unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Cover art size classification
// ---------------------------------------------------------------------------

/// A size classification for cover art images.
///
/// Sizes are approximate; the exact thresholds are:
///   - Thumbnail  :   < 200 px on shortest side
///   - Small      : 200–499 px on shortest side
///   - Medium     : 500–999 px on shortest side
///   - Large      : 1000–1999 px on shortest side
///   - ExtraLarge : ≥ 2000 px on shortest side
///   - Unknown    : dimensions not reported (width or height is None/0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoverArtSize {
    /// Dimensions unknown / not reported
    Unknown,
    /// < 200 px (suitable for list thumbnails)
    Thumbnail,
    /// 200–499 px (preview quality)
    Small,
    /// 500–999 px (standard display)
    Medium,
    /// 1000–1999 px (hi-DPI display)
    Large,
    /// ≥ 2000 px (print quality)
    ExtraLarge,
}

impl CoverArtSize {
    /// Classify a `CoverArtInfo` entry by its shortest dimension.
    pub fn from_art(art: &CoverArtInfo) -> Self {
        let w = width(art);
        let h = height(art);
        if w == 0 || h == 0 {
            return Self::Unknown;
        }
        let min_side = w.min(h);
        match min_side {
            0..=199 => Self::Thumbnail,
            200..=499 => Self::Small,
            500..=999 => Self::Medium,
            1000..=1999 => Self::Large,
            _ => Self::ExtraLarge,
        }
    }
}

impl std::fmt::Display for CoverArtSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Thumbnail => write!(f, "thumbnail"),
            Self::Small => write!(f, "small"),
            Self::Medium => write!(f, "medium"),
            Self::Large => write!(f, "large"),
            Self::ExtraLarge => write!(f, "extra-large"),
        }
    }
}

// ---------------------------------------------------------------------------
// Selection helpers
// ---------------------------------------------------------------------------

/// Select the largest cover art from a slice, by pixel count.
///
/// Returns `None` if the slice is empty.
pub fn select_largest(arts: &[CoverArtInfo]) -> Option<&CoverArtInfo> {
    arts.iter().max_by_key(|a| pixel_count(a))
}

/// Select the smallest cover art from a slice, by pixel count.
///
/// Returns `None` if the slice is empty.
pub fn select_smallest(arts: &[CoverArtInfo]) -> Option<&CoverArtInfo> {
    arts.iter().min_by_key(|a| pixel_count(a))
}

/// Select the best cover art that meets a minimum size requirement.
///
/// Returns the largest image whose shortest side is at least `min_side_px` pixels.
/// Falls back to the overall largest image if nothing meets the minimum.
pub fn select_best(arts: &[CoverArtInfo], min_side_px: u32) -> Option<&CoverArtInfo> {
    if arts.is_empty() {
        return None;
    }
    // First: look for an image that meets the minimum size
    if let Some(best) = arts
        .iter()
        .filter(|a| width(a) >= min_side_px && height(a) >= min_side_px)
        .max_by_key(|a| pixel_count(a))
    {
        return Some(best);
    }

    // Fallback: return the largest available
    select_largest(arts)
}

/// Filter a list of cover art images to only include those at or above a minimum size.
///
/// Returns a `Vec<&CoverArtInfo>` sorted largest-first.
pub fn filter_by_min_size(arts: &[CoverArtInfo], min_side_px: u32) -> Vec<&CoverArtInfo> {
    let mut qualifying: Vec<&CoverArtInfo> = arts
        .iter()
        .filter(|a| width(a) >= min_side_px && height(a) >= min_side_px)
        .collect();
    // Sort largest-first
    qualifying.sort_by_key(|a| std::cmp::Reverse(pixel_count(a)));
    qualifying
}

// ---------------------------------------------------------------------------
// URL validation
// ---------------------------------------------------------------------------

/// Check whether a URL looks plausibly like an image URL.
///
/// This is a lightweight check (no network request). It verifies:
///   - The URL is non-empty
///   - It starts with `http://` or `https://`
///   - It is at least 10 characters long (prevents trivial truncations)
pub fn is_valid_art_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    let has_scheme = url.starts_with("https://") || url.starts_with("http://");
    has_scheme && url.len() >= 10
}

/// Check whether a URL points to a likely JPEG or PNG image based on the path extension.
///
/// Returns `true` if the URL path ends with `.jpg`, `.jpeg`, `.png`, or `.webp`
/// (case-insensitive). Returns `false` for URLs with no extension or other extensions
/// (e.g. signed CDN URLs that hide the extension).
pub fn url_has_image_extension(url: &str) -> bool {
    let lower = url.to_lowercase();
    // Strip query string for extension check
    let path = lower.split('?').next().unwrap_or(&lower);
    path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".png")
        || path.ends_with(".webp")
}

// ---------------------------------------------------------------------------
// MIME type helpers
// ---------------------------------------------------------------------------

/// Return the expected MIME type for a cover art URL based on its extension.
///
/// Falls back to `"image/jpeg"` for unknown or missing extensions (most common).
pub fn mime_type_for_url(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    let path = lower.split('?').next().unwrap_or(&lower);
    if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg" // Default: JPEG is by far the most common
    }
}

// ---------------------------------------------------------------------------
// De-duplication
// ---------------------------------------------------------------------------

/// De-duplicate a list of cover art images by URL.
///
/// Keeps the first occurrence of each URL (preserving priority ordering).
pub fn deduplicate(arts: Vec<CoverArtInfo>) -> Vec<CoverArtInfo> {
    let mut seen = std::collections::HashSet::new();
    arts.into_iter()
        .filter(|a| seen.insert(a.url.clone()))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::CoverArtInfo;

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

    // --- CoverArtSize::from_art ---

    #[test]
    fn cover_art_size_unknown_when_no_dimensions() {
        let a = art_unknown("https://x.com/a.jpg");
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Unknown);
    }

    #[test]
    fn cover_art_size_thumbnail() {
        let a = art("https://x.com/a.jpg", 100, 100);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Thumbnail);
    }

    #[test]
    fn cover_art_size_small() {
        let a = art("https://x.com/a.jpg", 300, 300);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Small);
    }

    #[test]
    fn cover_art_size_medium() {
        let a = art("https://x.com/a.jpg", 500, 500);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Medium);
    }

    #[test]
    fn cover_art_size_large() {
        let a = art("https://x.com/a.jpg", 1400, 1400);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Large);
    }

    #[test]
    fn cover_art_size_extra_large() {
        let a = art("https://x.com/a.jpg", 3000, 3000);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::ExtraLarge);
    }

    #[test]
    fn cover_art_size_uses_min_dimension() {
        // 100 × 3000 — shortest side is 100 → Thumbnail
        let a = art("https://x.com/a.jpg", 100, 3000);
        assert_eq!(CoverArtSize::from_art(&a), CoverArtSize::Thumbnail);
    }

    #[test]
    fn cover_art_size_display() {
        assert_eq!(CoverArtSize::Thumbnail.to_string(), "thumbnail");
        assert_eq!(CoverArtSize::ExtraLarge.to_string(), "extra-large");
    }

    // --- select_largest ---

    #[test]
    fn select_largest_returns_biggest_by_pixel_count() {
        let arts = vec![
            art("https://x.com/s.jpg", 300, 300),
            art("https://x.com/l.jpg", 1400, 1400),
            art("https://x.com/m.jpg", 500, 500),
        ];
        let best = select_largest(&arts).unwrap();
        assert_eq!(best.width, Some(1400));
    }

    #[test]
    fn select_largest_empty_returns_none() {
        assert!(select_largest(&[]).is_none());
    }

    // --- select_smallest ---

    #[test]
    fn select_smallest_returns_smallest_by_pixel_count() {
        let arts = vec![
            art("https://x.com/s.jpg", 300, 300),
            art("https://x.com/t.jpg", 100, 100),
        ];
        let small = select_smallest(&arts).unwrap();
        assert_eq!(small.width, Some(100));
    }

    // --- select_best ---

    #[test]
    fn select_best_meets_minimum_returns_qualifying() {
        let arts = vec![
            art("https://x.com/s.jpg", 300, 300),
            art("https://x.com/l.jpg", 1400, 1400),
        ];
        let best = select_best(&arts, 1000).unwrap();
        assert_eq!(best.width, Some(1400));
    }

    #[test]
    fn select_best_no_qualifying_falls_back_to_largest() {
        let arts = vec![
            art("https://x.com/a.jpg", 200, 200),
            art("https://x.com/b.jpg", 300, 300),
        ];
        // No image meets 2000px minimum
        let best = select_best(&arts, 2000).unwrap();
        assert_eq!(best.width, Some(300)); // Fallback: largest available
    }

    // --- is_valid_art_url ---

    #[test]
    fn valid_art_url_https() {
        assert!(is_valid_art_url("https://example.com/cover.jpg"));
    }

    #[test]
    fn valid_art_url_http() {
        assert!(is_valid_art_url("http://example.com/cover.jpg"));
    }

    #[test]
    fn invalid_art_url_empty() {
        assert!(!is_valid_art_url(""));
    }

    #[test]
    fn invalid_art_url_no_scheme() {
        assert!(!is_valid_art_url("example.com/cover.jpg"));
    }

    // --- url_has_image_extension ---

    #[test]
    fn url_has_image_extension_jpg() {
        assert!(url_has_image_extension("https://x.com/cover.jpg"));
    }

    #[test]
    fn url_has_image_extension_png_with_query() {
        assert!(url_has_image_extension("https://x.com/cover.png?w=500"));
    }

    #[test]
    fn url_no_extension_returns_false() {
        assert!(!url_has_image_extension(
            "https://cdn.example.com/signed/abc123"
        ));
    }

    // --- deduplicate ---

    #[test]
    fn deduplicate_removes_duplicate_urls() {
        let arts = vec![
            art("https://x.com/a.jpg", 500, 500),
            art("https://x.com/a.jpg", 500, 500), // duplicate
            art("https://x.com/b.jpg", 1000, 1000),
        ];
        let deduped = deduplicate(arts);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn deduplicate_preserves_order() {
        let arts = vec![
            art("https://x.com/first.jpg", 500, 500),
            art("https://x.com/second.jpg", 300, 300),
        ];
        let deduped = deduplicate(arts);
        assert_eq!(deduped[0].url, "https://x.com/first.jpg");
    }
}
