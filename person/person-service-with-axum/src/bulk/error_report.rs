//! The per-row import error report
//! (`agents/share/bulk-import-export.md` §7).
//!
//! One valid row committing must never abort the load: invalid rows are
//! skipped and recorded here, and the report is written to a downloadable
//! artifact referenced by `bulk_jobs.error_report_url`. Each row carries
//! the source row number, the offending field (empty for a whole-line
//! parse failure), a stable machine `code`, and a human message.

use serde::{Deserialize, Serialize};

/// One rejected import row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorRow {
    /// 1-based record row number within the input file.
    pub row_number: usize,
    /// Offending field path (`""` for a whole-line parse failure).
    pub field: String,
    /// Stable machine code: `parse` | `validation` | `database`.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

impl ErrorRow {
    /// A whole-line parse failure (no specific field).
    #[must_use]
    pub fn parse(row_number: usize, message: impl Into<String>) -> Self {
        Self {
            row_number,
            field: String::new(),
            code: "parse".to_string(),
            message: message.into(),
        }
    }

    /// A field-level validation failure.
    #[must_use]
    pub fn validation(
        row_number: usize,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            row_number,
            field: field.into(),
            code: "validation".to_string(),
            message: message.into(),
        }
    }

    /// A persistence failure while writing the row.
    #[must_use]
    pub fn database(row_number: usize, message: impl Into<String>) -> Self {
        Self {
            row_number,
            field: String::new(),
            code: "database".to_string(),
            message: message.into(),
        }
    }
}

/// Escape one CSV field per RFC 4180 (quote when it contains a comma,
/// quote, CR, or LF; double any embedded quotes).
fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Render the error rows as a CSV document (header + one row each), the
/// downloadable error-report artifact.
#[must_use]
pub fn to_csv(rows: &[ErrorRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("row_number,field,code,message\n");
    for r in rows {
        // Writing to a String is infallible; the result is ignored.
        let _ = writeln!(
            out,
            "{},{},{},{}",
            r.row_number,
            csv_escape(&r.field),
            csv_escape(&r.code),
            csv_escape(&r.message),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{ErrorRow, to_csv};

    #[test]
    fn row_shape_is_stable() {
        let row = ErrorRow::validation(3, "name.family", "Family name is required");
        assert_eq!(row.row_number, 3);
        assert_eq!(row.field, "name.family");
        assert_eq!(row.code, "validation");
        assert_eq!(row.message, "Family name is required");
        // Serializes to the documented four keys.
        let v = serde_json::to_value(&row).unwrap();
        assert_eq!(v["row_number"], 3);
        assert_eq!(v["field"], "name.family");
        assert_eq!(v["code"], "validation");
        assert_eq!(v["message"], "Family name is required");
    }

    #[test]
    fn parse_and_database_rows_have_empty_field() {
        assert_eq!(ErrorRow::parse(1, "bad json").field, "");
        assert_eq!(ErrorRow::database(2, "insert failed").field, "");
    }

    #[test]
    fn csv_has_header_and_escapes_commas() {
        let rows = vec![
            ErrorRow::parse(1, "unexpected, token"),
            ErrorRow::validation(2, "tax_id", "invalid"),
        ];
        let csv = to_csv(&rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "row_number,field,code,message");
        assert_eq!(lines[1], "1,,parse,\"unexpected, token\"");
        assert_eq!(lines[2], "2,tax_id,validation,invalid");
    }
}
