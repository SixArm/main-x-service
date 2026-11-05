//! The [`row::BulkCaseRow`](super::row::BulkCaseRow) flattening
//! declaration (`agents/share/bulk-import-export.md` §5) — [`super::csv`]
//! renders it; [`super::jsonl`] does not need it (JSONL carries the row's
//! full JSON `Value` verbatim, one object per line).
//!
//! - **scalars** → one column each: `pid`, `title`, `case_number`,
//!   `agency_id`, `agency_name`, `opened_date`.
//! - **`case_type` / `status` / `priority`** → **JSON-encoded**, not
//!   scalar, even though each is "one value per row". Each is an enum
//!   with a data-carrying `Custom(String)` variant, which `serde`
//!   externally tags as a JSON *object* (`{"Custom":"foo"}`) rather than
//!   a bare string — a plain scalar cell could not tell a unit variant's
//!   name from a `Custom` payload apart, or round-trip the object shape
//!   at all. JSON-encoding sidesteps that: `"Benefit"` and
//!   `{"Custom":"Foo"}` both round-trip losslessly as JSON text.
//! - every **array / array-of-objects** → a single **JSON-encoded**
//!   column (`alternate_titles`, `subjects`, `keywords`, `identifiers`,
//!   `same_as`, `in_language`).
//!
//! Unlike person's (which has a single nested `name` object needing
//! dotted columns) or organization's (`address`), `case_matcher::Case`
//! has **no single nested object** — every field is either a scalar, an
//! array, or (for the three enums above) treated as JSON for the reason
//! given. The exact column set is declared in the crate spec (§10.5).

use serde_json::Value;

/// How a column's value is classified.
#[derive(Clone, Copy)]
pub enum Kind {
    /// A JSON scalar (string/number); absent ⇒ the field's own
    /// `Option`/default applies.
    Scalar,
    /// A nested value (array, or a data-carrying enum) carried as compact
    /// JSON text; always present (never omitted), so a required array
    /// such as `subjects` always round-trips.
    Json,
}

/// One column: its header, the path into the row's JSON `Value`, and its
/// [`Kind`].
pub struct Column {
    /// The column/header name (CSV header cell).
    pub header: &'static str,
    /// The path into the row wire `Value` this column reads/writes.
    pub path: &'static [&'static str],
    /// How this column's value is classified.
    pub kind: Kind,
}

/// The case bulk-row column set (spec §10.5). Order is the export column
/// order; import matches by header name (CSV), so the order is not
/// load-bearing for reading.
pub const COLUMNS: &[Column] = &[
    col("pid", &["pid"], Kind::Scalar),
    col("title", &["title"], Kind::Scalar),
    col("alternate_titles", &["alternate_titles"], Kind::Json),
    col("case_number", &["case_number"], Kind::Scalar),
    col("agency_id", &["agency_id"], Kind::Scalar),
    col("agency_name", &["agency_name"], Kind::Scalar),
    col("case_type", &["case_type"], Kind::Json),
    col("status", &["status"], Kind::Json),
    col("priority", &["priority"], Kind::Json),
    col("opened_date", &["opened_date"], Kind::Scalar),
    col("subjects", &["subjects"], Kind::Json),
    col("keywords", &["keywords"], Kind::Json),
    col("identifiers", &["identifiers"], Kind::Json),
    col("same_as", &["same_as"], Kind::Json),
    col("in_language", &["in_language"], Kind::Json),
];

/// `const`-fn column constructor (keeps [`COLUMNS`] readable).
const fn col(header: &'static str, path: &'static [&'static str], kind: Kind) -> Column {
    Column { header, path, kind }
}

/// The export header row / column-name list, in [`COLUMNS`] order.
#[must_use]
pub fn header() -> Vec<&'static str> {
    COLUMNS.iter().map(|c| c.header).collect()
}

/// Navigate `value` by `path`, returning [`Value::Null`] for any missing key.
#[must_use]
pub fn get<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key).unwrap_or(&Value::Null);
    }
    cur
}

/// Set `val` at `path` inside `map`, creating intermediate objects.
pub fn set(map: &mut serde_json::Map<String, Value>, path: &[&str], val: Value) {
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

#[cfg(test)]
mod tests {
    use super::{COLUMNS, header};

    #[test]
    fn header_lists_the_declared_columns() {
        let h = header();
        assert_eq!(h.len(), COLUMNS.len());
        assert_eq!(h.first(), Some(&"pid"));
        assert!(h.contains(&"title"));
        assert!(h.contains(&"case_number"));
        assert!(h.contains(&"identifiers"));
        assert!(h.contains(&"case_type"));
    }
}
