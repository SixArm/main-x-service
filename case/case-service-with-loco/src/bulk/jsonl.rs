//! JSONL codec — the lossless reference format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! One [`BulkCaseRow`] per line — `{"pid": "...", "title": "...", …}` —
//! so a JSONL round-trip is lossless including nested identifiers /
//! subjects / keywords / the `Custom`-carrying enum variants. Reading is
//! line-oriented: the caller iterates lines and parses each with
//! [`parse_line`], recording per-row parse errors (§7) rather than
//! aborting the whole file.

use super::row::BulkCaseRow;

/// Serialize one row to a single JSONL line (no trailing newline).
///
/// # Errors
///
/// Returns an error if the row fails to serialize.
pub fn to_line(row: &BulkCaseRow) -> loco_rs::Result<String> {
    serde_json::to_string(row)
        .map_err(|e| loco_rs::Error::Message(format!("serialize row to JSONL: {e}")))
}

/// Parse one JSONL line into a [`BulkCaseRow`].
///
/// The raw `serde_json` error is preserved so callers can surface it in
/// the per-row error report.
///
/// # Errors
///
/// Returns the underlying [`serde_json::Error`] on malformed input.
pub fn parse_line(line: &str) -> serde_json::Result<BulkCaseRow> {
    serde_json::from_str(line)
}

/// Encode a slice of rows to a JSONL byte buffer (newline-terminated
/// lines), the export output shape.
///
/// # Errors
///
/// Returns an error if any row fails to serialize.
pub fn encode(rows: &[BulkCaseRow]) -> loco_rs::Result<Vec<u8>> {
    let mut out = Vec::new();
    for row in rows {
        out.extend_from_slice(to_line(row)?.as_bytes());
        out.push(b'\n');
    }
    Ok(out)
}

/// Split a JSONL byte buffer into its non-empty text lines, ignoring
/// blank lines. Returns an error if the bytes are not valid UTF-8.
///
/// # Errors
///
/// Returns an error if `input` is not valid UTF-8.
pub fn split_lines(input: &[u8]) -> loco_rs::Result<Vec<String>> {
    let text = std::str::from_utf8(input)
        .map_err(|e| loco_rs::Error::Message(format!("bulk input is not valid UTF-8: {e}")))?;
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
/// Returns an error if `input` is not valid UTF-8 (via [`split_lines`])
/// or if the row count exceeds `max`.
pub fn split_lines_capped(input: &[u8], max: usize) -> loco_rs::Result<Vec<String>> {
    let lines = split_lines(input)?;
    if lines.len() > max {
        return Err(loco_rs::Error::Message(format!(
            "bulk import exceeds the row cap: {} rows > {max}",
            lines.len()
        )));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{encode, parse_line, split_lines, to_line};
    use crate::bulk::row::BulkCaseRow;
    use case_matcher::Case;
    use uuid::Uuid;

    fn sample(title: &str) -> BulkCaseRow {
        BulkCaseRow::with_pid(Uuid::new_v4(), Case::new(title))
    }

    #[test]
    fn line_round_trips_losslessly() {
        let row = sample("Housing benefit appeal");
        let line = to_line(&row).unwrap();
        assert!(!line.contains('\n'));
        let back = parse_line(&line).unwrap();
        assert_eq!(back.pid, row.pid);
        assert_eq!(back.case.title, "Housing benefit appeal");
    }

    #[test]
    fn encode_then_split_yields_one_line_per_row() {
        let rows = vec![sample("A"), sample("B")];
        let bytes = encode(&rows).unwrap();
        let lines = split_lines(&bytes).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(parse_line(&lines[0]).unwrap().case.title, "A");
        assert_eq!(parse_line(&lines[1]).unwrap().case.title, "B");
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

    /// A line naming only `title` (`case_matcher::Case`'s one required
    /// field — it carries no `#[serde(default)]`, so a wholly bare `{}`
    /// fails to *parse*, not merely to validate) is a *valid* keyless row:
    /// every other field defaults, `pid` to `None`. A blank `title` still
    /// parses fine here — it fails the service's own `title`-required
    /// validation later in the pipeline, not at parse time.
    #[test]
    fn a_minimal_object_parses_as_a_keyless_row_with_default_fields() {
        let row = parse_line(r#"{"title":""}"#).unwrap();
        assert!(row.pid.is_none());
        assert_eq!(row.case.title, "");
    }

    #[test]
    fn split_lines_capped_rejects_over_the_cap() {
        let bytes = b"{}\n{}\n{}\n";
        assert!(super::split_lines_capped(bytes, 2).is_err());
        assert_eq!(super::split_lines_capped(bytes, 3).unwrap().len(), 3);
    }

    #[test]
    fn split_lines_capped_ignores_blanks_when_counting() {
        let bytes = b"\n{}\n  \n{}\n\n";
        assert_eq!(super::split_lines_capped(bytes, 2).unwrap().len(), 2);
    }

    #[test]
    fn parse_line_handles_a_giant_line_without_panicking() {
        let giant = "a".repeat(2 * 1024 * 1024);
        assert!(parse_line(&giant).is_err());
        assert_eq!(split_lines(giant.as_bytes()).unwrap().len(), 1);
    }

    #[test]
    fn split_lines_handles_truncated_utf8_without_panicking() {
        let mut bytes = b"{}\n".to_vec();
        bytes.push(0xF0); // start of a 4-byte sequence, then EOF
        assert!(split_lines(&bytes).is_err());
    }
}
