// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Media Classification Engine
//
// Implements a 4-level media classification hierarchy:
//   Group  → broad category (Audio, Video, Image, Document, Archive, Unknown)
//   Format → specific file format (MP3, FLAC, MKV, PNG, PDF, …)
//   Class  → content type / purpose (Music, Podcast, Movie, TVShow, …)
//   Quality → encoding quality tier (Lossless, HiRes, Lossy320, …)
//
// Classification can be driven by file extension, full file path, or
// enriched later with metadata (bitrate, codec, tags).
//
// License: GPL-2.0-or-later

use std::fmt; // Display trait for human-readable enum output
use std::path::Path; // Path handling for classify_by_path

use serde::{Deserialize, Serialize}; // Serialization support for all enums

use crate::error::{MmError, MmResult}; // Unified error type

// ─────────────────────────────────────────────────────────────────────
// Level 1: MediaGroup — broad media category
// ─────────────────────────────────────────────────────────────────────

/// Top-level grouping for media files.
/// Every recognised format maps to exactly one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaGroup {
    /// Sound-only media (music, podcasts, audiobooks, sound effects)
    Audio,
    /// Moving-picture media (movies, TV, concerts, home videos)
    Video,
    /// Still-picture media (photos, illustrations, icons)
    Image,
    /// Textual / document media (PDFs, e-books, office docs)
    Document,
    /// Compressed archives and disk images
    Archive,
    /// File type could not be determined
    Unknown,
}

impl fmt::Display for MediaGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Human-readable group names for UI display and logging
        match self {
            Self::Audio => write!(f, "Audio"),
            Self::Video => write!(f, "Video"),
            Self::Image => write!(f, "Image"),
            Self::Document => write!(f, "Document"),
            Self::Archive => write!(f, "Archive"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Level 2: MediaFormat — specific file format / codec container
// ─────────────────────────────────────────────────────────────────────

/// Specific file format identified by extension and/or magic bytes.
/// Each variant knows its canonical extension and MIME type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaFormat {
    // ── Audio formats ──────────────────────────────────────────────
    /// MPEG-1 Audio Layer III
    MP3,
    /// Free Lossless Audio Codec
    FLAC,
    /// Advanced Audio Coding (MPEG-4 audio)
    AAC,
    /// Waveform Audio File Format (uncompressed PCM)
    WAV,
    /// Audio Interchange File Format (Apple uncompressed)
    AIFF,
    /// Apple Lossless Audio Codec
    ALAC,
    /// Ogg Vorbis container
    OGG,
    /// Opus audio in Ogg container
    OPUS,
    /// Windows Media Audio
    WMA,
    /// MPEG-4 audio container (iTunes, audiobooks)
    M4A,
    /// MPEG-4 audiobook container (Apple)
    M4B,
    /// Monkey's Audio lossless
    APE,
    /// WavPack hybrid/lossless
    WV,
    /// Musepack audio
    MPC,
    /// True Audio lossless
    TTA,
    /// Direct Stream Digital (DSD)
    DSF,
    /// DSD Interchange File Format
    DFF,
    /// Adaptive Multi-Rate (mobile voice)
    AMR,
    /// Au / SND audio (Sun/NeXT)
    AU,
    /// RealAudio
    RA,
    /// MIDI sequence
    MID,
    /// SPC700 (SNES audio)
    SPC,
    /// Tracker module format
    MOD,
    /// Scream Tracker 3 module
    S3M,
    /// Extended Module
    XM,
    /// Impulse Tracker module
    IT,
    /// Core Audio Format (Apple)
    CAF,
    /// Audio codec 3 / Dolby Digital
    AC3,
    /// DTS Coherent Acoustics
    DTS,

    // ── Video formats ──────────────────────────────────────────────
    /// Matroska video container
    MKV,
    /// MPEG-4 Part 14 container
    MP4,
    /// Audio Video Interleave (Microsoft)
    AVI,
    /// QuickTime movie container (Apple)
    MOV,
    /// Windows Media Video
    WMV,
    /// WebM (VP8/VP9/AV1 in Matroska)
    WEBM,
    /// Flash Video
    FLV,
    /// MPEG-4 video (iTunes)
    M4V,
    /// MPEG-2 Transport Stream
    TS,
    /// MPEG Program Stream
    MPG,
    /// MPEG-2 video elementary stream
    MPEG,
    /// 3GPP mobile video
    ThreeGP,
    /// RealMedia container
    RM,
    /// RealMedia variable bitrate
    RMVB,
    /// Video Object (DVD)
    VOB,
    /// Ogg video container (Theora)
    OGV,
    /// Advanced Systems Format (Microsoft)
    ASF,
    /// Material Exchange Format (broadcast)
    MXF,
    /// Matroska 3D / stereoscopic
    MK3D,
    /// Nullsoft Streaming Video
    NSV,
    /// Flash video (alternative extension)
    F4V,

    // ── Image formats ──────────────────────────────────────────────
    /// JPEG / JFIF image
    JPG,
    /// Portable Network Graphics
    PNG,
    /// Graphics Interchange Format
    GIF,
    /// Windows Bitmap
    BMP,
    /// Tagged Image File Format
    TIFF,
    /// WebP image (Google)
    WEBP,
    /// Scalable Vector Graphics
    SVG,
    /// High Efficiency Image Format (HEVC)
    HEIF,
    /// High Efficiency Image Coding
    HEIC,
    /// AV1 Image File Format
    AVIF,
    /// Adobe Photoshop Document
    PSD,
    /// Truevision TGA / TARGA
    TGA,
    /// Windows Icon
    ICO,
    /// Canon Raw version 2
    CR2,
    /// Nikon Electronic Format (raw)
    NEF,
    /// Adobe Digital Negative (raw)
    DNG,
    /// Sony Alpha Raw
    ARW,
    /// Olympus Raw Format
    ORF,
    /// Fuji Raw File
    RAF,
    /// Panasonic Raw
    RW2,
    /// Raw image (generic)
    RAW,
    /// OpenEXR high-dynamic-range
    EXR,
    /// Radiance HDR
    HDR,
    /// Portable Pixmap (Netpbm)
    PPM,
    /// Portable Graymap
    PGM,
    /// JPEG 2000
    JP2,
    /// JPEG XL
    JXL,

    // ── Document formats ───────────────────────────────────────────
    /// Portable Document Format
    PDF,
    /// Electronic Publication (e-book)
    EPUB,
    /// Amazon Kindle e-book (Mobipocket)
    MOBI,
    /// Microsoft Word (legacy binary)
    DOC,
    /// Microsoft Word (Office Open XML)
    DOCX,
    /// Microsoft Excel (legacy binary)
    XLS,
    /// Microsoft Excel (Office Open XML)
    XLSX,
    /// Microsoft PowerPoint (legacy binary)
    PPT,
    /// Microsoft PowerPoint (Office Open XML)
    PPTX,
    /// OpenDocument Text
    ODT,
    /// OpenDocument Spreadsheet
    ODS,
    /// OpenDocument Presentation
    ODP,
    /// Rich Text Format
    RTF,
    /// Plain text
    TXT,
    /// Comma-separated values
    CSV,
    /// HyperText Markup Language
    HTML,
    /// Extensible Markup Language
    XML,
    /// JavaScript Object Notation
    JSON,
    /// YAML Ain't Markup Language
    YAML,
    /// Markdown
    MD,
    /// LaTeX source
    TEX,
    /// DjVu scanned-document format
    DJVU,
    /// Comic Book Archive (ZIP)
    CBZ,
    /// Comic Book Archive (RAR)
    CBR,
    /// FictionBook e-book (XML)
    FB2,
    /// Subtitle file (SubRip)
    SRT,
    /// Advanced SubStation Alpha subtitles
    ASS,
    /// WebVTT subtitles
    VTT,

    // ── Archive formats ────────────────────────────────────────────
    /// ZIP archive
    ZIP,
    /// RAR archive (WinRAR)
    RAR,
    /// 7-Zip archive
    SevenZ,
    /// Tape Archive
    TAR,
    /// Gzip compressed
    GZ,
    /// Bzip2 compressed
    BZ2,
    /// XZ compressed (LZMA2)
    XZ,
    /// Zstandard compressed
    ZST,
    /// LZ4 compressed
    LZ4,
    /// ISO 9660 disk image
    ISO,
    /// Apple Disk Image
    DMG,
    /// Windows Installer package
    MSI,
    /// Debian package
    DEB,
    /// RPM Package Manager
    RPM,
    /// macOS installer package
    PKG,
    /// Java Archive
    JAR,
    /// Android Package
    APK,

    // ── Catch-all ──────────────────────────────────────────────────
    /// Format could not be identified
    UnknownFormat,
}

impl fmt::Display for MediaFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Human-readable format names (matches common industry names)
        match self {
            // Audio
            Self::MP3 => write!(f, "MP3"),
            Self::FLAC => write!(f, "FLAC"),
            Self::AAC => write!(f, "AAC"),
            Self::WAV => write!(f, "WAV"),
            Self::AIFF => write!(f, "AIFF"),
            Self::ALAC => write!(f, "ALAC"),
            Self::OGG => write!(f, "Ogg Vorbis"),
            Self::OPUS => write!(f, "Opus"),
            Self::WMA => write!(f, "WMA"),
            Self::M4A => write!(f, "M4A"),
            Self::M4B => write!(f, "M4B"),
            Self::APE => write!(f, "Monkey's Audio"),
            Self::WV => write!(f, "WavPack"),
            Self::MPC => write!(f, "Musepack"),
            Self::TTA => write!(f, "True Audio"),
            Self::DSF => write!(f, "DSD (DSF)"),
            Self::DFF => write!(f, "DSD (DFF)"),
            Self::AMR => write!(f, "AMR"),
            Self::AU => write!(f, "Au/SND"),
            Self::RA => write!(f, "RealAudio"),
            Self::MID => write!(f, "MIDI"),
            Self::SPC => write!(f, "SPC"),
            Self::MOD => write!(f, "MOD"),
            Self::S3M => write!(f, "S3M"),
            Self::XM => write!(f, "XM"),
            Self::IT => write!(f, "IT"),
            Self::CAF => write!(f, "Core Audio Format"),
            Self::AC3 => write!(f, "AC-3 / Dolby Digital"),
            Self::DTS => write!(f, "DTS"),
            // Video
            Self::MKV => write!(f, "Matroska Video"),
            Self::MP4 => write!(f, "MP4"),
            Self::AVI => write!(f, "AVI"),
            Self::MOV => write!(f, "QuickTime"),
            Self::WMV => write!(f, "WMV"),
            Self::WEBM => write!(f, "WebM"),
            Self::FLV => write!(f, "Flash Video"),
            Self::M4V => write!(f, "M4V"),
            Self::TS => write!(f, "MPEG-TS"),
            Self::MPG => write!(f, "MPEG"),
            Self::MPEG => write!(f, "MPEG-2"),
            Self::ThreeGP => write!(f, "3GP"),
            Self::RM => write!(f, "RealMedia"),
            Self::RMVB => write!(f, "RealMedia VBR"),
            Self::VOB => write!(f, "DVD Video Object"),
            Self::OGV => write!(f, "Ogg Video"),
            Self::ASF => write!(f, "ASF"),
            Self::MXF => write!(f, "MXF"),
            Self::MK3D => write!(f, "Matroska 3D"),
            Self::NSV => write!(f, "NSV"),
            Self::F4V => write!(f, "Flash Video (F4V)"),
            // Image
            Self::JPG => write!(f, "JPEG"),
            Self::PNG => write!(f, "PNG"),
            Self::GIF => write!(f, "GIF"),
            Self::BMP => write!(f, "BMP"),
            Self::TIFF => write!(f, "TIFF"),
            Self::WEBP => write!(f, "WebP"),
            Self::SVG => write!(f, "SVG"),
            Self::HEIF => write!(f, "HEIF"),
            Self::HEIC => write!(f, "HEIC"),
            Self::AVIF => write!(f, "AVIF"),
            Self::PSD => write!(f, "Photoshop"),
            Self::TGA => write!(f, "TGA"),
            Self::ICO => write!(f, "ICO"),
            Self::CR2 => write!(f, "Canon RAW"),
            Self::NEF => write!(f, "Nikon RAW"),
            Self::DNG => write!(f, "DNG"),
            Self::ARW => write!(f, "Sony RAW"),
            Self::ORF => write!(f, "Olympus RAW"),
            Self::RAF => write!(f, "Fuji RAW"),
            Self::RW2 => write!(f, "Panasonic RAW"),
            Self::RAW => write!(f, "RAW"),
            Self::EXR => write!(f, "OpenEXR"),
            Self::HDR => write!(f, "Radiance HDR"),
            Self::PPM => write!(f, "PPM"),
            Self::PGM => write!(f, "PGM"),
            Self::JP2 => write!(f, "JPEG 2000"),
            Self::JXL => write!(f, "JPEG XL"),
            // Document
            Self::PDF => write!(f, "PDF"),
            Self::EPUB => write!(f, "EPUB"),
            Self::MOBI => write!(f, "Kindle"),
            Self::DOC => write!(f, "Word (DOC)"),
            Self::DOCX => write!(f, "Word (DOCX)"),
            Self::XLS => write!(f, "Excel (XLS)"),
            Self::XLSX => write!(f, "Excel (XLSX)"),
            Self::PPT => write!(f, "PowerPoint (PPT)"),
            Self::PPTX => write!(f, "PowerPoint (PPTX)"),
            Self::ODT => write!(f, "OpenDocument Text"),
            Self::ODS => write!(f, "OpenDocument Spreadsheet"),
            Self::ODP => write!(f, "OpenDocument Presentation"),
            Self::RTF => write!(f, "Rich Text"),
            Self::TXT => write!(f, "Plain Text"),
            Self::CSV => write!(f, "CSV"),
            Self::HTML => write!(f, "HTML"),
            Self::XML => write!(f, "XML"),
            Self::JSON => write!(f, "JSON"),
            Self::YAML => write!(f, "YAML"),
            Self::MD => write!(f, "Markdown"),
            Self::TEX => write!(f, "LaTeX"),
            Self::DJVU => write!(f, "DjVu"),
            Self::CBZ => write!(f, "CBZ"),
            Self::CBR => write!(f, "CBR"),
            Self::FB2 => write!(f, "FictionBook"),
            Self::SRT => write!(f, "SubRip"),
            Self::ASS => write!(f, "ASS Subtitles"),
            Self::VTT => write!(f, "WebVTT"),
            // Archive
            Self::ZIP => write!(f, "ZIP"),
            Self::RAR => write!(f, "RAR"),
            Self::SevenZ => write!(f, "7-Zip"),
            Self::TAR => write!(f, "TAR"),
            Self::GZ => write!(f, "Gzip"),
            Self::BZ2 => write!(f, "Bzip2"),
            Self::XZ => write!(f, "XZ"),
            Self::ZST => write!(f, "Zstandard"),
            Self::LZ4 => write!(f, "LZ4"),
            Self::ISO => write!(f, "ISO"),
            Self::DMG => write!(f, "Apple Disk Image"),
            Self::MSI => write!(f, "Windows Installer"),
            Self::DEB => write!(f, "Debian Package"),
            Self::RPM => write!(f, "RPM Package"),
            Self::PKG => write!(f, "macOS Package"),
            Self::JAR => write!(f, "Java Archive"),
            Self::APK => write!(f, "Android Package"),
            // Unknown
            Self::UnknownFormat => write!(f, "Unknown"),
        }
    }
}

impl MediaFormat {
    /// Returns the canonical file extension (without leading dot) for this format.
    pub fn extension(&self) -> &str {
        match self {
            // Audio
            Self::MP3 => "mp3",
            Self::FLAC => "flac",
            Self::AAC => "aac",
            Self::WAV => "wav",
            Self::AIFF => "aiff",
            Self::ALAC => "m4a", // ALAC uses the M4A container
            Self::OGG => "ogg",
            Self::OPUS => "opus",
            Self::WMA => "wma",
            Self::M4A => "m4a",
            Self::M4B => "m4b",
            Self::APE => "ape",
            Self::WV => "wv",
            Self::MPC => "mpc",
            Self::TTA => "tta",
            Self::DSF => "dsf",
            Self::DFF => "dff",
            Self::AMR => "amr",
            Self::AU => "au",
            Self::RA => "ra",
            Self::MID => "mid",
            Self::SPC => "spc",
            Self::MOD => "mod",
            Self::S3M => "s3m",
            Self::XM => "xm",
            Self::IT => "it",
            Self::CAF => "caf",
            Self::AC3 => "ac3",
            Self::DTS => "dts",
            // Video
            Self::MKV => "mkv",
            Self::MP4 => "mp4",
            Self::AVI => "avi",
            Self::MOV => "mov",
            Self::WMV => "wmv",
            Self::WEBM => "webm",
            Self::FLV => "flv",
            Self::M4V => "m4v",
            Self::TS => "ts",
            Self::MPG => "mpg",
            Self::MPEG => "mpeg",
            Self::ThreeGP => "3gp",
            Self::RM => "rm",
            Self::RMVB => "rmvb",
            Self::VOB => "vob",
            Self::OGV => "ogv",
            Self::ASF => "asf",
            Self::MXF => "mxf",
            Self::MK3D => "mk3d",
            Self::NSV => "nsv",
            Self::F4V => "f4v",
            // Image
            Self::JPG => "jpg",
            Self::PNG => "png",
            Self::GIF => "gif",
            Self::BMP => "bmp",
            Self::TIFF => "tiff",
            Self::WEBP => "webp",
            Self::SVG => "svg",
            Self::HEIF => "heif",
            Self::HEIC => "heic",
            Self::AVIF => "avif",
            Self::PSD => "psd",
            Self::TGA => "tga",
            Self::ICO => "ico",
            Self::CR2 => "cr2",
            Self::NEF => "nef",
            Self::DNG => "dng",
            Self::ARW => "arw",
            Self::ORF => "orf",
            Self::RAF => "raf",
            Self::RW2 => "rw2",
            Self::RAW => "raw",
            Self::EXR => "exr",
            Self::HDR => "hdr",
            Self::PPM => "ppm",
            Self::PGM => "pgm",
            Self::JP2 => "jp2",
            Self::JXL => "jxl",
            // Document
            Self::PDF => "pdf",
            Self::EPUB => "epub",
            Self::MOBI => "mobi",
            Self::DOC => "doc",
            Self::DOCX => "docx",
            Self::XLS => "xls",
            Self::XLSX => "xlsx",
            Self::PPT => "ppt",
            Self::PPTX => "pptx",
            Self::ODT => "odt",
            Self::ODS => "ods",
            Self::ODP => "odp",
            Self::RTF => "rtf",
            Self::TXT => "txt",
            Self::CSV => "csv",
            Self::HTML => "html",
            Self::XML => "xml",
            Self::JSON => "json",
            Self::YAML => "yaml",
            Self::MD => "md",
            Self::TEX => "tex",
            Self::DJVU => "djvu",
            Self::CBZ => "cbz",
            Self::CBR => "cbr",
            Self::FB2 => "fb2",
            Self::SRT => "srt",
            Self::ASS => "ass",
            Self::VTT => "vtt",
            // Archive
            Self::ZIP => "zip",
            Self::RAR => "rar",
            Self::SevenZ => "7z",
            Self::TAR => "tar",
            Self::GZ => "gz",
            Self::BZ2 => "bz2",
            Self::XZ => "xz",
            Self::ZST => "zst",
            Self::LZ4 => "lz4",
            Self::ISO => "iso",
            Self::DMG => "dmg",
            Self::MSI => "msi",
            Self::DEB => "deb",
            Self::RPM => "rpm",
            Self::PKG => "pkg",
            Self::JAR => "jar",
            Self::APK => "apk",
            // Unknown
            Self::UnknownFormat => "",
        }
    }

    /// Returns the IANA MIME type for this format.
    /// Falls back to "application/octet-stream" for unknown formats.
    pub fn mime_type(&self) -> &str {
        match self {
            // Audio
            Self::MP3 => "audio/mpeg",
            Self::FLAC => "audio/flac",
            Self::AAC => "audio/aac",
            Self::WAV => "audio/wav",
            Self::AIFF => "audio/aiff",
            Self::ALAC => "audio/mp4",
            Self::OGG => "audio/ogg",
            Self::OPUS => "audio/opus",
            Self::WMA => "audio/x-ms-wma",
            Self::M4A => "audio/mp4",
            Self::M4B => "audio/mp4",
            Self::APE => "audio/x-ape",
            Self::WV => "audio/x-wavpack",
            Self::MPC => "audio/x-musepack",
            Self::TTA => "audio/x-tta",
            Self::DSF => "audio/x-dsf",
            Self::DFF => "audio/x-dff",
            Self::AMR => "audio/amr",
            Self::AU => "audio/basic",
            Self::RA => "audio/x-realaudio",
            Self::MID => "audio/midi",
            Self::SPC => "audio/x-spc",
            Self::MOD => "audio/x-mod",
            Self::S3M => "audio/x-s3m",
            Self::XM => "audio/x-xm",
            Self::IT => "audio/x-it",
            Self::CAF => "audio/x-caf",
            Self::AC3 => "audio/ac3",
            Self::DTS => "audio/vnd.dts",
            // Video
            Self::MKV => "video/x-matroska",
            Self::MP4 => "video/mp4",
            Self::AVI => "video/x-msvideo",
            Self::MOV => "video/quicktime",
            Self::WMV => "video/x-ms-wmv",
            Self::WEBM => "video/webm",
            Self::FLV => "video/x-flv",
            Self::M4V => "video/mp4",
            Self::TS => "video/mp2t",
            Self::MPG => "video/mpeg",
            Self::MPEG => "video/mpeg",
            Self::ThreeGP => "video/3gpp",
            Self::RM => "application/vnd.rn-realmedia",
            Self::RMVB => "application/vnd.rn-realmedia-vbr",
            Self::VOB => "video/x-ms-vob",
            Self::OGV => "video/ogg",
            Self::ASF => "video/x-ms-asf",
            Self::MXF => "application/mxf",
            Self::MK3D => "video/x-matroska-3d",
            Self::NSV => "video/x-nsv",
            Self::F4V => "video/mp4",
            // Image
            Self::JPG => "image/jpeg",
            Self::PNG => "image/png",
            Self::GIF => "image/gif",
            Self::BMP => "image/bmp",
            Self::TIFF => "image/tiff",
            Self::WEBP => "image/webp",
            Self::SVG => "image/svg+xml",
            Self::HEIF => "image/heif",
            Self::HEIC => "image/heic",
            Self::AVIF => "image/avif",
            Self::PSD => "image/vnd.adobe.photoshop",
            Self::TGA => "image/x-tga",
            Self::ICO => "image/x-icon",
            Self::CR2 => "image/x-canon-cr2",
            Self::NEF => "image/x-nikon-nef",
            Self::DNG => "image/x-adobe-dng",
            Self::ARW => "image/x-sony-arw",
            Self::ORF => "image/x-olympus-orf",
            Self::RAF => "image/x-fuji-raf",
            Self::RW2 => "image/x-panasonic-rw2",
            Self::RAW => "image/x-raw",
            Self::EXR => "image/x-exr",
            Self::HDR => "image/vnd.radiance",
            Self::PPM => "image/x-portable-pixmap",
            Self::PGM => "image/x-portable-graymap",
            Self::JP2 => "image/jp2",
            Self::JXL => "image/jxl",
            // Document
            Self::PDF => "application/pdf",
            Self::EPUB => "application/epub+zip",
            Self::MOBI => "application/x-mobipocket-ebook",
            Self::DOC => "application/msword",
            Self::DOCX => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::XLS => "application/vnd.ms-excel",
            Self::XLSX => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::PPT => "application/vnd.ms-powerpoint",
            Self::PPTX => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::ODT => "application/vnd.oasis.opendocument.text",
            Self::ODS => "application/vnd.oasis.opendocument.spreadsheet",
            Self::ODP => "application/vnd.oasis.opendocument.presentation",
            Self::RTF => "application/rtf",
            Self::TXT => "text/plain",
            Self::CSV => "text/csv",
            Self::HTML => "text/html",
            Self::XML => "application/xml",
            Self::JSON => "application/json",
            Self::YAML => "application/x-yaml",
            Self::MD => "text/markdown",
            Self::TEX => "application/x-tex",
            Self::DJVU => "image/vnd.djvu",
            Self::CBZ => "application/vnd.comicbook+zip",
            Self::CBR => "application/vnd.comicbook-rar",
            Self::FB2 => "application/x-fictionbook+xml",
            Self::SRT => "application/x-subrip",
            Self::ASS => "text/x-ssa",
            Self::VTT => "text/vtt",
            // Archive
            Self::ZIP => "application/zip",
            Self::RAR => "application/vnd.rar",
            Self::SevenZ => "application/x-7z-compressed",
            Self::TAR => "application/x-tar",
            Self::GZ => "application/gzip",
            Self::BZ2 => "application/x-bzip2",
            Self::XZ => "application/x-xz",
            Self::ZST => "application/zstd",
            Self::LZ4 => "application/x-lz4",
            Self::ISO => "application/x-iso9660-image",
            Self::DMG => "application/x-apple-diskimage",
            Self::MSI => "application/x-msi",
            Self::DEB => "application/vnd.debian.binary-package",
            Self::RPM => "application/x-rpm",
            Self::PKG => "application/x-newton-compatible-pkg",
            Self::JAR => "application/java-archive",
            Self::APK => "application/vnd.android.package-archive",
            // Unknown
            Self::UnknownFormat => "application/octet-stream",
        }
    }

    /// Returns the `MediaGroup` that this format belongs to.
    pub fn group(&self) -> MediaGroup {
        match self {
            // Audio formats
            Self::MP3
            | Self::FLAC
            | Self::AAC
            | Self::WAV
            | Self::AIFF
            | Self::ALAC
            | Self::OGG
            | Self::OPUS
            | Self::WMA
            | Self::M4A
            | Self::M4B
            | Self::APE
            | Self::WV
            | Self::MPC
            | Self::TTA
            | Self::DSF
            | Self::DFF
            | Self::AMR
            | Self::AU
            | Self::RA
            | Self::MID
            | Self::SPC
            | Self::MOD
            | Self::S3M
            | Self::XM
            | Self::IT
            | Self::CAF
            | Self::AC3
            | Self::DTS => MediaGroup::Audio,

            // Video formats
            Self::MKV
            | Self::MP4
            | Self::AVI
            | Self::MOV
            | Self::WMV
            | Self::WEBM
            | Self::FLV
            | Self::M4V
            | Self::TS
            | Self::MPG
            | Self::MPEG
            | Self::ThreeGP
            | Self::RM
            | Self::RMVB
            | Self::VOB
            | Self::OGV
            | Self::ASF
            | Self::MXF
            | Self::MK3D
            | Self::NSV
            | Self::F4V => MediaGroup::Video,

            // Image formats
            Self::JPG
            | Self::PNG
            | Self::GIF
            | Self::BMP
            | Self::TIFF
            | Self::WEBP
            | Self::SVG
            | Self::HEIF
            | Self::HEIC
            | Self::AVIF
            | Self::PSD
            | Self::TGA
            | Self::ICO
            | Self::CR2
            | Self::NEF
            | Self::DNG
            | Self::ARW
            | Self::ORF
            | Self::RAF
            | Self::RW2
            | Self::RAW
            | Self::EXR
            | Self::HDR
            | Self::PPM
            | Self::PGM
            | Self::JP2
            | Self::JXL => MediaGroup::Image,

            // Document formats
            Self::PDF
            | Self::EPUB
            | Self::MOBI
            | Self::DOC
            | Self::DOCX
            | Self::XLS
            | Self::XLSX
            | Self::PPT
            | Self::PPTX
            | Self::ODT
            | Self::ODS
            | Self::ODP
            | Self::RTF
            | Self::TXT
            | Self::CSV
            | Self::HTML
            | Self::XML
            | Self::JSON
            | Self::YAML
            | Self::MD
            | Self::TEX
            | Self::DJVU
            | Self::CBZ
            | Self::CBR
            | Self::FB2
            | Self::SRT
            | Self::ASS
            | Self::VTT => MediaGroup::Document,

            // Archive formats
            Self::ZIP
            | Self::RAR
            | Self::SevenZ
            | Self::TAR
            | Self::GZ
            | Self::BZ2
            | Self::XZ
            | Self::ZST
            | Self::LZ4
            | Self::ISO
            | Self::DMG
            | Self::MSI
            | Self::DEB
            | Self::RPM
            | Self::PKG
            | Self::JAR
            | Self::APK => MediaGroup::Archive,

            // Unknown
            Self::UnknownFormat => MediaGroup::Unknown,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Level 3: MediaClass — content type / purpose
// ─────────────────────────────────────────────────────────────────────

/// Content classification describing what the media *is* rather than what container it uses.
///
/// This level typically requires metadata inspection or user tagging to populate
/// accurately; extension-based classification defaults to `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaClass {
    /// Song / album track
    Music,
    /// Podcast episode
    Podcast,
    /// Audiobook chapter or full book
    Audiobook,
    /// Feature film
    Movie,
    /// Television series episode
    TVShow,
    /// Music video clip
    MusicVideo,
    /// Live concert recording
    Concert,
    /// Documentary film or series
    Documentary,
    /// Personal / amateur video
    HomeVideo,
    /// Short ringtone clip
    Ringtone,
    /// Isolated sound effect sample
    SoundEffect,
    /// Content class not yet determined
    Unknown,
}

impl fmt::Display for MediaClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Music => write!(f, "Music"),
            Self::Podcast => write!(f, "Podcast"),
            Self::Audiobook => write!(f, "Audiobook"),
            Self::Movie => write!(f, "Movie"),
            Self::TVShow => write!(f, "TV Show"),
            Self::MusicVideo => write!(f, "Music Video"),
            Self::Concert => write!(f, "Concert"),
            Self::Documentary => write!(f, "Documentary"),
            Self::HomeVideo => write!(f, "Home Video"),
            Self::Ringtone => write!(f, "Ringtone"),
            Self::SoundEffect => write!(f, "Sound Effect"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Level 4: MediaQuality — encoding quality tier
// ─────────────────────────────────────────────────────────────────────

/// Quality tier for media, primarily meaningful for audio.
/// Image and video quality is reported as `Standard` unless
/// additional metadata (resolution, bitrate) is analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaQuality {
    /// Lossless encoding (FLAC, ALAC, WAV, AIFF, APE, WV, TTA at 16-bit/44.1kHz)
    Lossless,
    /// High-resolution lossless (24-bit / 88.2 kHz+ or DSD)
    HiRes,
    /// Lossy at 320 kbps (highest common lossy tier)
    Lossy320,
    /// Lossy at 256 kbps (Apple Music default)
    Lossy256,
    /// Lossy at 192 kbps (Spotify default on desktop)
    Lossy192,
    /// Lossy at 128 kbps (standard streaming)
    Lossy128,
    /// Lossy below 128 kbps
    LossyLow,
    /// Standard / non-audio quality (images, documents, etc.)
    Standard,
    /// Quality not yet determined
    Unknown,
}

impl fmt::Display for MediaQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lossless => write!(f, "Lossless"),
            Self::HiRes => write!(f, "Hi-Res"),
            Self::Lossy320 => write!(f, "320 kbps"),
            Self::Lossy256 => write!(f, "256 kbps"),
            Self::Lossy192 => write!(f, "192 kbps"),
            Self::Lossy128 => write!(f, "128 kbps"),
            Self::LossyLow => write!(f, "Low Quality"),
            Self::Standard => write!(f, "Standard"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// MediaClassification — composite struct holding all 4 levels
// ─────────────────────────────────────────────────────────────────────

/// Complete 4-level classification of a media file.
///
/// Created by `classify_by_extension` or `classify_by_path` and can be
/// refined later with metadata-driven quality and class detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaClassification {
    /// Broad category: Audio, Video, Image, Document, Archive, Unknown
    pub group: MediaGroup,
    /// Specific file format identified by extension
    pub format: MediaFormat,
    /// Content class (Music, Movie, Podcast, …) — often Unknown until metadata is read
    pub class: MediaClass,
    /// Encoding quality tier — Unknown until bitrate / codec info is available
    pub quality: MediaQuality,
}

impl MediaClassification {
    /// Create a new classification with explicit values for all 4 levels.
    pub fn new(
        group: MediaGroup,
        format: MediaFormat,
        class: MediaClass,
        quality: MediaQuality,
    ) -> Self {
        Self {
            group,
            format,
            class,
            quality,
        }
    }

    /// Convenience constructor for a fully-unknown classification.
    pub fn unknown() -> Self {
        Self {
            group: MediaGroup::Unknown,
            format: MediaFormat::UnknownFormat,
            class: MediaClass::Unknown,
            quality: MediaQuality::Unknown,
        }
    }
}

impl fmt::Display for MediaClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Example: "Audio / FLAC / Music / Lossless"
        write!(
            f,
            "{} / {} / {} / {}",
            self.group, self.format, self.class, self.quality
        )
    }
}

// ─────────────────────────────────────────────────────────────────────
// Extension → Format mapping (100+ extensions)
// ─────────────────────────────────────────────────────────────────────

/// Maps a lowercase file extension (without dot) to a `MediaFormat`.
/// Returns `MediaFormat::UnknownFormat` for unrecognised extensions.
fn extension_to_format(ext: &str) -> MediaFormat {
    match ext {
        // ── Audio ──────────────────────────────────────────────────
        "mp3" => MediaFormat::MP3,
        "flac" => MediaFormat::FLAC,
        "aac" => MediaFormat::AAC,
        "wav" => MediaFormat::WAV,
        "aiff" | "aif" | "aifc" => MediaFormat::AIFF,
        "alac" => MediaFormat::ALAC,
        "ogg" | "oga" => MediaFormat::OGG,
        "opus" => MediaFormat::OPUS,
        "wma" => MediaFormat::WMA,
        "m4a" => MediaFormat::M4A,
        "m4b" => MediaFormat::M4B,
        "ape" => MediaFormat::APE,
        "wv" => MediaFormat::WV,
        "mpc" | "mp+" | "mpp" => MediaFormat::MPC,
        "tta" => MediaFormat::TTA,
        "dsf" => MediaFormat::DSF,
        "dff" => MediaFormat::DFF,
        "amr" => MediaFormat::AMR,
        "au" | "snd" => MediaFormat::AU,
        "ra" | "ram" => MediaFormat::RA,
        "mid" | "midi" | "kar" => MediaFormat::MID,
        "spc" => MediaFormat::SPC,
        "mod" => MediaFormat::MOD,
        "s3m" => MediaFormat::S3M,
        "xm" => MediaFormat::XM,
        "it" => MediaFormat::IT,
        "caf" => MediaFormat::CAF,
        "ac3" => MediaFormat::AC3,
        "dts" => MediaFormat::DTS,

        // ── Video ──────────────────────────────────────────────────
        "mkv" => MediaFormat::MKV,
        "mp4" | "m4p" => MediaFormat::MP4,
        "avi" => MediaFormat::AVI,
        "mov" | "qt" => MediaFormat::MOV,
        "wmv" => MediaFormat::WMV,
        "webm" => MediaFormat::WEBM,
        "flv" => MediaFormat::FLV,
        "m4v" => MediaFormat::M4V,
        "ts" | "mts" | "m2ts" => MediaFormat::TS,
        "mpg" | "mp2" => MediaFormat::MPG,
        "mpeg" => MediaFormat::MPEG,
        "3gp" | "3g2" | "3gpp" => MediaFormat::ThreeGP,
        "rm" => MediaFormat::RM,
        "rmvb" => MediaFormat::RMVB,
        "vob" => MediaFormat::VOB,
        "ogv" => MediaFormat::OGV,
        "asf" => MediaFormat::ASF,
        "mxf" => MediaFormat::MXF,
        "mk3d" => MediaFormat::MK3D,
        "nsv" => MediaFormat::NSV,
        "f4v" => MediaFormat::F4V,

        // ── Image ──────────────────────────────────────────────────
        "jpg" | "jpeg" | "jpe" | "jfif" => MediaFormat::JPG,
        "png" => MediaFormat::PNG,
        "gif" => MediaFormat::GIF,
        "bmp" | "dib" => MediaFormat::BMP,
        "tiff" | "tif" => MediaFormat::TIFF,
        "webp" => MediaFormat::WEBP,
        "svg" | "svgz" => MediaFormat::SVG,
        "heif" | "hif" => MediaFormat::HEIF,
        "heic" => MediaFormat::HEIC,
        "avif" => MediaFormat::AVIF,
        "psd" => MediaFormat::PSD,
        "tga" | "icb" | "vda" | "vst" => MediaFormat::TGA,
        "ico" => MediaFormat::ICO,
        "cr2" => MediaFormat::CR2,
        "nef" | "nrw" => MediaFormat::NEF,
        "dng" => MediaFormat::DNG,
        "arw" | "srf" | "sr2" => MediaFormat::ARW,
        "orf" => MediaFormat::ORF,
        "raf" => MediaFormat::RAF,
        "rw2" => MediaFormat::RW2,
        "raw" => MediaFormat::RAW,
        "exr" => MediaFormat::EXR,
        "hdr" => MediaFormat::HDR,
        "ppm" => MediaFormat::PPM,
        "pgm" => MediaFormat::PGM,
        "jp2" | "j2k" | "jpf" | "jpx" => MediaFormat::JP2,
        "jxl" => MediaFormat::JXL,

        // ── Document ───────────────────────────────────────────────
        "pdf" => MediaFormat::PDF,
        "epub" => MediaFormat::EPUB,
        "mobi" | "prc" | "azw" | "azw3" | "kfx" => MediaFormat::MOBI,
        "doc" => MediaFormat::DOC,
        "docx" => MediaFormat::DOCX,
        "xls" => MediaFormat::XLS,
        "xlsx" => MediaFormat::XLSX,
        "ppt" => MediaFormat::PPT,
        "pptx" => MediaFormat::PPTX,
        "odt" => MediaFormat::ODT,
        "ods" => MediaFormat::ODS,
        "odp" => MediaFormat::ODP,
        "rtf" => MediaFormat::RTF,
        "txt" | "text" | "log" => MediaFormat::TXT,
        "csv" | "tsv" => MediaFormat::CSV,
        "html" | "htm" | "xhtml" => MediaFormat::HTML,
        "xml" | "xsl" | "xslt" => MediaFormat::XML,
        "json" | "jsonl" | "json5" => MediaFormat::JSON,
        "yaml" | "yml" => MediaFormat::YAML,
        "md" | "markdown" => MediaFormat::MD,
        "tex" | "latex" => MediaFormat::TEX,
        "djvu" | "djv" => MediaFormat::DJVU,
        "cbz" => MediaFormat::CBZ,
        "cbr" => MediaFormat::CBR,
        "fb2" => MediaFormat::FB2,
        "srt" => MediaFormat::SRT,
        "ass" | "ssa" => MediaFormat::ASS,
        "vtt" => MediaFormat::VTT,

        // ── Archive ────────────────────────────────────────────────
        "zip" => MediaFormat::ZIP,
        "rar" => MediaFormat::RAR,
        "7z" => MediaFormat::SevenZ,
        "tar" => MediaFormat::TAR,
        "gz" | "gzip" => MediaFormat::GZ,
        "bz2" | "bzip2" => MediaFormat::BZ2,
        "xz" | "lzma" => MediaFormat::XZ,
        "zst" | "zstd" => MediaFormat::ZST,
        "lz4" => MediaFormat::LZ4,
        "iso" => MediaFormat::ISO,
        "dmg" => MediaFormat::DMG,
        "msi" => MediaFormat::MSI,
        "deb" => MediaFormat::DEB,
        "rpm" => MediaFormat::RPM,
        "pkg" => MediaFormat::PKG,
        "jar" => MediaFormat::JAR,
        "apk" => MediaFormat::APK,

        // ── Compound archive extensions (tar.gz etc.) ──────────────
        // These are handled if the caller strips the outer extension;
        // "tgz" is a common single-extension alias for tar+gzip.
        "tgz" => MediaFormat::GZ,
        "tbz2" | "tbz" => MediaFormat::BZ2,
        "txz" => MediaFormat::XZ,

        // ── Unrecognised ───────────────────────────────────────────
        _ => MediaFormat::UnknownFormat,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Public classification functions
// ─────────────────────────────────────────────────────────────────────

/// Classify a media file from its extension alone (case-insensitive).
///
/// The extension should be provided **without** a leading dot.
/// Returns a `MediaClassification` with group and format populated;
/// class defaults to `Unknown` and quality defaults to `Unknown`
/// (or `Standard` for non-audio groups) since those require metadata.
///
/// # Examples
/// ```
/// use mm_core::classify::classify_by_extension;
///
/// let c = classify_by_extension("flac");
/// assert_eq!(c.format.extension(), "flac");
/// ```
pub fn classify_by_extension(ext: &str) -> MediaClassification {
    // Normalise the extension to lowercase for case-insensitive matching
    let lower = ext.to_ascii_lowercase();

    // Look up the format from the extension table
    let format = extension_to_format(&lower);

    // Derive the group from the format
    let group = format.group();

    // Class is Unknown by default — needs metadata to determine
    let class = MediaClass::Unknown;

    // Quality: for non-audio groups we know the quality concept doesn't
    // directly apply, so we set Standard. For audio, Unknown until we
    // have bitrate / codec information.
    let quality = match group {
        MediaGroup::Audio => MediaQuality::Unknown,
        MediaGroup::Unknown => MediaQuality::Unknown,
        _ => MediaQuality::Standard,
    };

    MediaClassification {
        group,
        format,
        class,
        quality,
    }
}

/// Classify a media file from its full path.
///
/// Extracts the file extension from the path and delegates to
/// `classify_by_extension`.  Returns an error if the path has no
/// extension at all.
///
/// # Errors
/// Returns `MmError::Classify` if the path has no file extension.
pub fn classify_by_path(path: &Path) -> MmResult<MediaClassification> {
    // Extract the extension from the path, handling the None case
    let ext = path
        .extension() // Option<&OsStr>
        .and_then(|e| e.to_str()) // Option<&str> (lossy-free)
        .ok_or_else(|| {
            MmError::Classify(format!("path has no file extension: {}", path.display()))
        })?;

    Ok(classify_by_extension(ext))
}

// ─────────────────────────────────────────────────────────────────────
// Quality detection from bitrate
// ─────────────────────────────────────────────────────────────────────

/// Determine the audio quality tier from bitrate and lossless flag.
///
/// For lossless formats the `is_lossless` flag should be `true` and the
/// bitrate is used to distinguish standard CD quality from hi-res:
///   - >= 2_000 kbps → HiRes  (24-bit / high sample rate)
///   - <  2_000 kbps → Lossless (16-bit / 44.1 kHz)
///
/// For lossy formats (`is_lossless == false`) the bitrate thresholds are:
///   - >= 288 kbps → Lossy320  (320 or VBR V0)
///   - >= 224 kbps → Lossy256
///   - >= 160 kbps → Lossy192
///   - >= 112 kbps → Lossy128
///   - <  112 kbps → LossyLow
///
/// A bitrate of 0 returns `Unknown` regardless of the lossless flag.
pub fn detect_quality(bitrate_kbps: u32, is_lossless: bool) -> MediaQuality {
    // Zero bitrate means we don't have the information
    if bitrate_kbps == 0 {
        return MediaQuality::Unknown;
    }

    if is_lossless {
        // Hi-res lossless: 24-bit/96kHz stereo FLAC is typically ~4600 kbps,
        // 24-bit/48kHz is ~2300 kbps. CD quality 16/44.1 is ~1411 kbps.
        // We use 2000 kbps as the dividing line.
        if bitrate_kbps >= 2_000 {
            MediaQuality::HiRes
        } else {
            MediaQuality::Lossless
        }
    } else {
        // Lossy quality tiers — thresholds allow for VBR variance
        // (e.g. LAME V0 averages ~245 kbps but can spike to 320)
        if bitrate_kbps >= 288 {
            MediaQuality::Lossy320
        } else if bitrate_kbps >= 224 {
            MediaQuality::Lossy256
        } else if bitrate_kbps >= 160 {
            MediaQuality::Lossy192
        } else if bitrate_kbps >= 112 {
            MediaQuality::Lossy128
        } else {
            MediaQuality::LossyLow
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Group classification tests ─────────────────────────────────

    #[test]
    fn classify_audio_mp3() {
        let c = classify_by_extension("mp3");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::MP3);
        assert_eq!(c.class, MediaClass::Unknown);
        assert_eq!(c.quality, MediaQuality::Unknown);
    }

    #[test]
    fn classify_audio_flac() {
        let c = classify_by_extension("flac");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::FLAC);
    }

    #[test]
    fn classify_audio_wav() {
        let c = classify_by_extension("wav");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::WAV);
    }

    #[test]
    fn classify_audio_aac() {
        let c = classify_by_extension("aac");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::AAC);
    }

    #[test]
    fn classify_audio_ogg() {
        let c = classify_by_extension("ogg");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::OGG);
    }

    #[test]
    fn classify_audio_opus() {
        let c = classify_by_extension("opus");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::OPUS);
    }

    #[test]
    fn classify_audio_m4b_audiobook() {
        // M4B is the Apple audiobook container — still classified as Audio group
        let c = classify_by_extension("m4b");
        assert_eq!(c.group, MediaGroup::Audio);
        assert_eq!(c.format, MediaFormat::M4B);
    }

    #[test]
    fn classify_video_mkv() {
        let c = classify_by_extension("mkv");
        assert_eq!(c.group, MediaGroup::Video);
        assert_eq!(c.format, MediaFormat::MKV);
        // Video gets Standard quality by default (not Unknown)
        assert_eq!(c.quality, MediaQuality::Standard);
    }

    #[test]
    fn classify_video_mp4() {
        let c = classify_by_extension("mp4");
        assert_eq!(c.group, MediaGroup::Video);
        assert_eq!(c.format, MediaFormat::MP4);
    }

    #[test]
    fn classify_video_webm() {
        let c = classify_by_extension("webm");
        assert_eq!(c.group, MediaGroup::Video);
        assert_eq!(c.format, MediaFormat::WEBM);
    }

    #[test]
    fn classify_video_avi() {
        let c = classify_by_extension("avi");
        assert_eq!(c.group, MediaGroup::Video);
        assert_eq!(c.format, MediaFormat::AVI);
    }

    #[test]
    fn classify_image_jpg() {
        let c = classify_by_extension("jpg");
        assert_eq!(c.group, MediaGroup::Image);
        assert_eq!(c.format, MediaFormat::JPG);
        assert_eq!(c.quality, MediaQuality::Standard);
    }

    #[test]
    fn classify_image_jpeg_alias() {
        // "jpeg" should resolve to the same format as "jpg"
        let c = classify_by_extension("jpeg");
        assert_eq!(c.format, MediaFormat::JPG);
    }

    #[test]
    fn classify_image_png() {
        let c = classify_by_extension("png");
        assert_eq!(c.group, MediaGroup::Image);
        assert_eq!(c.format, MediaFormat::PNG);
    }

    #[test]
    fn classify_image_heic() {
        let c = classify_by_extension("heic");
        assert_eq!(c.group, MediaGroup::Image);
        assert_eq!(c.format, MediaFormat::HEIC);
    }

    #[test]
    fn classify_image_raw_cr2() {
        let c = classify_by_extension("cr2");
        assert_eq!(c.group, MediaGroup::Image);
        assert_eq!(c.format, MediaFormat::CR2);
    }

    #[test]
    fn classify_document_pdf() {
        let c = classify_by_extension("pdf");
        assert_eq!(c.group, MediaGroup::Document);
        assert_eq!(c.format, MediaFormat::PDF);
    }

    #[test]
    fn classify_document_epub() {
        let c = classify_by_extension("epub");
        assert_eq!(c.group, MediaGroup::Document);
        assert_eq!(c.format, MediaFormat::EPUB);
    }

    #[test]
    fn classify_document_docx() {
        let c = classify_by_extension("docx");
        assert_eq!(c.group, MediaGroup::Document);
        assert_eq!(c.format, MediaFormat::DOCX);
    }

    #[test]
    fn classify_archive_zip() {
        let c = classify_by_extension("zip");
        assert_eq!(c.group, MediaGroup::Archive);
        assert_eq!(c.format, MediaFormat::ZIP);
    }

    #[test]
    fn classify_archive_7z() {
        let c = classify_by_extension("7z");
        assert_eq!(c.group, MediaGroup::Archive);
        assert_eq!(c.format, MediaFormat::SevenZ);
    }

    #[test]
    fn classify_archive_iso() {
        let c = classify_by_extension("iso");
        assert_eq!(c.group, MediaGroup::Archive);
        assert_eq!(c.format, MediaFormat::ISO);
    }

    // ── Case insensitivity ─────────────────────────────────────────

    #[test]
    fn classify_case_insensitive() {
        // Extension matching should be case-insensitive
        let upper = classify_by_extension("MP3");
        let lower = classify_by_extension("mp3");
        let mixed = classify_by_extension("Mp3");
        assert_eq!(upper.format, lower.format);
        assert_eq!(lower.format, mixed.format);
    }

    // ── Unknown extension ──────────────────────────────────────────

    #[test]
    fn classify_unknown_extension() {
        let c = classify_by_extension("xyz123");
        assert_eq!(c.group, MediaGroup::Unknown);
        assert_eq!(c.format, MediaFormat::UnknownFormat);
        assert_eq!(c.quality, MediaQuality::Unknown);
    }

    #[test]
    fn classify_empty_extension() {
        let c = classify_by_extension("");
        assert_eq!(c.group, MediaGroup::Unknown);
        assert_eq!(c.format, MediaFormat::UnknownFormat);
    }

    // ── Path-based classification ──────────────────────────────────

    #[test]
    fn classify_by_path_success() {
        let path = PathBuf::from("/music/album/track.flac");
        let c = classify_by_path(&path).unwrap();
        assert_eq!(c.format, MediaFormat::FLAC);
        assert_eq!(c.group, MediaGroup::Audio);
    }

    #[test]
    fn classify_by_path_no_extension() {
        // A path with no extension should produce a Classify error
        let path = PathBuf::from("/music/album/README");
        let result = classify_by_path(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MmError::Classify(_)));
    }

    // ── Quality detection ──────────────────────────────────────────

    #[test]
    fn quality_lossless_cd() {
        // CD quality: 16-bit/44.1kHz stereo FLAC ≈ 1411 kbps
        let q = detect_quality(1411, true);
        assert_eq!(q, MediaQuality::Lossless);
    }

    #[test]
    fn quality_hires() {
        // 24-bit/96kHz stereo FLAC ≈ 4608 kbps
        let q = detect_quality(4608, true);
        assert_eq!(q, MediaQuality::HiRes);
    }

    #[test]
    fn quality_lossy_320() {
        let q = detect_quality(320, false);
        assert_eq!(q, MediaQuality::Lossy320);
    }

    #[test]
    fn quality_lossy_256() {
        let q = detect_quality(256, false);
        assert_eq!(q, MediaQuality::Lossy256);
    }

    #[test]
    fn quality_lossy_192() {
        let q = detect_quality(192, false);
        assert_eq!(q, MediaQuality::Lossy192);
    }

    #[test]
    fn quality_lossy_128() {
        let q = detect_quality(128, false);
        assert_eq!(q, MediaQuality::Lossy128);
    }

    #[test]
    fn quality_lossy_low() {
        let q = detect_quality(64, false);
        assert_eq!(q, MediaQuality::LossyLow);
    }

    #[test]
    fn quality_zero_bitrate_returns_unknown() {
        // Zero bitrate = not enough information
        assert_eq!(detect_quality(0, true), MediaQuality::Unknown);
        assert_eq!(detect_quality(0, false), MediaQuality::Unknown);
    }

    // ── Format methods ─────────────────────────────────────────────

    #[test]
    fn format_extension_roundtrip() {
        // For canonical formats, classifying by extension should return the same format
        let formats = [
            MediaFormat::MP3,
            MediaFormat::FLAC,
            MediaFormat::AAC,
            MediaFormat::WAV,
            MediaFormat::MKV,
            MediaFormat::MP4,
            MediaFormat::JPG,
            MediaFormat::PNG,
            MediaFormat::PDF,
            MediaFormat::ZIP,
        ];
        for fmt in formats {
            let ext = fmt.extension();
            let classified = classify_by_extension(ext);
            assert_eq!(classified.format, fmt, "Roundtrip failed for {fmt}");
        }
    }

    #[test]
    fn format_mime_types() {
        assert_eq!(MediaFormat::MP3.mime_type(), "audio/mpeg");
        assert_eq!(MediaFormat::MP4.mime_type(), "video/mp4");
        assert_eq!(MediaFormat::JPG.mime_type(), "image/jpeg");
        assert_eq!(MediaFormat::PDF.mime_type(), "application/pdf");
        assert_eq!(MediaFormat::ZIP.mime_type(), "application/zip");
        assert_eq!(
            MediaFormat::UnknownFormat.mime_type(),
            "application/octet-stream"
        );
    }

    // ── Display impls ──────────────────────────────────────────────

    #[test]
    fn display_group() {
        assert_eq!(MediaGroup::Audio.to_string(), "Audio");
        assert_eq!(MediaGroup::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn display_format() {
        assert_eq!(MediaFormat::FLAC.to_string(), "FLAC");
        assert_eq!(MediaFormat::MKV.to_string(), "Matroska Video");
        assert_eq!(MediaFormat::UnknownFormat.to_string(), "Unknown");
    }

    #[test]
    fn display_class() {
        assert_eq!(MediaClass::Music.to_string(), "Music");
        assert_eq!(MediaClass::TVShow.to_string(), "TV Show");
        assert_eq!(MediaClass::MusicVideo.to_string(), "Music Video");
    }

    #[test]
    fn display_quality() {
        assert_eq!(MediaQuality::Lossless.to_string(), "Lossless");
        assert_eq!(MediaQuality::HiRes.to_string(), "Hi-Res");
        assert_eq!(MediaQuality::Lossy320.to_string(), "320 kbps");
    }

    #[test]
    fn display_classification() {
        let c = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::FLAC,
            MediaClass::Music,
            MediaQuality::Lossless,
        );
        assert_eq!(c.to_string(), "Audio / FLAC / Music / Lossless");
    }

    // ── MediaClassification helpers ────────────────────────────────

    #[test]
    fn classification_unknown_helper() {
        let c = MediaClassification::unknown();
        assert_eq!(c.group, MediaGroup::Unknown);
        assert_eq!(c.format, MediaFormat::UnknownFormat);
        assert_eq!(c.class, MediaClass::Unknown);
        assert_eq!(c.quality, MediaQuality::Unknown);
    }

    // ── Alternative / alias extensions ─────────────────────────────

    #[test]
    fn classify_alternative_extensions() {
        // Test that common aliases resolve correctly
        assert_eq!(classify_by_extension("jpeg").format, MediaFormat::JPG);
        assert_eq!(classify_by_extension("jpe").format, MediaFormat::JPG);
        assert_eq!(classify_by_extension("jfif").format, MediaFormat::JPG);
        assert_eq!(classify_by_extension("tif").format, MediaFormat::TIFF);
        assert_eq!(classify_by_extension("aif").format, MediaFormat::AIFF);
        assert_eq!(classify_by_extension("htm").format, MediaFormat::HTML);
        assert_eq!(classify_by_extension("yml").format, MediaFormat::YAML);
        assert_eq!(classify_by_extension("midi").format, MediaFormat::MID);
        assert_eq!(classify_by_extension("3gp").format, MediaFormat::ThreeGP);
        assert_eq!(classify_by_extension("m2ts").format, MediaFormat::TS);
        assert_eq!(classify_by_extension("azw3").format, MediaFormat::MOBI);
        assert_eq!(classify_by_extension("qt").format, MediaFormat::MOV);
    }

    // ── Serde round-trip ───────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let original = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::FLAC,
            MediaClass::Music,
            MediaQuality::HiRes,
        );
        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("serialize");
        // Deserialize back
        let restored: MediaClassification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }
}
