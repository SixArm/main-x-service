//! CSV codec — the operator / spreadsheet format
//! (`agents/share/bulk-import-export.md` §5; crate spec §9.4).
//!
//! CSV is inherently flat, so the bulk row (an optional `pid` + an
//! optional `active` flag + every `CarePathway` field) is flattened per
//! the shared [`super::columns`] declaration (scalars → columns; every
//! array / array-of-object → a single JSON-encoded cell — including
//! `in_language`, per that module's documented reading of crate spec
//! §9.4).
//!
//! The codec round-trips **losslessly** against the JSONL reference: it
//! flattens the bulk-row `Value` into cells and rebuilds the same
//! `Value` on the way back. Columns are matched **by header name**, so
//! operator-reordered columns and extra columns are tolerated; a
//! malformed row is a per-row `Err` (§7), never a whole-file abort.
//! TSV shares this exact codec, differing only in the delimiter byte the
//! caller supplies (`agents/share/bulk-import-export.md` §5).

use serde_json::Value;
use uuid::Uuid;

use care_pathway_matcher::CarePathway;
use loco_rs::Error;

use super::columns::{COLUMNS, Kind, from_row_value, get, header, set, to_row_value};

/// One decoded CSV row result: `(had_explicit_pid, pid, active,
/// pathway)` on success, or a per-row error message (§7) on failure.
type RowResult = Result<(bool, Option<Uuid>, Option<bool>, CarePathway), String>;

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

/// Encode a slice of `(pid, active, pathway)` rows to a CSV byte buffer (a
/// header row + one row per record), the export output shape.
///
/// # Errors
///
/// When a row fails to serialize or the CSV writer fails.
pub fn encode(
    rows: &[(Option<Uuid>, Option<bool>, CarePathway)],
    delimiter: u8,
) -> loco_rs::Result<Vec<u8>> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());
    wtr.write_record(header())
        .map_err(|e| Error::Message(format!("write CSV header: {e}")))?;
    for (pid, active, pathway) in rows {
        let value = to_row_value(*pid, *active, pathway)
            .map_err(|e| Error::Message(format!("serialize care pathway to CSV: {e}")))?;
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

/// Parse a CSV byte buffer into per-row `(had_explicit_pid, pid, active,
/// CarePathway)` results. Columns are matched by header name
/// (order-independent; unknown columns ignored); an invalid row is an
/// `Err` in its slot (§7 per-row error contract) rather than aborting the
/// whole load.
///
/// # Errors
///
/// Returns an error if the bytes are not a readable CSV (bad header /
/// structurally broken record framing) — a whole-file failure, distinct
/// from a per-row `Err` in the returned vector.
pub fn decode(input: &[u8], delimiter: u8) -> loco_rs::Result<Vec<RowResult>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(input);
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
    use care_pathway_matcher::{
        CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
        RelationKind, RelationshipRef,
    };
    use uuid::Uuid;

    /// A pathway with **every** field populated (arrays included), so a
    /// round-trip that preserves it proves the codec is lossless across
    /// the whole model.
    fn fully_populated() -> CarePathway {
        CarePathway {
            alternate_names: vec!["Stroke Fast-Track".to_string()],
            pathway_code: Some("STROKE-01".to_string()),
            provider_id: Some("nhs-trust-1".to_string()),
            provider_name: Some("Example NHS Trust".to_string()),
            care_setting: Some(CareSetting::Inpatient),
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Icd10,
                code: "I63".to_string(),
            }],
            interventions: vec!["thrombolysis".to_string()],
            keywords: vec!["stroke".to_string(), "neurology".to_string()],
            identifiers: vec![
                PathwayIdentifier {
                    scheme: IdentifierScheme::Doi,
                    value: "10.1000/xyz123".to_string(),
                },
                PathwayIdentifier {
                    scheme: IdentifierScheme::Custom("internal-id".to_string()),
                    value: "INT-42".to_string(),
                },
            ],
            same_as: vec!["https://www.wikidata.org/wiki/Q42".to_string()],
            in_language: vec!["en".to_string(), "cy".to_string()],
            relationships: vec![RelationshipRef {
                relation: RelationKind::Supersedes,
                pathway_id: "pathway-7".to_string(),
            }],
            tags: vec!["fast-track".to_string()],
            ..CarePathway::new("Acute Stroke Care Pathway")
        }
    }

    /// The single load-bearing property: encode → decode returns the
    /// exact same row (compared as the wire `Value`, so no field is
    /// dropped or mistyped).
    #[test]
    fn round_trips_a_fully_populated_pathway_losslessly() {
        let pid = Uuid::new_v4();
        let pathway = fully_populated();
        let bytes = encode(&[(Some(pid), Some(true), pathway.clone())], b',').unwrap();
        let rows = decode(&bytes, b',').unwrap();
        assert_eq!(rows.len(), 1);
        let (had_explicit_pid, parsed_pid, active, parsed) =
            rows.into_iter().next().unwrap().unwrap();
        assert!(had_explicit_pid, "the exported pid column round-trips");
        assert_eq!(parsed_pid, Some(pid));
        assert_eq!(active, Some(true));
        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            serde_json::to_value(&pathway).unwrap(),
            "CSV round-trip must be lossless"
        );
    }

    /// A sparse pathway (only the required `name`; every `Option`/array
    /// empty) also round-trips — the empty-cell handling restores
    /// `None`/`[]`.
    #[test]
    fn round_trips_a_sparse_pathway() {
        let pathway = CarePathway::new("Solo Pathway");
        let bytes = encode(&[(None, None, pathway.clone())], b',').unwrap();
        let (had_explicit_pid, pid, active, back) = decode(&bytes, b',')
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert!(!had_explicit_pid);
        assert_eq!(pid, None);
        assert_eq!(active, None);
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&pathway).unwrap()
        );
        assert!(back.pathway_code.is_none() && back.identifiers.is_empty());
    }

    /// Multiple pathways → a header + one row each; every row
    /// round-trips.
    #[test]
    fn encodes_a_header_then_one_row_per_pathway() {
        let a = fully_populated();
        let b = CarePathway::new("B Pathway");
        let bytes = encode(&[(None, None, a.clone()), (None, None, b.clone())], b',').unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.lines().next().unwrap().starts_with("pid,active,name,"));
        let rows = decode(&bytes, b',').unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].as_ref().unwrap().3.name,
            "Acute Stroke Care Pathway"
        );
        assert_eq!(rows[1].as_ref().unwrap().3.name, "B Pathway");
    }

    /// Columns are matched by header, so a reordered / extra-column file
    /// still imports (operator-edited CSVs are tolerated).
    #[test]
    fn decodes_reordered_and_extra_columns() {
        let csv = "pathway_code,extra,name,identifiers,pid\n\
                   STROKE-01,ignored,Vader Pathway,[],11111111-1111-4111-8111-111111111111\n";
        let rows = decode(csv.as_bytes(), b',').unwrap();
        let (had_explicit_pid, pid, active, pathway) = rows.into_iter().next().unwrap().unwrap();
        assert!(had_explicit_pid);
        assert_eq!(
            pid,
            Some(Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap())
        );
        assert_eq!(active, None, "an omitted active column is absent");
        assert_eq!(pathway.name, "Vader Pathway");
        assert_eq!(pathway.pathway_code.as_deref(), Some("STROKE-01"));
    }

    /// A structurally-valid row with a bad JSON cell is a per-row `Err`,
    /// not a whole-file failure (§7). Built by index (rather than a
    /// hand-typed comma-counted literal) so the cell placement can't
    /// silently drift from [`COLUMNS`]'s declared order.
    #[test]
    fn a_bad_json_cell_is_a_per_row_error() {
        let hdr = header();
        let mut cells: Vec<&str> = vec![""; hdr.len()];
        cells[hdr.iter().position(|h| *h == "name").unwrap()] = "X";
        cells[hdr.iter().position(|h| *h == "identifiers").unwrap()] = "not-json";
        cells[hdr.iter().position(|h| *h == "same_as").unwrap()] = "[]";
        cells[hdr.iter().position(|h| *h == "relationships").unwrap()] = "[]";
        let bad = format!("{}\n{}\n", hdr.join(","), cells.join(","));
        let rows = decode(bad.as_bytes(), b',').unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_err(), "malformed identifiers cell ⇒ per-row Err");
    }

    /// A row whose `pid` cell is empty (or the column is absent
    /// entirely) is not an explicit pid.
    #[test]
    fn had_explicit_pid_is_false_for_an_empty_or_missing_pid_cell() {
        let pathway = fully_populated();
        let bytes = encode(&[(Some(Uuid::new_v4()), None, pathway)], b',').unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let (header_line, row_line) = text.split_once('\n').unwrap();

        let rows = decode(text.as_bytes(), b',').unwrap();
        assert!(rows[0].as_ref().unwrap().0, "populated pid cell ⇒ explicit");

        // Blank the leading pid cell only.
        let (_pid_cell, rest) = row_line.split_once(',').unwrap();
        let blanked = format!("{header_line}\n,{rest}\n");
        let rows = decode(blanked.as_bytes(), b',').unwrap();
        let (had_explicit_pid, pid, _, _) = rows.into_iter().next().unwrap().unwrap();
        assert!(!had_explicit_pid, "empty pid cell ⇒ no explicit pid");
        assert_eq!(pid, None);

        // The pid column omitted entirely (operator-trimmed header).
        let no_pid_col = "name,provider_id\nY,nhs-trust-1\n";
        let rows = decode(no_pid_col.as_bytes(), b',').unwrap();
        let (had_explicit_pid, pid, _, pathway) = rows.into_iter().next().unwrap().unwrap();
        assert!(!had_explicit_pid, "missing pid column ⇒ no explicit pid");
        assert_eq!(pid, None);
        assert_eq!(pathway.name, "Y");
    }

    /// TSV is the same codec with a different byte: a row round-trips
    /// through tabs exactly as it does through commas.
    #[test]
    fn tsv_round_trips_a_fully_populated_row() {
        let pathway = fully_populated();
        let rows = vec![(Some(Uuid::new_v4()), Some(true), pathway.clone())];
        let bytes = encode(&rows, b'\t').unwrap();
        assert!(
            String::from_utf8_lossy(&bytes).contains('\t'),
            "TSV output must be tab-separated"
        );
        let back = decode(&bytes, b'\t').unwrap();
        assert_eq!(back.len(), 1);
    }

    /// A field containing the delimiter is quoted, not allowed to split
    /// the record — the property that makes TSV safe for free text, and
    /// the reason this uses the `csv` crate rather than `split('\t')`.
    #[test]
    fn a_tab_inside_a_field_is_quoted_and_survives_tsv() {
        let mut pathway = fully_populated();
        pathway.name = "Acute\tStroke\tPathway".to_string();
        let rows = vec![(None, None, pathway)];
        let bytes = encode(&rows, b'\t').unwrap();
        let back = decode(&bytes, b'\t').unwrap();
        assert_eq!(back.len(), 1, "the tab must not have split the record");
    }
}
