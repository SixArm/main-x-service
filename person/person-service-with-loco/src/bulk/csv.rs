//! CSV codec — the operator / spreadsheet format
//! (`agents/share/bulk-import-export.md` §5).
//!
//! CSV is inherently flat, so the person wire type is flattened with the
//! family-wide §5 convention:
//!
//! - **scalar** fields → one column each (`id`, `active`, `gender`,
//!   `birth_date`, `tax_id`, `deceased`, `deceased_datetime`,
//!   `marital_status`, `multiple_birth`, `managing_organization`,
//!   `created_at`, `updated_at`);
//! - the primary **name** (a single nested object) → **dotted** columns
//!   (`name.family` scalar; its `use_type` / `given` / `prefix` / `suffix`
//!   parts follow the rule below);
//! - every **array / array-of-objects** → a single **JSON-encoded cell**
//!   (`name.given`, `name.prefix`, `name.suffix`, `identifiers`,
//!   `additional_names`, `telecom`, `documents`, `emergency_contacts`,
//!   `addresses`, `photo`, `links`).
//!
//! The exact column set is declared in the crate spec (§10). The codec
//! round-trips **losslessly** against the JSONL reference: it flattens the
//! person's Serde `Value` into cells and rebuilds the same `Value` on the
//! way back, so `decode(encode(p)) == p`. Columns are matched **by header
//! name**, so operator-reordered columns and extra columns are tolerated;
//! a malformed row is a per-row `Err` (§7), never a whole-file abort.

use serde_json::Value;

use crate::models::Person;
use crate::{Error, Result};

/// How a column's cell is rendered on export and parsed on import.
#[derive(Clone, Copy)]
enum Kind {
    /// A JSON scalar rendered **bare** (a string without quotes; empty cell
    /// ⇒ the field is omitted, so `Option`/`default` fields fall back).
    Scalar,
    /// A boolean rendered `true` / `false`; empty cell ⇒ omitted (so
    /// `active` defaults `true`, `deceased` `false`, `multiple_birth` `None`).
    Bool,
    /// A nested value rendered as a compact **JSON** cell (`[]` / `null` /
    /// `[...]`); always emitted (never omitted), so a required array such as
    /// `name.given` always round-trips.
    Json,
}

/// One CSV column: its header, the path into the person wire `Value`, and
/// its cell [`Kind`].
struct Column {
    header: &'static str,
    path: &'static [&'static str],
    kind: Kind,
}

/// The person CSV column set (spec §10). Order is the export column order;
/// import matches by header name, so the order here is not load-bearing for
/// reading.
const COLUMNS: &[Column] = &[
    col("id", &["id"], Kind::Scalar),
    col("active", &["active"], Kind::Bool),
    col("name.family", &["name", "family"], Kind::Scalar),
    col("name.use_type", &["name", "use_type"], Kind::Scalar),
    col("name.given", &["name", "given"], Kind::Json),
    col("name.prefix", &["name", "prefix"], Kind::Json),
    col("name.suffix", &["name", "suffix"], Kind::Json),
    col("gender", &["gender"], Kind::Scalar),
    col("birth_date", &["birth_date"], Kind::Scalar),
    col("tax_id", &["tax_id"], Kind::Scalar),
    col("deceased", &["deceased"], Kind::Bool),
    col("deceased_datetime", &["deceased_datetime"], Kind::Scalar),
    col("marital_status", &["marital_status"], Kind::Scalar),
    col("multiple_birth", &["multiple_birth"], Kind::Bool),
    col(
        "managing_organization",
        &["managing_organization"],
        Kind::Scalar,
    ),
    col("created_at", &["created_at"], Kind::Scalar),
    col("updated_at", &["updated_at"], Kind::Scalar),
    col("identifiers", &["identifiers"], Kind::Json),
    col("additional_names", &["additional_names"], Kind::Json),
    col("telecom", &["telecom"], Kind::Json),
    col("documents", &["documents"], Kind::Json),
    col("emergency_contacts", &["emergency_contacts"], Kind::Json),
    col("addresses", &["addresses"], Kind::Json),
    col("photo", &["photo"], Kind::Json),
    col("links", &["links"], Kind::Json),
];

/// `const`-fn column constructor (keeps [`COLUMNS`] readable).
const fn col(header: &'static str, path: &'static [&'static str], kind: Kind) -> Column {
    Column { header, path, kind }
}

/// The export header row, in [`COLUMNS`] order.
#[must_use]
pub fn header() -> Vec<&'static str> {
    COLUMNS.iter().map(|c| c.header).collect()
}

/// Navigate `value` by `path`, returning [`Value::Null`] for any missing key.
fn get<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key).unwrap_or(&Value::Null);
    }
    cur
}

/// Set `val` at `path` inside `map`, creating intermediate objects.
fn set(map: &mut serde_json::Map<String, Value>, path: &[&str], val: Value) {
    match path {
        [key] => {
            map.insert((*key).to_string(), val);
        }
        [key, rest @ ..] => {
            let entry = map
                .entry((*key).to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Value::Object(obj) = entry {
                set(obj, rest, val);
            }
        }
        [] => {}
    }
}

/// Render one field `Value` to its cell text for the given [`Kind`].
fn render(value: &Value, kind: Kind) -> String {
    match kind {
        Kind::Scalar | Kind::Bool => match value {
            Value::Null => String::new(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
        Kind::Json => value.to_string(),
    }
}

/// Encode a slice of persons to a CSV byte buffer (a header row + one row
/// per person), the export output shape.
///
/// # Errors
///
/// Returns [`Error::Api`] if a person fails to serialize or the CSV writer
/// fails.
pub fn encode(persons: &[Person]) -> Result<Vec<u8>> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(header())
        .map_err(|e| Error::Api(format!("write CSV header: {e}")))?;
    for person in persons {
        let value = serde_json::to_value(person)
            .map_err(|e| Error::Api(format!("serialize person to CSV: {e}")))?;
        let row: Vec<String> = COLUMNS
            .iter()
            .map(|c| render(get(&value, c.path), c.kind))
            .collect();
        wtr.write_record(&row)
            .map_err(|e| Error::Api(format!("write CSV row: {e}")))?;
    }
    wtr.into_inner()
        .map_err(|e| Error::Api(format!("finish CSV: {e}")))
}

/// Parse a CSV byte buffer into per-row [`Person`] results. Columns are
/// matched by header name (order-independent; unknown columns ignored); an
/// invalid row is an `Err` in its slot (§7 per-row error contract) rather
/// than aborting the whole load.
///
/// # Errors
///
/// Returns [`Error::Validation`] if the bytes are not a readable CSV (bad
/// header / structurally broken record framing).
pub fn decode(input: &[u8]) -> Result<Vec<serde_json::Result<Person>>> {
    let mut rdr = csv::Reader::from_reader(input);
    let headers = rdr
        .headers()
        .map_err(|e| Error::Validation(format!("read CSV header: {e}")))?
        .clone();
    // Resolve each expected column to its index in the actual header row.
    let indices: Vec<Option<usize>> = COLUMNS
        .iter()
        .map(|c| headers.iter().position(|h| h == c.header))
        .collect();

    let mut out = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| Error::Validation(format!("read CSV row: {e}")))?;
        out.push(record_to_person(&record, &indices));
    }
    Ok(out)
}

/// Rebuild one [`Person`] from a CSV record + the resolved column indices,
/// by reconstructing the person wire `Value` and deserializing it.
fn record_to_person(
    record: &csv::StringRecord,
    indices: &[Option<usize>],
) -> serde_json::Result<Person> {
    let mut map = serde_json::Map::new();
    for (column, index) in COLUMNS.iter().zip(indices) {
        let cell = index.and_then(|i| record.get(i)).unwrap_or("");
        match column.kind {
            Kind::Scalar => {
                if !cell.is_empty() {
                    set(&mut map, column.path, Value::String(cell.to_string()));
                }
            }
            Kind::Bool => {
                if !cell.is_empty() {
                    set(&mut map, column.path, Value::Bool(cell == "true"));
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
    use crate::models::{
        Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse, DocumentType,
        Gender, HumanName, Identifier, IdentifierType, IdentityDocument, LinkType, NameUse, Person,
        PersonLink,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use uuid::Uuid;

    /// A person with **every** field populated (nested objects + arrays), so
    /// a round-trip that preserves it proves the codec is lossless across the
    /// whole model.
    fn fully_populated() -> Person {
        let mut p = Person::new(
            HumanName {
                use_type: Some(NameUse::Official),
                family: "Lovelace".to_string(),
                given: vec!["Ada".to_string(), "Augusta".to_string()],
                prefix: vec!["Ms".to_string()],
                suffix: vec!["Jr".to_string()],
            },
            Gender::Female,
        );
        p.id = Uuid::new_v4();
        p.active = false;
        p.identifiers = vec![Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        )];
        p.additional_names = vec![HumanName {
            use_type: Some(NameUse::Maiden),
            family: "Byron".to_string(),
            given: vec!["Ada".to_string()],
            prefix: vec![],
            suffix: vec![],
        }];
        p.telecom = vec![ContactPoint {
            system: ContactPointSystem::Email,
            value: "ada@example.com".to_string(),
            use_type: Some(ContactPointUse::Home),
        }];
        p.birth_date = Some(NaiveDate::from_ymd_opt(1815, 12, 10).unwrap());
        p.tax_id = Some("TAX-1".to_string());
        p.documents = vec![IdentityDocument {
            document_type: DocumentType::Passport,
            number: "X1".to_string(),
            issuing_country: Some("GB".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: true,
        }];
        p.deceased = true;
        p.deceased_datetime = Some(Utc.with_ymd_and_hms(1852, 11, 27, 0, 0, 0).unwrap());
        p.addresses = vec![Address {
            use_type: Some(AddressUse::Home),
            line1: Some("1 Analytical Way".to_string()),
            line2: None,
            city: Some("London".to_string()),
            state: None,
            postal_code: Some("SW1".to_string()),
            country: Some("GB".to_string()),
        }];
        p.marital_status = Some("married".to_string());
        p.multiple_birth = Some(false);
        p.photo = vec!["https://example.com/ada.png".to_string()];
        p.managing_organization = Some(Uuid::new_v4());
        p.links = vec![PersonLink {
            other_person_id: Uuid::new_v4(),
            link_type: LinkType::Seealso,
        }];
        p
    }

    /// The single load-bearing property: encode → decode returns the exact
    /// same person (compared as the wire `Value`, so no field is dropped or
    /// mistyped).
    #[test]
    fn round_trips_a_fully_populated_person_losslessly() {
        let p = fully_populated();
        let bytes = encode(std::slice::from_ref(&p)).unwrap();
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        let back = rows.into_iter().next().unwrap().expect("row parses");
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&p).unwrap(),
            "CSV round-trip must be lossless"
        );
    }

    /// A sparse person (only the required fields; every `Option`/array empty)
    /// also round-trips — the empty-cell handling restores `None`/`[]`.
    #[test]
    fn round_trips_a_sparse_person() {
        let p = Person::new(
            HumanName {
                use_type: None,
                family: "Solo".to_string(),
                given: vec!["Han".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let bytes = encode(std::slice::from_ref(&p)).unwrap();
        let back = decode(&bytes).unwrap().into_iter().next().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            serde_json::to_value(&p).unwrap()
        );
        assert!(back.tax_id.is_none() && back.identifiers.is_empty());
    }

    /// Multiple persons → a header + one row each; every row round-trips.
    #[test]
    fn encodes_a_header_then_one_row_per_person() {
        let a = fully_populated();
        let b = Person::new(
            HumanName {
                use_type: None,
                family: "B".to_string(),
                given: vec!["Bee".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Other,
        );
        let bytes = encode(&[a.clone(), b.clone()]).unwrap();
        // Header line first.
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            text.lines()
                .next()
                .unwrap()
                .starts_with("id,active,name.family,")
        );
        let rows = decode(&bytes).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].as_ref().unwrap().name.family, "Lovelace");
        assert_eq!(rows[1].as_ref().unwrap().name.family, "B");
    }

    /// Columns are matched by header, so a reordered / extra-column file
    /// still imports (operator-edited CSVs are tolerated).
    #[test]
    fn decodes_reordered_and_extra_columns() {
        let csv = "gender,extra,name.family,name.given,id,active\n\
                   male,ignored,Vader,[\"Anakin\"],11111111-1111-4111-8111-111111111111,true\n";
        let rows = decode(csv.as_bytes()).unwrap();
        let p = rows.into_iter().next().unwrap().expect("parses");
        assert_eq!(p.name.family, "Vader");
        assert_eq!(p.name.given, vec!["Anakin".to_string()]);
        assert_eq!(p.gender, Gender::Male);
        assert!(p.active);
    }

    /// A structurally-valid row with a bad JSON cell is a per-row `Err`, not
    /// a whole-file failure (§7).
    #[test]
    fn a_bad_json_cell_is_a_per_row_error() {
        let hdr = header().join(",");
        // A row whose `identifiers` cell is malformed JSON.
        let bad = format!(
            "{hdr}\n11111111-1111-4111-8111-111111111111,true,X,,[],[],[],male,,,false,,,,,,,not-json,[],[],[],[],[],[],[]\n"
        );
        let rows = decode(bad.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_err(), "malformed identifiers cell ⇒ per-row Err");
    }

    #[test]
    fn header_lists_the_declared_columns() {
        let h = header();
        assert_eq!(h.first(), Some(&"id"));
        assert!(h.contains(&"name.family"));
        assert!(h.contains(&"identifiers"));
        assert!(h.contains(&"links"));
    }
}
