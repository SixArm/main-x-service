//! CSV codec — the operator / spreadsheet format
//! (`agents/share/bulk-import-export.md` §5; crate spec §10.7).
//!
//! CSV is inherently flat, so the bulk row (an optional `pid` plus every
//! `Organization` field) is flattened per the shared [`super::columns`]
//! declaration (scalars → columns; the single nested `address` → dotted
//! columns; arrays / arrays-of-objects → a single JSON-encoded cell).
//!
//! The codec round-trips **losslessly** against the JSONL reference: it
//! flattens the bulk-row `Value` into cells and rebuilds the same
//! `Value` on the way back. Columns are matched **by header name**, so
//! operator-reordered columns and extra columns are tolerated; a
//! malformed row is a per-row `Err` (§7), never a whole-file abort.

use serde_json::Value;
use uuid::Uuid;

use loco_rs::Error;
use organization_matcher::Organization;

use super::columns::{COLUMNS, Kind, from_row_value, get, header, set, to_row_value};

/// One decoded CSV row result: `(had_explicit_pid, pid, organization)` on
/// success, or a per-row error message (§7) on failure.
type RowResult = Result<(bool, Option<Uuid>, Organization), String>;

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

/// Encode a slice of `(pid, organization)` rows to a CSV byte buffer (a
/// header row + one row per record), the export output shape.
///
/// # Errors
///
/// When a row fails to serialize or the CSV writer fails.
pub fn encode(rows: &[(Option<Uuid>, Organization)]) -> loco_rs::Result<Vec<u8>> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(header())
        .map_err(|e| Error::Message(format!("write CSV header: {e}")))?;
    for (pid, org) in rows {
        let value = to_row_value(*pid, org)
            .map_err(|e| Error::Message(format!("serialize organization to CSV: {e}")))?;
        let row: Vec<String> = COLUMNS
            .iter()
            .map(|c| render(get(&value, c.path), c.kind))
            .collect();
        wtr.write_record(&row)
            .map_err(|e| Error::Message(format!("write CSV row: {e}")))?;
    }
    wtr.into_inner()
        .map_err(|e| Error::Message(format!("finish CSV: {e}")))
}

/// Parse a CSV byte buffer into per-row `(had_explicit_pid, pid,
/// Organization)` results. Columns are matched by header name
/// (order-independent; unknown columns ignored); an invalid row is an
/// `Err` in its slot (§7 per-row error contract) rather than aborting the
/// whole load.
///
/// # Errors
///
/// Returns an error if the bytes are not a readable CSV (bad header /
/// structurally broken record framing) — a whole-file failure, distinct
/// from a per-row `Err` in the returned vector.
pub fn decode(input: &[u8]) -> loco_rs::Result<Vec<RowResult>> {
    let mut rdr = csv::Reader::from_reader(input);
    let headers = rdr
        .headers()
        .map_err(|e| Error::Message(format!("read CSV header: {e}")))?
        .clone();
    // Resolve each expected column to its index in the actual header row.
    let indices: Vec<Option<usize>> = COLUMNS
        .iter()
        .map(|c| headers.iter().position(|h| h == c.header))
        .collect();

    let mut out = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| Error::Message(format!("read CSV row: {e}")))?;
        out.push(record_to_row(&record, &indices));
    }
    Ok(out)
}

/// Rebuild one bulk row from a CSV record + the resolved column indices,
/// by reconstructing the wire `Value` and delegating to
/// [`from_row_value`].
fn record_to_row(record: &csv::StringRecord, indices: &[Option<usize>]) -> RowResult {
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
                // A present cell is parsed as JSON; an empty/missing cell
                // is omitted (not set to `null`) so the field's serde
                // default applies — `null` would fail to deserialize
                // into a `Vec`.
                if !cell.is_empty() {
                    set(
                        &mut map,
                        column.path,
                        serde_json::from_str(cell).map_err(|e| e.to_string())?,
                    );
                }
            }
        }
    }
    from_row_value(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, header};
    use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization, PostalAddress};
    use uuid::Uuid;

    /// An organization with **every** field populated (nested object +
    /// arrays), so a round-trip that preserves it proves the codec is
    /// lossless across the whole model.
    fn fully_populated() -> Organization {
        Organization {
            legal_name: Some("Acme Incorporated".to_string()),
            alternate_names: vec!["Acme".to_string(), "Roadrunner Supplies".to_string()],
            identifiers: vec![
                OrgIdentifier {
                    scheme: IdentifierScheme::Lei,
                    value: "5493001KJTIIGC8Y1R12".to_string(),
                },
                OrgIdentifier {
                    scheme: IdentifierScheme::Custom("internal-id".to_string()),
                    value: "INT-42".to_string(),
                },
            ],
            url: Some("https://acme.example".to_string()),
            same_as: vec!["https://www.wikidata.org/wiki/Q42".to_string()],
            address: Some(PostalAddress {
                street_address: Some("1 Analytical Way".to_string()),
                locality: Some("London".to_string()),
                region: Some("Greater London".to_string()),
                postal_code: Some("SW1".to_string()),
                country: Some("GB".to_string()),
            }),
            jurisdiction: Some("GB".to_string()),
            founding_date: Some("1985-04-01".to_string()),
            telephone: Some("+44 20 7946 0958".to_string()),
            email: Some("accounts@acme.example".to_string()),
            keywords: vec!["anvils".to_string(), "rockets".to_string()],
            ..Organization::new("Acme, Inc.")
        }
    }

    /// The single load-bearing property: encode → decode returns the
    /// exact same row (compared as the wire `Value`, so no field is
    /// dropped or mistyped).
    #[test]
    fn round_trips_a_fully_populated_organization_losslessly() {
        let pid = Uuid::new_v4();
        let org = fully_populated();
        let bytes = encode(&[(Some(pid), org.clone())]).unwrap();
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        let (had_explicit_pid, parsed_pid, parsed) = rows.into_iter().next().unwrap().unwrap();
        assert!(had_explicit_pid, "the exported pid column round-trips");
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            serde_json::to_value(&org).unwrap(),
            "CSV round-trip must be lossless"
        );
    }

    /// A sparse organization (only the required `name`; every
    /// `Option`/array empty) also round-trips — the empty-cell handling
    /// restores `None`/`[]`.
    #[test]
    fn round_trips_a_sparse_organization() {
        let org = Organization::new("Solo Ltd");
        let bytes = encode(&[(None, org.clone())]).unwrap();
        let (had_explicit_pid, pid, back) =
            decode(&bytes).unwrap().into_iter().next().unwrap().unwrap();
        assert!(!had_explicit_pid);
        assert_eq!(pid, None);
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&org).unwrap()
        );
        assert!(back.legal_name.is_none() && back.identifiers.is_empty());
    }

    /// Multiple organizations → a header + one row each; every row
    /// round-trips.
    #[test]
    fn encodes_a_header_then_one_row_per_organization() {
        let a = fully_populated();
        let b = Organization::new("B, Inc.");
        let bytes = encode(&[(None, a.clone()), (None, b.clone())]).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            text.lines()
                .next()
                .unwrap()
                .starts_with("pid,name,legal_name,")
        );
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].as_ref().unwrap().2.name, "Acme, Inc.");
        assert_eq!(rows[1].as_ref().unwrap().2.name, "B, Inc.");
    }

    /// Columns are matched by header, so a reordered / extra-column file
    /// still imports (operator-edited CSVs are tolerated).
    #[test]
    fn decodes_reordered_and_extra_columns() {
        let csv = "jurisdiction,extra,name,identifiers,pid\n\
                   US,ignored,Vader Corp,[],11111111-1111-4111-8111-111111111111\n";
        let rows = decode(csv.as_bytes()).unwrap();
        let (had_explicit_pid, pid, org) = rows.into_iter().next().unwrap().unwrap();
        assert!(had_explicit_pid);
        assert_eq!(
            pid,
            Some(Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap())
        );
        assert_eq!(org.name, "Vader Corp");
        assert_eq!(org.jurisdiction.as_deref(), Some("US"));
    }

    /// A structurally-valid row with a bad JSON cell is a per-row `Err`,
    /// not a whole-file failure (§7).
    #[test]
    fn a_bad_json_cell_is_a_per_row_error() {
        let hdr = header().join(",");
        // 17 columns; the `identifiers` column (index 13, 0-based) gets a
        // malformed JSON cell.
        let bad = format!("{hdr}\n,X,,,,,,,,,,,,not-json,[],[],[]\n");
        let rows = decode(bad.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_err(), "malformed identifiers cell ⇒ per-row Err");
    }

    /// A row whose `pid` cell is empty (or the column is absent
    /// entirely) is not an explicit pid.
    #[test]
    fn had_explicit_pid_is_false_for_an_empty_or_missing_pid_cell() {
        let org = fully_populated();
        let bytes = encode(&[(Some(Uuid::new_v4()), org)]).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let (header_line, row_line) = text.split_once('\n').unwrap();

        let rows = decode(text.as_bytes()).unwrap();
        assert!(rows[0].as_ref().unwrap().0, "populated pid cell ⇒ explicit");

        // Blank the leading pid cell only.
        let (_pid_cell, rest) = row_line.split_once(',').unwrap();
        let blanked = format!("{header_line}\n,{rest}\n");
        let rows = decode(blanked.as_bytes()).unwrap();
        let (had_explicit_pid, pid, _) = rows.into_iter().next().unwrap().unwrap();
        assert!(!had_explicit_pid, "empty pid cell ⇒ no explicit pid");
        assert_eq!(pid, None);

        // The pid column omitted entirely (operator-trimmed header).
        let no_pid_col = "name,jurisdiction\nY,US\n";
        let rows = decode(no_pid_col.as_bytes()).unwrap();
        let (had_explicit_pid, pid, org) = rows.into_iter().next().unwrap().unwrap();
        assert!(!had_explicit_pid, "missing pid column ⇒ no explicit pid");
        assert_eq!(pid, None);
        assert_eq!(org.name, "Y");
    }
}
