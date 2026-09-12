//! JSONL codec — the lossless reference format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! One bulk row (an optional `pid` + an optional `active` flag + every
//! `CarePathway` field — see [`super::columns`]) per line, so a JSONL
//! round-trip is lossless including nested identifiers / condition codes
//! / relationships / arrays. Reading is line-oriented: the caller
//! iterates lines and parses each with [`parse_line`], recording per-row
//! parse errors (§7) rather than aborting the whole file.

use uuid::Uuid;

use care_pathway_matcher::CarePathway;
use loco_rs::Error;

use super::columns;

/// Serialize one bulk row to a single JSONL line (no trailing newline).
///
/// # Errors
///
/// When the pathway fails to serialize (see [`columns::to_row_value`]).
pub fn to_line(
    pid: Option<Uuid>,
    active: Option<bool>,
    pathway: &CarePathway,
) -> loco_rs::Result<String> {
    let value = columns::to_row_value(pid, active, pathway)
        .map_err(|e| Error::Message(format!("serialize care pathway to JSONL: {e}")))?;
    Ok(value.to_string())
}

/// Encode a slice of `(pid, active, pathway)` rows to a JSONL byte buffer
/// (newline-terminated lines), the export output shape.
///
/// # Errors
///
/// When any row fails to serialize.
pub fn encode(rows: &[(Option<Uuid>, Option<bool>, CarePathway)]) -> loco_rs::Result<Vec<u8>> {
    let mut out = Vec::new();
    for (pid, active, pathway) in rows {
        out.extend_from_slice(to_line(*pid, *active, pathway)?.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

/// Parse one JSONL line into `(had_explicit_pid, pid, active,
/// CarePathway)`.
///
/// # Errors
///
/// A per-row error message (§7 contract) when the line is not valid JSON,
/// or [`columns::from_row_value`] rejects it (a malformed `pid`/`active`,
/// or a shape that does not deserialize into a `CarePathway`).
pub fn parse_line(line: &str) -> Result<(bool, Option<Uuid>, Option<bool>, CarePathway), String> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
    columns::from_row_value(value)
}

/// Split a JSONL byte buffer into its non-empty text lines, ignoring
/// blank lines.
///
/// # Errors
///
/// When `input` is not valid UTF-8.
pub fn split_lines(input: &[u8]) -> loco_rs::Result<Vec<String>> {
    let text = std::str::from_utf8(input)
        .map_err(|e| Error::Message(format!("bulk input is not valid UTF-8: {e}")))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(ToString::to_string)
        .collect())
}

/// Like [`split_lines`], but reject the whole load when the non-blank row
/// count exceeds `max` (SEC-B2 row cap). Bounds the per-job work so a
/// file of millions of tiny lines cannot enqueue millions of per-row
/// database round-trips.
///
/// # Errors
///
/// When `input` is not valid UTF-8, or the row count exceeds `max`.
pub fn split_lines_capped(input: &[u8], max: usize) -> loco_rs::Result<Vec<String>> {
    let lines = split_lines(input)?;
    if lines.len() > max {
        return Err(Error::Message(format!(
            "bulk import exceeds the row cap: {} rows > {max}",
            lines.len()
        )));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{encode, parse_line, split_lines, to_line};
    use care_pathway_matcher::{CarePathway, IdentifierScheme, PathwayIdentifier};
    use uuid::Uuid;

    fn sample(name: &str) -> CarePathway {
        CarePathway {
            pathway_code: Some(format!("{name}-01")),
            identifiers: vec![PathwayIdentifier {
                scheme: IdentifierScheme::Doi,
                value: "10.1000/xyz123".to_string(),
            }],
            in_language: vec!["en".to_string()],
            ..CarePathway::new(name)
        }
    }

    #[test]
    fn line_round_trips_losslessly() {
        let pid = Uuid::new_v4();
        let pathway = sample("Acute Stroke");
        let line = to_line(Some(pid), Some(true), &pathway).unwrap();
        assert!(!line.contains('\n'));
        let (had_explicit_pid, parsed_pid, active, back) = parse_line(&line).unwrap();
        assert!(had_explicit_pid);
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(active, Some(true));
        assert_eq!(back.name, "Acute Stroke");
        assert_eq!(back.pathway_code.as_deref(), Some("Acute Stroke-01"));
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].value, "10.1000/xyz123");
        assert_eq!(back.in_language, vec!["en".to_string()]);
    }

    #[test]
    fn encode_then_split_yields_one_line_per_row() {
        let rows = vec![
            (None, None, sample("A")),
            (Some(Uuid::new_v4()), Some(false), sample("B")),
        ];
        let bytes = encode(&rows).unwrap();
        let lines = split_lines(&bytes).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(parse_line(&lines[0]).unwrap().3.name, "A");
        assert_eq!(parse_line(&lines[1]).unwrap().3.name, "B");
    }

    #[test]
    fn split_lines_ignores_blank_lines() {
        let bytes = b"\n  \n{}\n\n";
        assert_eq!(split_lines(bytes).unwrap().len(), 1);
    }

    #[test]
    fn parse_line_rejects_garbage() {
        assert!(parse_line("not json").is_err());
    }

    #[test]
    fn split_lines_capped_rejects_over_the_cap() {
        let bytes = b"{}\n{}\n{}\n";
        assert!(super::split_lines_capped(bytes, 2).is_err());
        assert_eq!(super::split_lines_capped(bytes, 3).unwrap().len(), 3);
    }

    /// SEC-B2 boundary case: a large single line is a per-row `Err`
    /// (malformed JSON), never a panic, and `split_lines` treats a giant
    /// valid-UTF-8 buffer with no newline as one line.
    #[test]
    fn parse_line_handles_a_giant_line_without_panicking() {
        let giant = "a".repeat(2 * 1024 * 1024);
        assert!(parse_line(&giant).is_err());
        assert_eq!(split_lines(giant.as_bytes()).unwrap().len(), 1);
    }

    /// `split_lines` must not panic on non-UTF-8 or otherwise adversarial
    /// byte sequences — a truncated multi-byte sequence is a clean `Err`.
    #[test]
    fn split_lines_handles_truncated_utf8_without_panicking() {
        let mut bytes = b"{}\n".to_vec();
        bytes.push(0xF0); // start of a 4-byte sequence, then EOF
        assert!(split_lines(&bytes).is_err());
    }
}
