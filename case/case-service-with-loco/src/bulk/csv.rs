//! CSV codec — the operator / spreadsheet format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! CSV is inherently flat, so [`row::BulkCaseRow`](super::row::BulkCaseRow)
//! is flattened per the shared [`super::columns`] declaration (scalars →
//! columns; arrays and the enum fields → a single JSON-encoded cell).
//!
//! The codec round-trips **losslessly** against the JSONL reference: it
//! flattens the row's Serde `Value` into cells and rebuilds the same
//! `Value` on the way back, so `decode(encode(r)) == r`. Columns are
//! matched **by header name**, so operator-reordered columns and extra
//! columns are tolerated; a malformed row is a per-row `Err` (§7), never
//! a whole-file abort.
//!
//! Unlike the person reference implementation's CSV codec, `decode` does
//! not need a separate `had_explicit_id` out-of-band flag: `BulkCaseRow`
//! is defined *for this bulk format* with `pid: Option<Uuid>` carrying no
//! fabricated default, so a parsed row's own `pid.is_none()` already
//! answers "did this row name an existing record" — see
//! [`row`](super::row)'s module docs.

use serde_json::Value;

use super::columns::{COLUMNS, Kind, get, header, set};
use super::row::BulkCaseRow;

/// Render one field `Value` to its cell text for the given [`Kind`].
fn render(value: &Value, kind: Kind) -> String {
    match kind {
        Kind::Scalar => match value {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
        Kind::Json => value.to_string(),
    }
}

/// Encode a slice of rows to a CSV byte buffer (a header row + one row
/// per record), the export output shape.
///
/// # Errors
///
/// Returns an error if a row fails to serialize or the CSV writer fails.
pub fn encode(rows: &[BulkCaseRow]) -> loco_rs::Result<Vec<u8>> {
    let mut wtr = ::csv::Writer::from_writer(Vec::new());
    wtr.write_record(header())
        .map_err(|e| loco_rs::Error::Message(format!("write CSV header: {e}")))?;
    for row in rows {
        let value = serde_json::to_value(row)
            .map_err(|e| loco_rs::Error::Message(format!("serialize row to CSV: {e}")))?;
        let record: Vec<String> = COLUMNS
            .iter()
            .map(|c| render(get(&value, c.path), c.kind))
            .collect();
        wtr.write_record(&record)
            .map_err(|e| loco_rs::Error::Message(format!("write CSV row: {e}")))?;
    }
    wtr.into_inner()
        .map_err(|e| loco_rs::Error::Message(format!("finish CSV: {e}")))
}

/// Parse a CSV byte buffer into per-row parse results. Columns are
/// matched by header name (order-independent; unknown columns ignored);
/// an invalid row is an `Err` in its slot (§7 per-row error contract)
/// rather than aborting the whole load.
///
/// # Errors
///
/// Returns an error if the bytes are not a readable CSV (bad header /
/// structurally broken record framing).
pub fn decode(input: &[u8]) -> loco_rs::Result<Vec<serde_json::Result<BulkCaseRow>>> {
    let mut rdr = ::csv::Reader::from_reader(input);
    let headers = rdr
        .headers()
        .map_err(|e| loco_rs::Error::Message(format!("read CSV header: {e}")))?
        .clone();
    // Resolve each expected column to its index in the actual header row.
    let indices: Vec<Option<usize>> = COLUMNS
        .iter()
        .map(|c| headers.iter().position(|h| h == c.header))
        .collect();

    let mut out = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| loco_rs::Error::Message(format!("read CSV row: {e}")))?;
        out.push(record_to_row(&record, &indices));
    }
    Ok(out)
}

/// Rebuild one [`BulkCaseRow`] from a CSV record + the resolved column
/// indices, by reconstructing the row's wire `Value` and deserializing it.
fn record_to_row(
    record: &::csv::StringRecord,
    indices: &[Option<usize>],
) -> serde_json::Result<BulkCaseRow> {
    let mut map = serde_json::Map::new();
    for (column, index) in COLUMNS.iter().zip(indices) {
        let cell = index.and_then(|i| record.get(i)).unwrap_or("");
        match column.kind {
            Kind::Scalar => {
                if !cell.is_empty() {
                    set(&mut map, column.path, Value::String(cell.to_string()));
                }
            }
            Kind::Json => {
                // A present cell is parsed as JSON; an empty/missing cell is
                // omitted (not set to `null`) so the field's serde default
                // applies — `null` would fail to deserialize into a `Vec`.
                if !cell.is_empty() {
                    set(&mut map, column.path, serde_json::from_str(cell)?);
                }
            }
        }
    }
    serde_json::from_value(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, header};
    use crate::bulk::row::BulkCaseRow;
    use case_matcher::{Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, Priority};
    use uuid::Uuid;

    /// A row with **every** field populated (nested arrays + the
    /// `Custom`-carrying enum variants), so a round-trip that preserves
    /// it proves the codec is lossless across the whole model.
    fn fully_populated() -> BulkCaseRow {
        let case = Case {
            title: "Housing benefit appeal".to_string(),
            alternate_titles: vec!["HBA-1".to_string()],
            case_number: Some("CN-2024-001".to_string()),
            agency_id: Some("dhs".to_string()),
            agency_name: Some("Dept. of Housing Services".to_string()),
            case_type: Some(CaseType::Custom("Grant Dispute".to_string())),
            status: Some(CaseStatus::InProgress),
            priority: Some(Priority::High),
            opened_date: Some("2024-01-31".to_string()),
            subjects: vec!["person:abc123".to_string()],
            keywords: vec!["housing".to_string(), "appeal".to_string()],
            identifiers: vec![CaseIdentifier {
                scheme: IdentifierScheme::Docket,
                value: "CV-2024-001234".to_string(),
            }],
            same_as: vec!["https://courts.example.gov/cv-2024-001234".to_string()],
            in_language: vec!["en".to_string()],
        };
        BulkCaseRow::with_pid(Uuid::new_v4(), case)
    }

    /// The single load-bearing property: encode → decode returns the
    /// exact same row (compared as the wire `Value`, so no field is
    /// dropped or mistyped), including the `Custom` enum variant.
    #[test]
    fn round_trips_a_fully_populated_row_losslessly() {
        let row = fully_populated();
        let bytes = encode(std::slice::from_ref(&row)).unwrap();
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        let back = rows.into_iter().next().unwrap().expect("row parses");
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&row).unwrap(),
            "CSV round-trip must be lossless"
        );
        assert_eq!(back.pid, row.pid);
    }

    /// A sparse row (only the required `title`; every `Option`/array
    /// empty, no pid) also round-trips.
    #[test]
    fn round_trips_a_sparse_keyless_row() {
        let row = BulkCaseRow::keyless(Case::new("Bare"));
        let bytes = encode(std::slice::from_ref(&row)).unwrap();
        let back = decode(&bytes).unwrap().into_iter().next().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&row).unwrap()
        );
        assert!(back.pid.is_none());
        assert!(back.case.identifiers.is_empty());
    }

    /// Multiple rows → a header + one row each; every row round-trips.
    #[test]
    fn encodes_a_header_then_one_row_per_record() {
        let a = fully_populated();
        let b = BulkCaseRow::keyless(Case::new("B"));
        let bytes = encode(&[a.clone(), b.clone()]).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.lines().next().unwrap().starts_with("pid,title,"));
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].as_ref().unwrap().case.title,
            "Housing benefit appeal"
        );
        assert_eq!(rows[1].as_ref().unwrap().case.title, "B");
    }

    /// Columns are matched by header, so a reordered / extra-column file
    /// still imports (operator-edited CSVs are tolerated).
    #[test]
    fn decodes_reordered_and_extra_columns() {
        let csv = "case_number,extra,title,pid\n\
                   CN-1,ignored,Reordered,11111111-1111-4111-8111-111111111111\n";
        let rows = decode(csv.as_bytes()).unwrap();
        let row = rows.into_iter().next().unwrap().expect("parses");
        assert_eq!(row.case.title, "Reordered");
        assert_eq!(row.case.case_number.as_deref(), Some("CN-1"));
        assert_eq!(
            row.pid,
            Some(Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap())
        );
    }

    /// A structurally-valid row with a bad JSON cell is a per-row `Err`,
    /// not a whole-file failure (§7).
    #[test]
    fn a_bad_json_cell_is_a_per_row_error() {
        let hdr = header().join(",");
        // `subjects` (a Json-kind column) carries an unparseable cell.
        let bad = format!("{hdr}\n,Bad,[],,,,,,,,not-json,[],[],[],[]\n");
        let rows = decode(bad.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_err(), "malformed subjects cell ⇒ per-row Err");
    }

    /// A row whose `pid` cell is empty (or the column is absent entirely)
    /// parses to `pid: None` — the CSV-native version of the "no explicit
    /// id" check the person reference implementation needs a separate
    /// flag for (see the module docs).
    #[test]
    fn empty_or_missing_pid_column_parses_to_none() {
        let row = fully_populated();
        let bytes = encode(std::slice::from_ref(&row)).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let (header_line, row_line) = text.split_once('\n').unwrap();

        // A populated pid cell IS explicit (sanity check on the fixture).
        let rows = decode(text.as_bytes()).unwrap();
        assert!(rows[0].as_ref().unwrap().pid.is_some());

        // Blank the leading pid cell only.
        let (_pid_cell, rest) = row_line.split_once(',').unwrap();
        let blanked = format!("{header_line}\n,{rest}\n");
        let rows = decode(blanked.as_bytes()).unwrap();
        assert!(rows[0].as_ref().unwrap().pid.is_none(), "empty pid cell");

        // The pid column omitted entirely (operator-trimmed header).
        let no_pid_col = "title\nNo Pid Column\n";
        let rows = decode(no_pid_col.as_bytes()).unwrap();
        assert!(
            rows[0].as_ref().unwrap().pid.is_none(),
            "missing pid column"
        );
    }
}
