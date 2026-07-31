//! Media typing for uploads (CMS-R6, CMS-D9) — pure, DB-free, and
//! never panicking on hostile bytes.
//!
//! An upload endpoint is an internet-facing file sink, so what a caller
//! *says* a file is carries no weight. Three rules follow, and all three
//! are enforced here:
//!
//! 1. **Sniff, then compare.** The type is derived from the leading
//!    bytes; a declared `Content-Type` that disagrees is a refusal, not
//!    a correction. (Trusting the declaration is how a `.png` that is
//!    really an HTML document gets served back as one.)
//! 2. **Allow-list, never deny-list.** A format nobody recognised is
//!    refused. A deny-list is a promise to have thought of every
//!    dangerous format in advance, which nobody can keep.
//! 3. **Nothing script-bearing.** HTML, and anything that can carry
//!    active content, is refused outright.
//!
//! ## Why SVG is refused in v1
//!
//! The spec (`assets.md`) allows SVG **when sanitized**. This
//! implementation refuses it instead, and the difference is deliberate:
//! an SVG sanitizer is not an HTML sanitizer. SVG carries script through
//! `<script>`, `on*` handlers, `<foreignObject>`, animated `href`s, and
//! external entity references — running it through
//! [`crate::rules::sanitize`] (an HTML5 tree sanitizer) would *look*
//! like protection while leaving a real attack surface, which is
//! precisely the "unverified security code that looks finished" this
//! project refuses elsewhere. Accepting SVG needs a purpose-built
//! sanitizer and its own round; until then the honest answer is `422`
//! with a reason, not a false sense of safety.
//!
//! Parsers here index with `get`, never `[]`, because every byte is
//! attacker-supplied.

/// One accepted media type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaType {
    /// The canonical MIME type.
    pub mime: &'static str,
    /// The asset kind this maps to ([`crate::rules::tokens::ASSET_KINDS`]).
    pub kind: &'static str,
    /// The conventional file extension, for display only.
    pub extension: &'static str,
}

/// Every media type this service accepts. Anything not on this list is
/// refused, including formats that are merely unrecognised.
pub const ACCEPTED: &[MediaType] = &[
    MediaType {
        mime: "image/png",
        kind: "image",
        extension: "png",
    },
    MediaType {
        mime: "image/jpeg",
        kind: "image",
        extension: "jpg",
    },
    MediaType {
        mime: "image/gif",
        kind: "image",
        extension: "gif",
    },
    MediaType {
        mime: "image/webp",
        kind: "image",
        extension: "webp",
    },
    MediaType {
        mime: "video/mp4",
        kind: "video",
        extension: "mp4",
    },
    MediaType {
        mime: "video/webm",
        kind: "video",
        extension: "webm",
    },
    MediaType {
        mime: "audio/mpeg",
        kind: "audio",
        extension: "mp3",
    },
    MediaType {
        mime: "audio/wav",
        kind: "audio",
        extension: "wav",
    },
    MediaType {
        mime: "application/pdf",
        kind: "document",
        extension: "pdf",
    },
];

/// Types that are recognisable but **refused**, each with the reason a
/// caller gets. Naming them explicitly beats a bare "unsupported":
/// an editor who uploaded an SVG logo needs to know why, and that the
/// answer may change.
pub const REFUSED: &[(&str, &str)] = &[
    (
        "image/svg+xml",
        "SVG can carry script (<script>, on* handlers, <foreignObject>); accepting it needs a \
         purpose-built SVG sanitizer, which this version does not have. Upload a raster export \
         (PNG/WebP) instead.",
    ),
    (
        "text/html",
        "HTML is executable in a browser context and is never stored as an asset.",
    ),
    (
        "application/zip",
        "Archives hide their contents from every check above; upload the files themselves.",
    ),
    (
        "application/x-executable",
        "Executables are never valid content.",
    ),
];

/// Look up an accepted type by MIME.
#[must_use]
pub fn accepted(mime: &str) -> Option<&'static MediaType> {
    ACCEPTED.iter().find(|m| m.mime == mime)
}

/// Sniff the media type from the leading bytes.
///
/// Returns the detected MIME, whether or not it is accepted — the
/// caller distinguishes "recognised and refused" from "not recognised
/// at all", which are different answers to give an editor.
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    let starts = |prefix: &[u8]| bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix;

    if starts(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if starts(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if starts(b"GIF87a") || starts(b"GIF89a") {
        return Some("image/gif");
    }
    if starts(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return Some("image/webp");
    }
    if starts(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        return Some("audio/wav");
    }
    if starts(b"%PDF-") {
        return Some("application/pdf");
    }
    if starts(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Some("video/webm");
    }
    // ISO base media (MP4): `....ftyp` at offset 4.
    if bytes.get(4..8) == Some(b"ftyp") {
        return Some("video/mp4");
    }
    if starts(b"ID3") || starts(&[0xFF, 0xFB]) || starts(&[0xFF, 0xF3]) || starts(&[0xFF, 0xF2]) {
        return Some("audio/mpeg");
    }
    // Text-ish formats worth *recognising* so they can be refused with a
    // reason rather than a shrug.
    let head = &bytes[..bytes.len().min(1024)];
    let text = String::from_utf8_lossy(head);
    let lowered = text.trim_start().to_ascii_lowercase();
    if lowered.starts_with("<svg") || (lowered.starts_with("<?xml") && lowered.contains("<svg")) {
        return Some("image/svg+xml");
    }
    if lowered.starts_with("<!doctype html") || lowered.starts_with("<html") {
        return Some("text/html");
    }
    if starts(b"PK\x03\x04") {
        return Some("application/zip");
    }
    if starts(b"\x7fELF") || starts(b"MZ") {
        return Some("application/x-executable");
    }
    None
}

/// The outcome of typing an upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Accepted, with the canonical type.
    Accepted(&'static MediaType),
    /// Recognised but refused, with the reason.
    Refused(String),
}

/// Type an upload: sniff the bytes, compare with what the caller
/// declared, and decide.
///
/// A `declared` MIME that disagrees with the sniffed one is refused
/// even when *both* are acceptable types — the mismatch itself is the
/// signal, and honouring the bytes silently would leave the stored
/// metadata lying about its own content.
#[must_use]
pub fn classify(bytes: &[u8], declared: Option<&str>) -> Verdict {
    if bytes.is_empty() {
        return Verdict::Refused("the upload is empty".to_string());
    }
    let Some(sniffed) = sniff(bytes) else {
        return Verdict::Refused(
            "the file's format was not recognised; accepted types are PNG, JPEG, GIF, WebP, MP4, \
             WebM, MP3, WAV, and PDF"
                .to_string(),
        );
    };
    if let Some((_, reason)) = REFUSED.iter().find(|(mime, _)| *mime == sniffed) {
        return Verdict::Refused(format!("{sniffed} is not accepted: {reason}"));
    }
    let Some(media) = accepted(sniffed) else {
        return Verdict::Refused(format!("{sniffed} is not an accepted media type"));
    };
    if let Some(declared) = declared {
        let declared = declared.split(';').next().unwrap_or("").trim();
        if !declared.is_empty() && !declared.eq_ignore_ascii_case(sniffed) {
            return Verdict::Refused(format!(
                "declared content type {declared:?} does not match the file's actual content \
                 ({sniffed})"
            ));
        }
    }
    Verdict::Accepted(media)
}

/// Intrinsic pixel dimensions, where the format states them in a header
/// this can read without decoding the image.
///
/// Returns `None` rather than guessing — a missing dimension is honest,
/// a wrong one silently breaks every layout that trusts it. Nothing here
/// decodes pixel data, so a malformed file costs a bounds check, not a
/// decoder bug.
#[must_use]
pub fn dimensions(bytes: &[u8], mime: &str) -> Option<(u32, u32)> {
    match mime {
        "image/png" => {
            // IHDR width/height are big-endian u32 at offsets 16 and 20.
            let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
            let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
            (width > 0 && height > 0).then_some((width, height))
        }
        "image/gif" => {
            // Logical screen descriptor: little-endian u16 at 6 and 8.
            let width = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?);
            let height = u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?);
            (width > 0 && height > 0).then_some((u32::from(width), u32::from(height)))
        }
        "image/jpeg" => jpeg_dimensions(bytes),
        _ => None,
    }
}

/// Walk JPEG segments to the first start-of-frame marker.
///
/// Bounded by the input length and by a segment-count cap, so a file of
/// nothing but zero-length markers terminates instead of spinning.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const MAX_SEGMENTS: usize = 512;
    let mut index = 2; // skip SOI
    for _ in 0..MAX_SEGMENTS {
        if *bytes.get(index)? != 0xFF {
            return None;
        }
        let marker = *bytes.get(index + 1)?;
        // SOF0..SOF15, excluding the non-frame markers DHT/JPG/DAC.
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let height = u16::from_be_bytes(bytes.get(index + 5..index + 7)?.try_into().ok()?);
            let width = u16::from_be_bytes(bytes.get(index + 7..index + 9)?.try_into().ok()?);
            return (width > 0 && height > 0).then_some((u32::from(width), u32::from(height)));
        }
        let length = u16::from_be_bytes(bytes.get(index + 2..index + 4)?.try_into().ok()?);
        // A zero-length segment would not advance the cursor.
        if length < 2 {
            return None;
        }
        index = index.checked_add(2)?.checked_add(usize::from(length))?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    fn jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8]; // SOI
        // An APP0 segment to prove the walk skips it.
        bytes.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00]);
        bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]); // SOF0
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes
    }

    #[test]
    fn accepted_formats_are_sniffed() {
        assert_eq!(sniff(&png(1, 1)), Some("image/png"));
        assert_eq!(sniff(&jpeg(1, 1)), Some("image/jpeg"));
        assert_eq!(sniff(b"GIF89a\x01\x00\x01\x00"), Some("image/gif"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WAVEfmt "), Some("audio/wav"));
        assert_eq!(sniff(b"%PDF-1.7\n"), Some("application/pdf"));
        assert_eq!(sniff(b"\x1a\x45\xdf\xa3"), Some("video/webm"));
        assert_eq!(sniff(b"\0\0\0\x18ftypmp42"), Some("video/mp4"));
        assert_eq!(sniff(b"ID3\x03\0"), Some("audio/mpeg"));
    }

    /// Dangerous formats are *recognised*, so they can be refused with a
    /// reason rather than an unhelpful shrug.
    #[test]
    fn dangerous_formats_are_recognised_and_refused() {
        for (bytes, mime) in [
            (
                b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".as_slice(),
                "image/svg+xml",
            ),
            (b"<!DOCTYPE html><html></html>".as_slice(), "text/html"),
            (b"PK\x03\x04".as_slice(), "application/zip"),
            (b"\x7fELF\x02\x01".as_slice(), "application/x-executable"),
        ] {
            assert_eq!(sniff(bytes), Some(mime));
            match classify(bytes, None) {
                Verdict::Refused(reason) => assert!(
                    reason.contains(mime),
                    "the refusal names the type: {reason}"
                ),
                Verdict::Accepted(_) => panic!("{mime} must never be accepted"),
            }
        }
    }

    /// The SVG refusal explains itself and points at the alternative —
    /// an editor with a logo needs to know what to do next.
    #[test]
    fn the_svg_refusal_is_explained() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
        let Verdict::Refused(reason) = classify(svg, Some("image/svg+xml")) else {
            panic!("SVG must be refused");
        };
        assert!(reason.contains("script"));
        assert!(reason.contains("PNG/WebP"));
    }

    /// A declaration that disagrees with the bytes is the signal; both
    /// being acceptable types does not make the mismatch acceptable.
    #[test]
    fn a_declaration_that_disagrees_with_the_bytes_is_refused() {
        let bytes = png(2, 2);
        assert!(matches!(
            classify(&bytes, Some("image/png")),
            Verdict::Accepted(_)
        ));
        assert!(matches!(
            classify(&bytes, Some("image/png; charset=binary")),
            Verdict::Accepted(_)
        ));
        let Verdict::Refused(reason) = classify(&bytes, Some("image/jpeg")) else {
            panic!("a mismatch must be refused");
        };
        assert!(reason.contains("does not match the file's actual content"));
        // No declaration at all is fine: the bytes decide.
        assert!(matches!(classify(&bytes, None), Verdict::Accepted(_)));
    }

    #[test]
    fn unrecognised_and_empty_uploads_are_refused() {
        let Verdict::Refused(reason) = classify(b"\x00\x01\x02\x03 random", None) else {
            panic!("unknown formats are refused");
        };
        assert!(reason.contains("not recognised"));
        assert!(matches!(classify(b"", None), Verdict::Refused(_)));
    }

    #[test]
    fn dimensions_are_read_from_headers() {
        assert_eq!(
            dimensions(&png(1920, 1080), "image/png"),
            Some((1920, 1080))
        );
        assert_eq!(dimensions(&jpeg(800, 600), "image/jpeg"), Some((800, 600)));
        assert_eq!(
            dimensions(b"GIF89a\x20\x00\x10\x00", "image/gif"),
            Some((32, 16))
        );
        // A format whose dimensions we do not read reports none rather
        // than guessing.
        assert_eq!(dimensions(b"%PDF-1.7", "application/pdf"), None);
    }

    /// Truncated and malformed files return `None`; nothing panics,
    /// overflows, or loops. Every byte here is attacker-supplied.
    #[test]
    fn malformed_headers_never_panic() {
        let cases: Vec<Vec<u8>> = vec![
            b"\x89PNG\r\n\x1a\n".to_vec(),            // truncated before IHDR
            png(0, 0),                                // zero dimensions
            vec![0xFF, 0xD8],                         // JPEG with no segments
            vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x02], // SOF too short
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00], // zero-length segment
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0xFF, 0xFF], // length running past the end
            b"GIF89a".to_vec(),                       // truncated GIF
            vec![0xFF; 4096],                         // marker soup
        ];
        for bytes in cases {
            for mime in ["image/png", "image/jpeg", "image/gif"] {
                let _ = dimensions(&bytes, mime);
            }
            let _ = sniff(&bytes);
            let _ = classify(&bytes, None);
        }
    }

    /// Every accepted type maps to a known asset kind.
    #[test]
    fn accepted_types_map_to_known_kinds() {
        for media in ACCEPTED {
            assert!(
                crate::rules::tokens::ASSET_KINDS.contains(&media.kind),
                "{} maps to unknown kind {}",
                media.mime,
                media.kind
            );
        }
    }
}
