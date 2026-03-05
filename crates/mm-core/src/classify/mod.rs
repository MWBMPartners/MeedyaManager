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

use std::fmt;                              // Display trait for human-readable enum output
use std::path::Path;                       // Path handling for classify_by_path

use serde::{Deserialize, Serialize};       // Serialization support for all enums

use crate::error::{MmError, MmResult};    // Unified error type

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
            MediaGroup::Audio    => write!(f, "Audio"),
            MediaGroup::Video    => write!(f, "Video"),
            MediaGroup::Image    => write!(f, "Image"),
            MediaGroup::Document => write!(f, "Document"),
            MediaGroup::Archive  => write!(f, "Archive"),
            MediaGroup::Unknown  => write!(f, "Unknown"),
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
            MediaFormat::MP3   => write!(f, "MP3"),
            MediaFormat::FLAC  => write!(f, "FLAC"),
            MediaFormat::AAC   => write!(f, "AAC"),
            MediaFormat::WAV   => write!(f, "WAV"),
            MediaFormat::AIFF  => write!(f, "AIFF"),
            MediaFormat::ALAC  => write!(f, "ALAC"),
            MediaFormat::OGG   => write!(f, "Ogg Vorbis"),
            MediaFormat::OPUS  => write!(f, "Opus"),
            MediaFormat::WMA   => write!(f, "WMA"),
            MediaFormat::M4A   => write!(f, "M4A"),
            MediaFormat::M4B   => write!(f, "M4B"),
            MediaFormat::APE   => write!(f, "Monkey's Audio"),
            MediaFormat::WV    => write!(f, "WavPack"),
            MediaFormat::MPC   => write!(f, "Musepack"),
            MediaFormat::TTA   => write!(f, "True Audio"),
            MediaFormat::DSF   => write!(f, "DSD (DSF)"),
            MediaFormat::DFF   => write!(f, "DSD (DFF)"),
            MediaFormat::AMR   => write!(f, "AMR"),
            MediaFormat::AU    => write!(f, "Au/SND"),
            MediaFormat::RA    => write!(f, "RealAudio"),
            MediaFormat::MID   => write!(f, "MIDI"),
            MediaFormat::SPC   => write!(f, "SPC"),
            MediaFormat::MOD   => write!(f, "MOD"),
            MediaFormat::S3M   => write!(f, "S3M"),
            MediaFormat::XM    => write!(f, "XM"),
            MediaFormat::IT    => write!(f, "IT"),
            MediaFormat::CAF   => write!(f, "Core Audio Format"),
            MediaFormat::AC3   => write!(f, "AC-3 / Dolby Digital"),
            MediaFormat::DTS   => write!(f, "DTS"),
            // Video
            MediaFormat::MKV     => write!(f, "Matroska Video"),
            MediaFormat::MP4     => write!(f, "MP4"),
            MediaFormat::AVI     => write!(f, "AVI"),
            MediaFormat::MOV     => write!(f, "QuickTime"),
            MediaFormat::WMV     => write!(f, "WMV"),
            MediaFormat::WEBM    => write!(f, "WebM"),
            MediaFormat::FLV     => write!(f, "Flash Video"),
            MediaFormat::M4V     => write!(f, "M4V"),
            MediaFormat::TS      => write!(f, "MPEG-TS"),
            MediaFormat::MPG     => write!(f, "MPEG"),
            MediaFormat::MPEG    => write!(f, "MPEG-2"),
            MediaFormat::ThreeGP => write!(f, "3GP"),
            MediaFormat::RM      => write!(f, "RealMedia"),
            MediaFormat::RMVB    => write!(f, "RealMedia VBR"),
            MediaFormat::VOB     => write!(f, "DVD Video Object"),
            MediaFormat::OGV     => write!(f, "Ogg Video"),
            MediaFormat::ASF     => write!(f, "ASF"),
            MediaFormat::MXF     => write!(f, "MXF"),
            MediaFormat::MK3D    => write!(f, "Matroska 3D"),
            MediaFormat::NSV     => write!(f, "NSV"),
            MediaFormat::F4V     => write!(f, "Flash Video (F4V)"),
            // Image
            MediaFormat::JPG  => write!(f, "JPEG"),
            MediaFormat::PNG  => write!(f, "PNG"),
            MediaFormat::GIF  => write!(f, "GIF"),
            MediaFormat::BMP  => write!(f, "BMP"),
            MediaFormat::TIFF => write!(f, "TIFF"),
            MediaFormat::WEBP => write!(f, "WebP"),
            MediaFormat::SVG  => write!(f, "SVG"),
            MediaFormat::HEIF => write!(f, "HEIF"),
            MediaFormat::HEIC => write!(f, "HEIC"),
            MediaFormat::AVIF => write!(f, "AVIF"),
            MediaFormat::PSD  => write!(f, "Photoshop"),
            MediaFormat::TGA  => write!(f, "TGA"),
            MediaFormat::ICO  => write!(f, "ICO"),
            MediaFormat::CR2  => write!(f, "Canon RAW"),
            MediaFormat::NEF  => write!(f, "Nikon RAW"),
            MediaFormat::DNG  => write!(f, "DNG"),
            MediaFormat::ARW  => write!(f, "Sony RAW"),
            MediaFormat::ORF  => write!(f, "Olympus RAW"),
            MediaFormat::RAF  => write!(f, "Fuji RAW"),
            MediaFormat::RW2  => write!(f, "Panasonic RAW"),
            MediaFormat::RAW  => write!(f, "RAW"),
            MediaFormat::EXR  => write!(f, "OpenEXR"),
            MediaFormat::HDR  => write!(f, "Radiance HDR"),
            MediaFormat::PPM  => write!(f, "PPM"),
            MediaFormat::PGM  => write!(f, "PGM"),
            MediaFormat::JP2  => write!(f, "JPEG 2000"),
            MediaFormat::JXL  => write!(f, "JPEG XL"),
            // Document
            MediaFormat::PDF  => write!(f, "PDF"),
            MediaFormat::EPUB => write!(f, "EPUB"),
            MediaFormat::MOBI => write!(f, "Kindle"),
            MediaFormat::DOC  => write!(f, "Word (DOC)"),
            MediaFormat::DOCX => write!(f, "Word (DOCX)"),
            MediaFormat::XLS  => write!(f, "Excel (XLS)"),
            MediaFormat::XLSX => write!(f, "Excel (XLSX)"),
            MediaFormat::PPT  => write!(f, "PowerPoint (PPT)"),
            MediaFormat::PPTX => write!(f, "PowerPoint (PPTX)"),
            MediaFormat::ODT  => write!(f, "OpenDocument Text"),
            MediaFormat::ODS  => write!(f, "OpenDocument Spreadsheet"),
            MediaFormat::ODP  => write!(f, "OpenDocument Presentation"),
            MediaFormat::RTF  => write!(f, "Rich Text"),
            MediaFormat::TXT  => write!(f, "Plain Text"),
            MediaFormat::CSV  => write!(f, "CSV"),
            MediaFormat::HTML => write!(f, "HTML"),
            MediaFormat::XML  => write!(f, "XML"),
            MediaFormat::JSON => write!(f, "JSON"),
            MediaFormat::YAML => write!(f, "YAML"),
            MediaFormat::MD   => write!(f, "Markdown"),
            MediaFormat::TEX  => write!(f, "LaTeX"),
            MediaFormat::DJVU => write!(f, "DjVu"),
            MediaFormat::CBZ  => write!(f, "CBZ"),
            MediaFormat::CBR  => write!(f, "CBR"),
            MediaFormat::FB2  => write!(f, "FictionBook"),
            MediaFormat::SRT  => write!(f, "SubRip"),
            MediaFormat::ASS  => write!(f, "ASS Subtitles"),
            MediaFormat::VTT  => write!(f, "WebVTT"),
            // Archive
            MediaFormat::ZIP    => write!(f, "ZIP"),
            MediaFormat::RAR    => write!(f, "RAR"),
            MediaFormat::SevenZ => write!(f, "7-Zip"),
            MediaFormat::TAR    => write!(f, "TAR"),
            MediaFormat::GZ     => write!(f, "Gzip"),
            MediaFormat::BZ2    => write!(f, "Bzip2"),
            MediaFormat::XZ     => write!(f, "XZ"),
            MediaFormat::ZST    => write!(f, "Zstandard"),
            MediaFormat::LZ4    => write!(f, "LZ4"),
            MediaFormat::ISO    => write!(f, "ISO"),
            MediaFormat::DMG    => write!(f, "Apple Disk Image"),
            MediaFormat::MSI    => write!(f, "Windows Installer"),
            MediaFormat::DEB    => write!(f, "Debian Package"),
            MediaFormat::RPM    => write!(f, "RPM Package"),
            MediaFormat::PKG    => write!(f, "macOS Package"),
            MediaFormat::JAR    => write!(f, "Java Archive"),
            MediaFormat::APK    => write!(f, "Android Package"),
            // Unknown
            MediaFormat::UnknownFormat => write!(f, "Unknown"),
        }
    }
}

impl MediaFormat {
    /// Returns the canonical file extension (without leading dot) for this format.
    pub fn extension(&self) -> &str {
        match self {
            // Audio
            MediaFormat::MP3  => "mp3",
            MediaFormat::FLAC => "flac",
            MediaFormat::AAC  => "aac",
            MediaFormat::WAV  => "wav",
            MediaFormat::AIFF => "aiff",
            MediaFormat::ALAC => "m4a",   // ALAC uses the M4A container
            MediaFormat::OGG  => "ogg",
            MediaFormat::OPUS => "opus",
            MediaFormat::WMA  => "wma",
            MediaFormat::M4A  => "m4a",
            MediaFormat::M4B  => "m4b",
            MediaFormat::APE  => "ape",
            MediaFormat::WV   => "wv",
            MediaFormat::MPC  => "mpc",
            MediaFormat::TTA  => "tta",
            MediaFormat::DSF  => "dsf",
            MediaFormat::DFF  => "dff",
            MediaFormat::AMR  => "amr",
            MediaFormat::AU   => "au",
            MediaFormat::RA   => "ra",
            MediaFormat::MID  => "mid",
            MediaFormat::SPC  => "spc",
            MediaFormat::MOD  => "mod",
            MediaFormat::S3M  => "s3m",
            MediaFormat::XM   => "xm",
            MediaFormat::IT   => "it",
            MediaFormat::CAF  => "caf",
            MediaFormat::AC3  => "ac3",
            MediaFormat::DTS  => "dts",
            // Video
            MediaFormat::MKV     => "mkv",
            MediaFormat::MP4     => "mp4",
            MediaFormat::AVI     => "avi",
            MediaFormat::MOV     => "mov",
            MediaFormat::WMV     => "wmv",
            MediaFormat::WEBM    => "webm",
            MediaFormat::FLV     => "flv",
            MediaFormat::M4V     => "m4v",
            MediaFormat::TS      => "ts",
            MediaFormat::MPG     => "mpg",
            MediaFormat::MPEG    => "mpeg",
            MediaFormat::ThreeGP => "3gp",
            MediaFormat::RM      => "rm",
            MediaFormat::RMVB    => "rmvb",
            MediaFormat::VOB     => "vob",
            MediaFormat::OGV     => "ogv",
            MediaFormat::ASF     => "asf",
            MediaFormat::MXF     => "mxf",
            MediaFormat::MK3D    => "mk3d",
            MediaFormat::NSV     => "nsv",
            MediaFormat::F4V     => "f4v",
            // Image
            MediaFormat::JPG  => "jpg",
            MediaFormat::PNG  => "png",
            MediaFormat::GIF  => "gif",
            MediaFormat::BMP  => "bmp",
            MediaFormat::TIFF => "tiff",
            MediaFormat::WEBP => "webp",
            MediaFormat::SVG  => "svg",
            MediaFormat::HEIF => "heif",
            MediaFormat::HEIC => "heic",
            MediaFormat::AVIF => "avif",
            MediaFormat::PSD  => "psd",
            MediaFormat::TGA  => "tga",
            MediaFormat::ICO  => "ico",
            MediaFormat::CR2  => "cr2",
            MediaFormat::NEF  => "nef",
            MediaFormat::DNG  => "dng",
            MediaFormat::ARW  => "arw",
            MediaFormat::ORF  => "orf",
            MediaFormat::RAF  => "raf",
            MediaFormat::RW2  => "rw2",
            MediaFormat::RAW  => "raw",
            MediaFormat::EXR  => "exr",
            MediaFormat::HDR  => "hdr",
            MediaFormat::PPM  => "ppm",
            MediaFormat::PGM  => "pgm",
            MediaFormat::JP2  => "jp2",
            MediaFormat::JXL  => "jxl",
            // Document
            MediaFormat::PDF  => "pdf",
            MediaFormat::EPUB => "epub",
            MediaFormat::MOBI => "mobi",
            MediaFormat::DOC  => "doc",
            MediaFormat::DOCX => "docx",
            MediaFormat::XLS  => "xls",
            MediaFormat::XLSX => "xlsx",
            MediaFormat::PPT  => "ppt",
            MediaFormat::PPTX => "pptx",
            MediaFormat::ODT  => "odt",
            MediaFormat::ODS  => "ods",
            MediaFormat::ODP  => "odp",
            MediaFormat::RTF  => "rtf",
            MediaFormat::TXT  => "txt",
            MediaFormat::CSV  => "csv",
            MediaFormat::HTML => "html",
            MediaFormat::XML  => "xml",
            MediaFormat::JSON => "json",
            MediaFormat::YAML => "yaml",
            MediaFormat::MD   => "md",
            MediaFormat::TEX  => "tex",
            MediaFormat::DJVU => "djvu",
            MediaFormat::CBZ  => "cbz",
            MediaFormat::CBR  => "cbr",
            MediaFormat::FB2  => "fb2",
            MediaFormat::SRT  => "srt",
            MediaFormat::ASS  => "ass",
            MediaFormat::VTT  => "vtt",
            // Archive
            MediaFormat::ZIP    => "zip",
            MediaFormat::RAR    => "rar",
            MediaFormat::SevenZ => "7z",
            MediaFormat::TAR    => "tar",
            MediaFormat::GZ     => "gz",
            MediaFormat::BZ2    => "bz2",
            MediaFormat::XZ     => "xz",
            MediaFormat::ZST    => "zst",
            MediaFormat::LZ4    => "lz4",
            MediaFormat::ISO    => "iso",
            MediaFormat::DMG    => "dmg",
            MediaFormat::MSI    => "msi",
            MediaFormat::DEB    => "deb",
            MediaFormat::RPM    => "rpm",
            MediaFormat::PKG    => "pkg",
            MediaFormat::JAR    => "jar",
            MediaFormat::APK    => "apk",
            // Unknown
            MediaFormat::UnknownFormat => "",
        }
    }

    /// Returns the IANA MIME type for this format.
    /// Falls back to "application/octet-stream" for unknown formats.
    pub fn mime_type(&self) -> &str {
        match self {
            // Audio
            MediaFormat::MP3  => "audio/mpeg",
            MediaFormat::FLAC => "audio/flac",
            MediaFormat::AAC  => "audio/aac",
            MediaFormat::WAV  => "audio/wav",
            MediaFormat::AIFF => "audio/aiff",
            MediaFormat::ALAC => "audio/mp4",
            MediaFormat::OGG  => "audio/ogg",
            MediaFormat::OPUS => "audio/opus",
            MediaFormat::WMA  => "audio/x-ms-wma",
            MediaFormat::M4A  => "audio/mp4",
            MediaFormat::M4B  => "audio/mp4",
            MediaFormat::APE  => "audio/x-ape",
            MediaFormat::WV   => "audio/x-wavpack",
            MediaFormat::MPC  => "audio/x-musepack",
            MediaFormat::TTA  => "audio/x-tta",
            MediaFormat::DSF  => "audio/x-dsf",
            MediaFormat::DFF  => "audio/x-dff",
            MediaFormat::AMR  => "audio/amr",
            MediaFormat::AU   => "audio/basic",
            MediaFormat::RA   => "audio/x-realaudio",
            MediaFormat::MID  => "audio/midi",
            MediaFormat::SPC  => "audio/x-spc",
            MediaFormat::MOD  => "audio/x-mod",
            MediaFormat::S3M  => "audio/x-s3m",
            MediaFormat::XM   => "audio/x-xm",
            MediaFormat::IT   => "audio/x-it",
            MediaFormat::CAF  => "audio/x-caf",
            MediaFormat::AC3  => "audio/ac3",
            MediaFormat::DTS  => "audio/vnd.dts",
            // Video
            MediaFormat::MKV     => "video/x-matroska",
            MediaFormat::MP4     => "video/mp4",
            MediaFormat::AVI     => "video/x-msvideo",
            MediaFormat::MOV     => "video/quicktime",
            MediaFormat::WMV     => "video/x-ms-wmv",
            MediaFormat::WEBM    => "video/webm",
            MediaFormat::FLV     => "video/x-flv",
            MediaFormat::M4V     => "video/mp4",
            MediaFormat::TS      => "video/mp2t",
            MediaFormat::MPG     => "video/mpeg",
            MediaFormat::MPEG    => "video/mpeg",
            MediaFormat::ThreeGP => "video/3gpp",
            MediaFormat::RM      => "application/vnd.rn-realmedia",
            MediaFormat::RMVB    => "application/vnd.rn-realmedia-vbr",
            MediaFormat::VOB     => "video/x-ms-vob",
            MediaFormat::OGV     => "video/ogg",
            MediaFormat::ASF     => "video/x-ms-asf",
            MediaFormat::MXF     => "application/mxf",
            MediaFormat::MK3D    => "video/x-matroska-3d",
            MediaFormat::NSV     => "video/x-nsv",
            MediaFormat::F4V     => "video/mp4",
            // Image
            MediaFormat::JPG  => "image/jpeg",
            MediaFormat::PNG  => "image/png",
            MediaFormat::GIF  => "image/gif",
            MediaFormat::BMP  => "image/bmp",
            MediaFormat::TIFF => "image/tiff",
            MediaFormat::WEBP => "image/webp",
            MediaFormat::SVG  => "image/svg+xml",
            MediaFormat::HEIF => "image/heif",
            MediaFormat::HEIC => "image/heic",
            MediaFormat::AVIF => "image/avif",
            MediaFormat::PSD  => "image/vnd.adobe.photoshop",
            MediaFormat::TGA  => "image/x-tga",
            MediaFormat::ICO  => "image/x-icon",
            MediaFormat::CR2  => "image/x-canon-cr2",
            MediaFormat::NEF  => "image/x-nikon-nef",
            MediaFormat::DNG  => "image/x-adobe-dng",
            MediaFormat::ARW  => "image/x-sony-arw",
            MediaFormat::ORF  => "image/x-olympus-orf",
            MediaFormat::RAF  => "image/x-fuji-raf",
            MediaFormat::RW2  => "image/x-panasonic-rw2",
            MediaFormat::RAW  => "image/x-raw",
            MediaFormat::EXR  => "image/x-exr",
            MediaFormat::HDR  => "image/vnd.radiance",
            MediaFormat::PPM  => "image/x-portable-pixmap",
            MediaFormat::PGM  => "image/x-portable-graymap",
            MediaFormat::JP2  => "image/jp2",
            MediaFormat::JXL  => "image/jxl",
            // Document
            MediaFormat::PDF  => "application/pdf",
            MediaFormat::EPUB => "application/epub+zip",
            MediaFormat::MOBI => "application/x-mobipocket-ebook",
            MediaFormat::DOC  => "application/msword",
            MediaFormat::DOCX => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            MediaFormat::XLS  => "application/vnd.ms-excel",
            MediaFormat::XLSX => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            MediaFormat::PPT  => "application/vnd.ms-powerpoint",
            MediaFormat::PPTX => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            MediaFormat::ODT  => "application/vnd.oasis.opendocument.text",
            MediaFormat::ODS  => "application/vnd.oasis.opendocument.spreadsheet",
            MediaFormat::ODP  => "application/vnd.oasis.opendocument.presentation",
            MediaFormat::RTF  => "application/rtf",
            MediaFormat::TXT  => "text/plain",
            MediaFormat::CSV  => "text/csv",
            MediaFormat::HTML => "text/html",
            MediaFormat::XML  => "application/xml",
            MediaFormat::JSON => "application/json",
            MediaFormat::YAML => "application/x-yaml",
            MediaFormat::MD   => "text/markdown",
            MediaFormat::TEX  => "application/x-tex",
            MediaFormat::DJVU => "image/vnd.djvu",
            MediaFormat::CBZ  => "application/vnd.comicbook+zip",
            MediaFormat::CBR  => "application/vnd.comicbook-rar",
            MediaFormat::FB2  => "application/x-fictionbook+xml",
            MediaFormat::SRT  => "application/x-subrip",
            MediaFormat::ASS  => "text/x-ssa",
            MediaFormat::VTT  => "text/vtt",
            // Archive
            MediaFormat::ZIP    => "application/zip",
            MediaFormat::RAR    => "application/vnd.rar",
            MediaFormat::SevenZ => "application/x-7z-compressed",
            MediaFormat::TAR    => "application/x-tar",
            MediaFormat::GZ     => "application/gzip",
            MediaFormat::BZ2    => "application/x-bzip2",
            MediaFormat::XZ     => "application/x-xz",
            MediaFormat::ZST    => "application/zstd",
            MediaFormat::LZ4    => "application/x-lz4",
            MediaFormat::ISO    => "application/x-iso9660-image",
            MediaFormat::DMG    => "application/x-apple-diskimage",
            MediaFormat::MSI    => "application/x-msi",
            MediaFormat::DEB    => "application/vnd.debian.binary-package",
            MediaFormat::RPM    => "application/x-rpm",
            MediaFormat::PKG    => "application/x-newton-compatible-pkg",
            MediaFormat::JAR    => "application/java-archive",
            MediaFormat::APK    => "application/vnd.android.package-archive",
            // Unknown
            MediaFormat::UnknownFormat => "application/octet-stream",
        }
    }

    /// Returns the `MediaGroup` that this format belongs to.
    pub fn group(&self) -> MediaGroup {
        match self {
            // Audio formats
            MediaFormat::MP3 | MediaFormat::FLAC | MediaFormat::AAC |
            MediaFormat::WAV | MediaFormat::AIFF | MediaFormat::ALAC |
            MediaFormat::OGG | MediaFormat::OPUS | MediaFormat::WMA |
            MediaFormat::M4A | MediaFormat::M4B | MediaFormat::APE |
            MediaFormat::WV  | MediaFormat::MPC  | MediaFormat::TTA |
            MediaFormat::DSF | MediaFormat::DFF  | MediaFormat::AMR |
            MediaFormat::AU  | MediaFormat::RA   | MediaFormat::MID |
            MediaFormat::SPC | MediaFormat::MOD  | MediaFormat::S3M |
            MediaFormat::XM  | MediaFormat::IT   | MediaFormat::CAF |
            MediaFormat::AC3 | MediaFormat::DTS => MediaGroup::Audio,

            // Video formats
            MediaFormat::MKV  | MediaFormat::MP4     | MediaFormat::AVI |
            MediaFormat::MOV  | MediaFormat::WMV     | MediaFormat::WEBM |
            MediaFormat::FLV  | MediaFormat::M4V     | MediaFormat::TS |
            MediaFormat::MPG  | MediaFormat::MPEG    | MediaFormat::ThreeGP |
            MediaFormat::RM   | MediaFormat::RMVB    | MediaFormat::VOB |
            MediaFormat::OGV  | MediaFormat::ASF     | MediaFormat::MXF |
            MediaFormat::MK3D | MediaFormat::NSV     | MediaFormat::F4V => MediaGroup::Video,

            // Image formats
            MediaFormat::JPG  | MediaFormat::PNG  | MediaFormat::GIF |
            MediaFormat::BMP  | MediaFormat::TIFF | MediaFormat::WEBP |
            MediaFormat::SVG  | MediaFormat::HEIF | MediaFormat::HEIC |
            MediaFormat::AVIF | MediaFormat::PSD  | MediaFormat::TGA |
            MediaFormat::ICO  | MediaFormat::CR2  | MediaFormat::NEF |
            MediaFormat::DNG  | MediaFormat::ARW  | MediaFormat::ORF |
            MediaFormat::RAF  | MediaFormat::RW2  | MediaFormat::RAW |
            MediaFormat::EXR  | MediaFormat::HDR  | MediaFormat::PPM |
            MediaFormat::PGM  | MediaFormat::JP2  | MediaFormat::JXL => MediaGroup::Image,

            // Document formats
            MediaFormat::PDF  | MediaFormat::EPUB | MediaFormat::MOBI |
            MediaFormat::DOC  | MediaFormat::DOCX | MediaFormat::XLS |
            MediaFormat::XLSX | MediaFormat::PPT  | MediaFormat::PPTX |
            MediaFormat::ODT  | MediaFormat::ODS  | MediaFormat::ODP |
            MediaFormat::RTF  | MediaFormat::TXT  | MediaFormat::CSV |
            MediaFormat::HTML | MediaFormat::XML  | MediaFormat::JSON |
            MediaFormat::YAML | MediaFormat::MD   | MediaFormat::TEX |
            MediaFormat::DJVU | MediaFormat::CBZ  | MediaFormat::CBR |
            MediaFormat::FB2  | MediaFormat::SRT  | MediaFormat::ASS |
            MediaFormat::VTT => MediaGroup::Document,

            // Archive formats
            MediaFormat::ZIP    | MediaFormat::RAR | MediaFormat::SevenZ |
            MediaFormat::TAR    | MediaFormat::GZ  | MediaFormat::BZ2 |
            MediaFormat::XZ     | MediaFormat::ZST | MediaFormat::LZ4 |
            MediaFormat::ISO    | MediaFormat::DMG | MediaFormat::MSI |
            MediaFormat::DEB    | MediaFormat::RPM | MediaFormat::PKG |
            MediaFormat::JAR    | MediaFormat::APK => MediaGroup::Archive,

            // Unknown
            MediaFormat::UnknownFormat => MediaGroup::Unknown,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Level 3: MediaClass — content type / purpose
// ─────────────────────────────────────────────────────────────────────

/// Content classification describing what the media *is* rather than
/// what container it uses.  This level typically requires metadata
/// inspection or user tagging to populate accurately; extension-based
/// classification defaults to `Unknown`.
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
            MediaClass::Music       => write!(f, "Music"),
            MediaClass::Podcast     => write!(f, "Podcast"),
            MediaClass::Audiobook   => write!(f, "Audiobook"),
            MediaClass::Movie       => write!(f, "Movie"),
            MediaClass::TVShow      => write!(f, "TV Show"),
            MediaClass::MusicVideo  => write!(f, "Music Video"),
            MediaClass::Concert     => write!(f, "Concert"),
            MediaClass::Documentary => write!(f, "Documentary"),
            MediaClass::HomeVideo   => write!(f, "Home Video"),
            MediaClass::Ringtone    => write!(f, "Ringtone"),
            MediaClass::SoundEffect => write!(f, "Sound Effect"),
            MediaClass::Unknown     => write!(f, "Unknown"),
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
            MediaQuality::Lossless  => write!(f, "Lossless"),
            MediaQuality::HiRes    => write!(f, "Hi-Res"),
            MediaQuality::Lossy320 => write!(f, "320 kbps"),
            MediaQuality::Lossy256 => write!(f, "256 kbps"),
            MediaQuality::Lossy192 => write!(f, "192 kbps"),
            MediaQuality::Lossy128 => write!(f, "128 kbps"),
            MediaQuality::LossyLow => write!(f, "Low Quality"),
            MediaQuality::Standard => write!(f, "Standard"),
            MediaQuality::Unknown  => write!(f, "Unknown"),
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
        Self { group, format, class, quality }
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
        "mp3"                       => MediaFormat::MP3,
        "flac"                      => MediaFormat::FLAC,
        "aac"                       => MediaFormat::AAC,
        "wav"                       => MediaFormat::WAV,
        "aiff" | "aif" | "aifc"    => MediaFormat::AIFF,
        "alac"                      => MediaFormat::ALAC,
        "ogg" | "oga"              => MediaFormat::OGG,
        "opus"                      => MediaFormat::OPUS,
        "wma"                       => MediaFormat::WMA,
        "m4a"                       => MediaFormat::M4A,
        "m4b"                       => MediaFormat::M4B,
        "ape"                       => MediaFormat::APE,
        "wv"                        => MediaFormat::WV,
        "mpc" | "mp+" | "mpp"      => MediaFormat::MPC,
        "tta"                       => MediaFormat::TTA,
        "dsf"                       => MediaFormat::DSF,
        "dff"                       => MediaFormat::DFF,
        "amr"                       => MediaFormat::AMR,
        "au" | "snd"               => MediaFormat::AU,
        "ra" | "ram"               => MediaFormat::RA,
        "mid" | "midi" | "kar"     => MediaFormat::MID,
        "spc"                       => MediaFormat::SPC,
        "mod"                       => MediaFormat::MOD,
        "s3m"                       => MediaFormat::S3M,
        "xm"                        => MediaFormat::XM,
        "it"                        => MediaFormat::IT,
        "caf"                       => MediaFormat::CAF,
        "ac3"                       => MediaFormat::AC3,
        "dts"                       => MediaFormat::DTS,

        // ── Video ──────────────────────────────────────────────────
        "mkv"                       => MediaFormat::MKV,
        "mp4" | "m4p"              => MediaFormat::MP4,
        "avi"                       => MediaFormat::AVI,
        "mov" | "qt"               => MediaFormat::MOV,
        "wmv"                       => MediaFormat::WMV,
        "webm"                      => MediaFormat::WEBM,
        "flv"                       => MediaFormat::FLV,
        "m4v"                       => MediaFormat::M4V,
        "ts" | "mts" | "m2ts"     => MediaFormat::TS,
        "mpg" | "mp2"              => MediaFormat::MPG,
        "mpeg"                      => MediaFormat::MPEG,
        "3gp" | "3g2" | "3gpp"    => MediaFormat::ThreeGP,
        "rm"                        => MediaFormat::RM,
        "rmvb"                      => MediaFormat::RMVB,
        "vob"                       => MediaFormat::VOB,
        "ogv"                       => MediaFormat::OGV,
        "asf"                       => MediaFormat::ASF,
        "mxf"                       => MediaFormat::MXF,
        "mk3d"                      => MediaFormat::MK3D,
        "nsv"                       => MediaFormat::NSV,
        "f4v"                       => MediaFormat::F4V,

        // ── Image ──────────────────────────────────────────────────
        "jpg" | "jpeg" | "jpe" | "jfif" => MediaFormat::JPG,
        "png"                       => MediaFormat::PNG,
        "gif"                       => MediaFormat::GIF,
        "bmp" | "dib"              => MediaFormat::BMP,
        "tiff" | "tif"             => MediaFormat::TIFF,
        "webp"                      => MediaFormat::WEBP,
        "svg" | "svgz"             => MediaFormat::SVG,
        "heif" | "hif"             => MediaFormat::HEIF,
        "heic"                      => MediaFormat::HEIC,
        "avif"                      => MediaFormat::AVIF,
        "psd"                       => MediaFormat::PSD,
        "tga" | "icb" | "vda" | "vst" => MediaFormat::TGA,
        "ico"                       => MediaFormat::ICO,
        "cr2"                       => MediaFormat::CR2,
        "nef" | "nrw"              => MediaFormat::NEF,
        "dng"                       => MediaFormat::DNG,
        "arw" | "srf" | "sr2"     => MediaFormat::ARW,
        "orf"                       => MediaFormat::ORF,
        "raf"                       => MediaFormat::RAF,
        "rw2"                       => MediaFormat::RW2,
        "raw"                       => MediaFormat::RAW,
        "exr"                       => MediaFormat::EXR,
        "hdr"                       => MediaFormat::HDR,
        "ppm"                       => MediaFormat::PPM,
        "pgm"                       => MediaFormat::PGM,
        "jp2" | "j2k" | "jpf" | "jpx" => MediaFormat::JP2,
        "jxl"                       => MediaFormat::JXL,

        // ── Document ───────────────────────────────────────────────
        "pdf"                       => MediaFormat::PDF,
        "epub"                      => MediaFormat::EPUB,
        "mobi" | "prc" | "azw" | "azw3" | "kfx" => MediaFormat::MOBI,
        "doc"                       => MediaFormat::DOC,
        "docx"                      => MediaFormat::DOCX,
        "xls"                       => MediaFormat::XLS,
        "xlsx"                      => MediaFormat::XLSX,
        "ppt"                       => MediaFormat::PPT,
        "pptx"                      => MediaFormat::PPTX,
        "odt"                       => MediaFormat::ODT,
        "ods"                       => MediaFormat::ODS,
        "odp"                       => MediaFormat::ODP,
        "rtf"                       => MediaFormat::RTF,
        "txt" | "text" | "log"     => MediaFormat::TXT,
        "csv" | "tsv"              => MediaFormat::CSV,
        "html" | "htm" | "xhtml"   => MediaFormat::HTML,
        "xml" | "xsl" | "xslt"    => MediaFormat::XML,
        "json" | "jsonl" | "json5" => MediaFormat::JSON,
        "yaml" | "yml"             => MediaFormat::YAML,
        "md" | "markdown"          => MediaFormat::MD,
        "tex" | "latex"            => MediaFormat::TEX,
        "djvu" | "djv"             => MediaFormat::DJVU,
        "cbz"                       => MediaFormat::CBZ,
        "cbr"                       => MediaFormat::CBR,
        "fb2"                       => MediaFormat::FB2,
        "srt"                       => MediaFormat::SRT,
        "ass" | "ssa"              => MediaFormat::ASS,
        "vtt"                       => MediaFormat::VTT,

        // ── Archive ────────────────────────────────────────────────
        "zip"                       => MediaFormat::ZIP,
        "rar"                       => MediaFormat::RAR,
        "7z"                        => MediaFormat::SevenZ,
        "tar"                       => MediaFormat::TAR,
        "gz" | "gzip"              => MediaFormat::GZ,
        "bz2" | "bzip2"           => MediaFormat::BZ2,
        "xz" | "lzma"             => MediaFormat::XZ,
        "zst" | "zstd"            => MediaFormat::ZST,
        "lz4"                       => MediaFormat::LZ4,
        "iso"                       => MediaFormat::ISO,
        "dmg"                       => MediaFormat::DMG,
        "msi"                       => MediaFormat::MSI,
        "deb"                       => MediaFormat::DEB,
        "rpm"                       => MediaFormat::RPM,
        "pkg"                       => MediaFormat::PKG,
        "jar"                       => MediaFormat::JAR,
        "apk"                       => MediaFormat::APK,

        // ── Compound archive extensions (tar.gz etc.) ──────────────
        // These are handled if the caller strips the outer extension;
        // "tgz" is a common single-extension alias for tar+gzip.
        "tgz"                       => MediaFormat::GZ,
        "tbz2" | "tbz"            => MediaFormat::BZ2,
        "txz"                       => MediaFormat::XZ,

        // ── Unrecognised ───────────────────────────────────────────
        _                           => MediaFormat::UnknownFormat,
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
        MediaGroup::Audio   => MediaQuality::Unknown,
        MediaGroup::Unknown => MediaQuality::Unknown,
        _                   => MediaQuality::Standard,
    };

    MediaClassification { group, format, class, quality }
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
        .extension()                          // Option<&OsStr>
        .and_then(|e| e.to_str())             // Option<&str> (lossy-free)
        .ok_or_else(|| {
            MmError::Classify(format!(
                "path has no file extension: {}",
                path.display()
            ))
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
            MediaFormat::MP3, MediaFormat::FLAC, MediaFormat::AAC,
            MediaFormat::WAV, MediaFormat::MKV, MediaFormat::MP4,
            MediaFormat::JPG, MediaFormat::PNG, MediaFormat::PDF,
            MediaFormat::ZIP,
        ];
        for fmt in formats {
            let ext = fmt.extension();
            let classified = classify_by_extension(ext);
            assert_eq!(classified.format, fmt, "Roundtrip failed for {}", fmt);
        }
    }

    #[test]
    fn format_mime_types() {
        assert_eq!(MediaFormat::MP3.mime_type(), "audio/mpeg");
        assert_eq!(MediaFormat::MP4.mime_type(), "video/mp4");
        assert_eq!(MediaFormat::JPG.mime_type(), "image/jpeg");
        assert_eq!(MediaFormat::PDF.mime_type(), "application/pdf");
        assert_eq!(MediaFormat::ZIP.mime_type(), "application/zip");
        assert_eq!(MediaFormat::UnknownFormat.mime_type(), "application/octet-stream");
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
        let restored: MediaClassification =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }
}
