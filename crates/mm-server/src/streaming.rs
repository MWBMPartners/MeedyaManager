// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Secure Media Server: Streaming (M10)
//
// Implements HTTP/1.1 and HTTP/2 byte-range streaming for media files:
//
//   StreamRequest  — parsed Range header parameters
//   StreamResponse — byte-range response metadata
//   StreamError    — typed streaming failure modes
//   StreamConfig   — buffer sizes and rate-limit settings
//   RangeParser    — parse `Range: bytes=start-end` headers (RFC 7233)
//   MediaStreamer   — resolve file path → StreamResponse with byte range
//
// Range requests follow RFC 7233:
//   - `Range: bytes=0-`         → from byte 0 to end of file
//   - `Range: bytes=0-1023`     → first 1024 bytes
//   - `Range: bytes=-512`       → last 512 bytes (suffix range)
//   - No Range header           → full file (200 OK)
//   - Out-of-range request      → 416 Range Not Satisfiable

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// StreamConfig
// ---------------------------------------------------------------------------

/// Configuration for the streaming subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Chunk size in bytes for streaming reads (default 256 KiB)
    pub chunk_size: usize,
    /// Maximum file size in bytes that will be streamed (default 10 GiB)
    pub max_file_bytes: u64,
    /// Enable response body compression for small non-media files (default false)
    pub enable_compression: bool,
    /// Path prefix that is the root of the served media directory
    pub media_root: String,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            chunk_size: 256 * 1024,                  // 256 KiB chunks
            max_file_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB max
            enable_compression: false,
            media_root: String::new(),
        }
    }
}

impl StreamConfig {
    /// Returns `true` if the media root is configured.
    pub fn is_valid(&self) -> bool {
        !self.media_root.is_empty()
    }
}

// ---------------------------------------------------------------------------
// StreamRequest — parsed Range header
// ---------------------------------------------------------------------------

/// Represents a parsed HTTP Range request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamRequest {
    /// Full file requested (no Range header present)
    Full,
    /// Range: bytes=start-end (both bounds explicit)
    Range { start: u64, end: u64 },
    /// Range: bytes=start- (from start to EOF)
    FromStart { start: u64 },
    /// Range: bytes=-suffix (last N bytes)
    Suffix { suffix: u64 },
}

impl StreamRequest {
    /// Returns `true` if a Range header was specified.
    pub fn is_range_request(&self) -> bool {
        !matches!(self, Self::Full)
    }

    /// Resolve this request to `(start, end)` given the total file `size`.
    ///
    /// Returns `None` if the range is unsatisfiable (start ≥ size, or suffix > size).
    pub fn resolve(&self, size: u64) -> Option<(u64, u64)> {
        match self {
            Self::Full => {
                if size == 0 {
                    None
                } else {
                    Some((0, size - 1))
                }
            }
            Self::Range { start, end } => {
                if *start >= size || *end >= size || *start > *end {
                    None
                } else {
                    Some((*start, *end))
                }
            }
            Self::FromStart { start } => {
                if *start >= size {
                    None
                } else {
                    Some((*start, size - 1))
                }
            }
            Self::Suffix { suffix } => {
                if *suffix == 0 || *suffix > size {
                    None
                } else {
                    Some((size - suffix, size - 1))
                }
            }
        }
    }

    /// Returns the number of bytes that will be transferred for this request
    /// given the total file `size`. Returns `None` if unsatisfiable.
    pub fn byte_count(&self, size: u64) -> Option<u64> {
        self.resolve(size).map(|(s, e)| e - s + 1)
    }
}

// ---------------------------------------------------------------------------
// StreamResponse — byte-range response descriptor
// ---------------------------------------------------------------------------

/// Metadata describing the HTTP response for a range request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamResponse {
    /// HTTP status code: 200 (full) or 206 (partial content)
    pub status_code: u16,
    /// Start byte offset (inclusive)
    pub start: u64,
    /// End byte offset (inclusive)
    pub end: u64,
    /// Total file size in bytes
    pub total_size: u64,
    /// MIME type of the file (e.g. `"audio/flac"`)
    pub content_type: String,
    /// Value for the `Content-Range` header (only set for 206 responses)
    pub content_range: Option<String>,
}

impl StreamResponse {
    /// Create a `StreamResponse` for a full file (200 OK).
    pub fn full(total_size: u64, content_type: &str) -> Self {
        Self {
            status_code: 200,
            start: 0,
            end: if total_size > 0 { total_size - 1 } else { 0 },
            total_size,
            content_type: content_type.to_string(),
            content_range: None,
        }
    }

    /// Create a `StreamResponse` for a partial range (206 Partial Content).
    pub fn partial(start: u64, end: u64, total_size: u64, content_type: &str) -> Self {
        let content_range = Some(format!("bytes {start}-{end}/{total_size}"));
        Self {
            status_code: 206,
            start,
            end,
            total_size,
            content_type: content_type.to_string(),
            content_range,
        }
    }

    /// The number of bytes in this response body.
    pub fn content_length(&self) -> u64 {
        if self.total_size == 0 {
            0
        } else {
            self.end - self.start + 1
        }
    }

    /// Returns `true` if this is a partial content response.
    pub fn is_partial(&self) -> bool {
        self.status_code == 206
    }
}

// ---------------------------------------------------------------------------
// StreamError
// ---------------------------------------------------------------------------

/// Failure modes for the streaming subsystem.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StreamError {
    /// The requested file was not found at the resolved path
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    /// The requested range extends beyond the file size (HTTP 416)
    #[error("Range not satisfiable: {range} for file of {size} bytes")]
    RangeNotSatisfiable { range: String, size: u64 },

    /// The Range header value is syntactically invalid
    #[error("Malformed Range header: {header}")]
    MalformedRange { header: String },

    /// The media root directory is not configured
    #[error("Media root is not configured")]
    MediaRootNotConfigured,

    /// Path traversal attempt detected (resolved path escapes media root)
    #[error("Path traversal denied: {path}")]
    PathTraversalDenied { path: String },

    /// File is too large to stream (exceeds max_file_bytes)
    #[error("File too large: {size} bytes (max {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },

    /// An I/O error occurred while reading the file
    #[error("I/O error streaming {path}: {message}")]
    IoError { path: String, message: String },
}

// ---------------------------------------------------------------------------
// RangeParser
// ---------------------------------------------------------------------------

/// Parses the value of an HTTP `Range: bytes=...` header.
pub struct RangeParser;

impl RangeParser {
    /// Parse an HTTP Range header value (the part after `"bytes="`).
    ///
    /// # Examples
    ///
    /// ```
    /// use mm_server::streaming::RangeParser;
    /// assert!(RangeParser::parse("bytes=0-1023").is_ok());
    /// assert!(RangeParser::parse("bytes=0-").is_ok());
    /// assert!(RangeParser::parse("bytes=-512").is_ok());
    /// assert!(RangeParser::parse("bytes=abc").is_err());
    /// ```
    pub fn parse(header: &str) -> Result<StreamRequest, StreamError> {
        // Validate and strip the "bytes=" prefix
        let spec = header
            .strip_prefix("bytes=")
            .ok_or_else(|| StreamError::MalformedRange {
                header: header.to_string(),
            })?;

        if let Some(suffix_str) = spec.strip_prefix('-') {
            // Suffix range: bytes=-N
            let n: u64 = suffix_str
                .parse()
                .map_err(|_| StreamError::MalformedRange {
                    header: header.to_string(),
                })?;
            return Ok(StreamRequest::Suffix { suffix: n });
        }

        let parts: Vec<&str> = spec.splitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(StreamError::MalformedRange {
                header: header.to_string(),
            });
        }

        let start: u64 = parts[0].parse().map_err(|_| StreamError::MalformedRange {
            header: header.to_string(),
        })?;

        if parts[1].is_empty() {
            // Open-ended: bytes=start-
            return Ok(StreamRequest::FromStart { start });
        }

        let end: u64 = parts[1].parse().map_err(|_| StreamError::MalformedRange {
            header: header.to_string(),
        })?;

        Ok(StreamRequest::Range { start, end })
    }
}

// ---------------------------------------------------------------------------
// MediaStreamer
// ---------------------------------------------------------------------------

/// Resolves a media file path and computes the `StreamResponse` for a
/// range request, enforcing the media root and path traversal checks.
pub struct MediaStreamer {
    /// Streaming configuration
    config: StreamConfig,
}

impl MediaStreamer {
    /// Create a `MediaStreamer` with the given config.
    pub fn new(config: StreamConfig) -> Result<Self, StreamError> {
        if !config.is_valid() {
            return Err(StreamError::MediaRootNotConfigured);
        }
        Ok(Self { config })
    }

    /// Infer the MIME type from the file extension.
    pub fn content_type(path: &str) -> &'static str {
        // Map common media extensions to MIME types
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".flac") {
            return "audio/flac";
        }
        if lower.ends_with(".mp3") {
            return "audio/mpeg";
        }
        if lower.ends_with(".m4a") {
            return "audio/mp4";
        }
        if lower.ends_with(".aac") {
            return "audio/aac";
        }
        if lower.ends_with(".ogg") {
            return "audio/ogg";
        }
        if lower.ends_with(".opus") {
            return "audio/opus";
        }
        if lower.ends_with(".wav") {
            return "audio/wav";
        }
        if lower.ends_with(".aiff") {
            return "audio/aiff";
        }
        if lower.ends_with(".mp4") {
            return "video/mp4";
        }
        if lower.ends_with(".mkv") {
            return "video/x-matroska";
        }
        if lower.ends_with(".mov") {
            return "video/quicktime";
        }
        if lower.ends_with(".avi") {
            return "video/x-msvideo";
        }
        if lower.ends_with(".webm") {
            return "video/webm";
        }
        if lower.ends_with(".m3u8") {
            return "application/vnd.apple.mpegurl";
        }
        "application/octet-stream"
    }

    /// Check whether `path` is safely within `media_root` (no path traversal).
    ///
    /// Returns `Err(StreamError::PathTraversalDenied)` if any path component
    /// resolves to `..` (parent directory), the path is absolute, or it
    /// contains null bytes or other control characters.
    ///
    /// Uses `std::path::Path::components()` to inspect normalised components
    /// rather than simple string matching, preventing bypass attempts such as
    /// `"....//etc/passwd"` or `"%2e%2e%2fpasswd"` (caller must URL-decode).
    pub fn is_safe_path(&self, relative_path: &str) -> Result<String, StreamError> {
        use std::path::{Component, Path};

        // Reject null bytes and control characters before any path logic
        if relative_path.bytes().any(|b| b == 0 || b < 0x20) {
            return Err(StreamError::PathTraversalDenied {
                path: relative_path.to_string(),
            });
        }

        // Strip leading path separator to make it relative
        let clean = relative_path.trim_start_matches('/');

        // Walk every normalised component of the path — reject Parent (`..`),
        // Prefix (Windows drive letters), and RootDir (absolute paths).
        for component in Path::new(clean).components() {
            match component {
                Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                    return Err(StreamError::PathTraversalDenied {
                        path: relative_path.to_string(),
                    });
                }
                Component::CurDir | Component::Normal(_) => {}
            }
        }

        let full_path = format!("{}/{}", self.media_root_trimmed(), clean);
        Ok(full_path)
    }

    /// Returns the media root path without a trailing slash.
    fn media_root_trimmed(&self) -> &str {
        self.config.media_root.trim_end_matches('/')
    }

    /// Compute the `StreamResponse` for a request.
    ///
    /// For M10 this is a stub: it validates the range, infers the MIME type,
    /// and returns the response descriptor without opening the real file.
    /// Production implementation performs `tokio::fs::metadata()` to get size.
    pub fn prepare_response(
        &self,
        relative_path: &str,
        range: &StreamRequest,
        file_size: u64, // caller provides this (from tokio::fs::metadata)
    ) -> Result<StreamResponse, StreamError> {
        // Security: reject path traversal
        let full_path = self.is_safe_path(relative_path)?;

        // Enforce file size limit
        if file_size > self.config.max_file_bytes {
            return Err(StreamError::FileTooLarge {
                size: file_size,
                max: self.config.max_file_bytes,
            });
        }

        let content_type = Self::content_type(&full_path);

        // Resolve the range to byte offsets
        let resolved =
            range
                .resolve(file_size)
                .ok_or_else(|| StreamError::RangeNotSatisfiable {
                    range: format!("{range:?}"),
                    size: file_size,
                })?;

        if range.is_range_request() {
            Ok(StreamResponse::partial(
                resolved.0,
                resolved.1,
                file_size,
                content_type,
            ))
        } else {
            Ok(StreamResponse::full(file_size, content_type))
        }
    }

    /// Returns the chunk size in bytes.
    pub fn chunk_size(&self) -> usize {
        self.config.chunk_size
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn streamer() -> MediaStreamer {
        MediaStreamer::new(StreamConfig {
            media_root: "/media".into(),
            ..StreamConfig::default()
        })
        .unwrap()
    }

    // --- StreamConfig ---

    #[test]
    fn stream_config_defaults() {
        let cfg = StreamConfig::default();
        assert_eq!(cfg.chunk_size, 256 * 1024);
        assert!(!cfg.is_valid()); // empty media root
    }

    #[test]
    fn stream_config_valid_when_root_set() {
        let cfg = StreamConfig {
            media_root: "/media".into(),
            ..StreamConfig::default()
        };
        assert!(cfg.is_valid());
    }

    // --- RangeParser ---

    #[test]
    fn parse_full_range() {
        let r = RangeParser::parse("bytes=0-1023").unwrap();
        assert_eq!(
            r,
            StreamRequest::Range {
                start: 0,
                end: 1023
            }
        );
    }

    #[test]
    fn parse_open_ended_range() {
        let r = RangeParser::parse("bytes=512-").unwrap();
        assert_eq!(r, StreamRequest::FromStart { start: 512 });
    }

    #[test]
    fn parse_suffix_range() {
        let r = RangeParser::parse("bytes=-1024").unwrap();
        assert_eq!(r, StreamRequest::Suffix { suffix: 1024 });
    }

    #[test]
    fn parse_malformed_range_returns_error() {
        assert!(RangeParser::parse("something=0-10").is_err());
        assert!(RangeParser::parse("bytes=abc-def").is_err());
    }

    // --- StreamRequest::resolve ---

    #[test]
    fn resolve_full_request() {
        let (s, e) = StreamRequest::Full.resolve(1000).unwrap();
        assert_eq!((s, e), (0, 999));
    }

    #[test]
    fn resolve_range_request() {
        let (s, e) = StreamRequest::Range {
            start: 100,
            end: 199,
        }
        .resolve(1000)
        .unwrap();
        assert_eq!((s, e), (100, 199));
    }

    #[test]
    fn resolve_out_of_range_returns_none() {
        let r = StreamRequest::Range {
            start: 900,
            end: 1100,
        }
        .resolve(1000);
        assert!(r.is_none(), "end exceeds file size");
    }

    #[test]
    fn resolve_suffix_range() {
        let (s, e) = StreamRequest::Suffix { suffix: 100 }.resolve(1000).unwrap();
        assert_eq!((s, e), (900, 999));
    }

    #[test]
    fn byte_count_for_range() {
        let count = StreamRequest::Range { start: 0, end: 99 }
            .byte_count(1000)
            .unwrap();
        assert_eq!(count, 100);
    }

    // --- StreamResponse ---

    #[test]
    fn full_response_status_200() {
        let resp = StreamResponse::full(5000, "audio/flac");
        assert_eq!(resp.status_code, 200);
        assert!(!resp.is_partial());
        assert_eq!(resp.content_length(), 5000);
        assert!(resp.content_range.is_none());
    }

    #[test]
    fn partial_response_status_206() {
        let resp = StreamResponse::partial(0, 1023, 5000, "audio/flac");
        assert_eq!(resp.status_code, 206);
        assert!(resp.is_partial());
        assert_eq!(resp.content_length(), 1024);
        assert_eq!(resp.content_range.as_deref(), Some("bytes 0-1023/5000"));
    }

    // --- MediaStreamer ---

    #[test]
    fn new_rejects_unconfigured_media_root() {
        assert!(MediaStreamer::new(StreamConfig::default()).is_err());
    }

    #[test]
    fn content_type_mapping() {
        assert_eq!(
            MediaStreamer::content_type("/music/track.flac"),
            "audio/flac"
        );
        assert_eq!(MediaStreamer::content_type("/video/film.mp4"), "video/mp4");
        assert_eq!(
            MediaStreamer::content_type("/misc/data.bin"),
            "application/octet-stream"
        );
        assert_eq!(
            MediaStreamer::content_type("/music/album.mp3"),
            "audio/mpeg"
        );
        assert_eq!(
            MediaStreamer::content_type("/stream/list.m3u8"),
            "application/vnd.apple.mpegurl"
        );
    }

    #[test]
    fn is_safe_path_blocks_traversal() {
        let err = streamer().is_safe_path("../../etc/passwd").unwrap_err();
        assert!(matches!(err, StreamError::PathTraversalDenied { .. }));
    }

    #[test]
    fn is_safe_path_blocks_dotdot_prefix() {
        // Path that starts with leading slashes then dotdot
        let err = streamer().is_safe_path("/../../etc/shadow").unwrap_err();
        assert!(matches!(err, StreamError::PathTraversalDenied { .. }));
    }

    #[test]
    fn is_safe_path_blocks_null_byte() {
        let err = streamer().is_safe_path("music/\x00evil").unwrap_err();
        assert!(matches!(err, StreamError::PathTraversalDenied { .. }));
    }

    #[test]
    fn is_safe_path_allows_normal_path() {
        let result = streamer().is_safe_path("music/song.flac");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("/media/music/song.flac"));
    }

    #[test]
    fn is_safe_path_allows_nested_normal_path() {
        let result = streamer().is_safe_path("albums/2024/track01.mp3");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("albums/2024/track01.mp3"));
    }

    #[test]
    fn prepare_response_full() {
        let resp = streamer()
            .prepare_response("music/song.flac", &StreamRequest::Full, 1_000_000)
            .unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.total_size, 1_000_000);
        assert_eq!(resp.content_type, "audio/flac");
    }

    #[test]
    fn prepare_response_partial() {
        let range = StreamRequest::Range { start: 0, end: 255 };
        let resp = streamer()
            .prepare_response("music/song.mp3", &range, 10_000)
            .unwrap();
        assert_eq!(resp.status_code, 206);
        assert_eq!(resp.content_length(), 256);
    }

    #[test]
    fn prepare_response_range_not_satisfiable() {
        let range = StreamRequest::Range {
            start: 5000,
            end: 6000,
        };
        let err = streamer()
            .prepare_response("music/song.flac", &range, 1000)
            .unwrap_err();
        assert!(matches!(err, StreamError::RangeNotSatisfiable { .. }));
    }

    #[test]
    fn prepare_response_file_too_large() {
        let cfg = StreamConfig {
            media_root: "/media".into(),
            max_file_bytes: 100,
            ..StreamConfig::default()
        };
        let s = MediaStreamer::new(cfg).unwrap();
        let err = s
            .prepare_response("big.mkv", &StreamRequest::Full, 200)
            .unwrap_err();
        assert!(matches!(err, StreamError::FileTooLarge { .. }));
    }

    #[test]
    fn chunk_size_returned_correctly() {
        assert_eq!(streamer().chunk_size(), 256 * 1024);
    }

    #[test]
    fn stream_error_display_messages() {
        let e = StreamError::FileNotFound {
            path: "/a.mp3".into(),
        };
        assert!(e.to_string().contains("/a.mp3"));
        let e = StreamError::PathTraversalDenied {
            path: "../etc".into(),
        };
        assert!(e.to_string().contains("../etc"));
    }
}
