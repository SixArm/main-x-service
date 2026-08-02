//! JSONL codec — the lossless reference format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! One bulk row (an optional `pid` plus every `Organization` field — see
//! [`super::columns`]) per line, so a JSONL round-trip is lossless
//! including nested identifiers / address / arrays. Reading is
//! line-oriented: the caller iterates lines and parses each with
//! [`parse_line`], recording per-row parse errors (§7) rather than
//! aborting the whole file.

use uuid::Uuid;

use loco_rs::Error;
use organization_matcher::Organization;

use super::columns;

/// Serialize one bulk row to a single JSONL line (no trailing newline).
///
/// # Errors
///
/// When the organization fails to serialize (see [`columns::to_row_value`]).
pub fn to_line(pid: Option<Uuid>, org: &Organization) -> loco_rs::Result<String> {
    let value = columns::to_row_value(pid, org)
        .map_err(|e| Error::Message(format!("serialize organization to JSONL: {e}")))?;
    Ok(value.to_string())
}

/// Encode a slice of `(pid, organization)` rows to a JSONL byte buffer
/// (newline-terminated lines), the export output shape.
///
/// # Errors
///
/// When any row fails to serialize.
pub fn encode(rows: &[(Option<Uuid>, Organization)]) -> loco_rs::Result<Vec<u8>> {
    let mut out = Vec::new();
    for (pid, org) in rows {
        out.extend_from_slice(to_line(*pid, org)?.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

/// Parse one JSONL line into `(had_explicit_pid, pid, Organization)`.
///
/// # Errors
///
/// A per-row error message (§7 contract) when the line is not valid JSON,
/// or [`columns::from_row_value`] rejects it (a malformed `pid`, or a
/// shape that does not deserialize into an `Organization`).
pub fn parse_line(line: &str) -> Result<(bool, Option<Uuid>, Organization), String> {
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
    use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization, PostalAddress};
    use uuid::Uuid;

    fn sample(name: &str) -> Organization {
        Organization {
            legal_name: Some(format!("{name} Incorporated")),
            identifiers: vec![OrgIdentifier {
                scheme: IdentifierScheme::Lei,
                value: "5493001KJTIIGC8Y1R12".to_string(),
            }],
            address: Some(PostalAddress {
                locality: Some("Sedona".to_string()),
                country: Some("US".to_string()),
                ..PostalAddress::default()
            }),
            ..Organization::new(name)
        }
    }

    #[test]
    fn line_round_trips_losslessly() {
        let pid = Uuid::new_v4();
        let org = sample("Acme");
        let line = to_line(Some(pid), &org).unwrap();
        assert!(!line.contains('\n'));
        let (had_explicit_pid, parsed_pid, back) = parse_line(&line).unwrap();
        assert!(had_explicit_pid);
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(back.name, "Acme");
        assert_eq!(back.legal_name.as_deref(), Some("Acme Incorporated"));
        assert_eq!(back.identifiers.len(), 1);
        assert_eq!(back.identifiers[0].value, "5493001KJTIIGC8Y1R12");
        assert_eq!(back.address.unwrap().locality.as_deref(), Some("Sedona"));
    }

    #[test]
    fn encode_then_split_yields_one_line_per_row() {
        let rows = vec![(None, sample("A")), (Some(Uuid::new_v4()), sample("B"))];
        let bytes = encode(&rows).unwrap();
        let lines = split_lines(&bytes).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(parse_line(&lines[0]).unwrap().2.name, "A");
        assert_eq!(parse_line(&lines[1]).unwrap().2.name, "B");
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
