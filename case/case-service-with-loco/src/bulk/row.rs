//! The bulk wire envelope — [`BulkCaseRow`].
//!
//! `case_matcher::Case` (the service's API DTO, stored verbatim as JSONB)
//! carries **no `pid` field of its own** — the public id lives only in
//! the `cases` table row and the `CaseRef { pid, title }` the REST API
//! returns. A bulk row, unlike a single `POST`/`PUT` body, needs to be
//! able to *name* an existing record (so a re-imported export upserts in
//! place rather than creating a duplicate), so the bulk wire format is
//! `Case`'s fields plus an out-of-band, optional `pid`.
//!
//! This is a deliberate simplification versus the person reference
//! implementation: person's domain type carries its own `id: Uuid` field
//! with `#[serde(default = "Uuid::new_v4")]`, which makes "no id given"
//! and "an id was given and a fresh random one happens to match" the same
//! bit pattern after parsing — forcing a raw-JSON-line sniff
//! (`stable_key::row_has_explicit_id`) to recover the distinction from
//! the original bytes. `BulkCaseRow::pid` is a genuine `Option<Uuid>`
//! with **no default fabrication**: absence deserializes to `None` and
//! stays `None`, so [`stable_key::is_keyless`](super::stable_key::is_keyless)
//! can simply ask `pid.is_none()` — no byte-sniffing required.

use case_matcher::Case;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One bulk row: the stored `Case` plus the record's public id, when the
/// row names one (an export always does; a hand-authored import row may
/// not).
///
/// `#[serde(flatten)]` puts `Case`'s fields at the same JSON level as
/// `pid`, so a JSONL line reads as `{"pid": "...", "title": "...", …}` —
/// the same shape `GET /api/cases/{pid}` would produce if it inlined the
/// path parameter into the body.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkCaseRow {
    /// The case's public id, when the row names an existing record.
    /// `None` for a keyless row (see the module docs).
    #[serde(default)]
    pub pid: Option<Uuid>,
    /// The stored case fields.
    #[serde(flatten)]
    pub case: Case,
}

impl BulkCaseRow {
    /// Wrap an existing `(pid, Case)` pair — the shape an **export** row
    /// always takes.
    #[must_use]
    pub fn with_pid(pid: Uuid, case: Case) -> Self {
        Self {
            pid: Some(pid),
            case,
        }
    }

    /// Wrap a `Case` with no known pid — a **keyless** import row.
    #[must_use]
    pub fn keyless(case: Case) -> Self {
        Self { pid: None, case }
    }
}

#[cfg(test)]
mod tests {
    use super::BulkCaseRow;
    use case_matcher::Case;
    use uuid::Uuid;

    /// The `pid` field sits alongside `Case`'s own fields (`title`, …) at
    /// the same JSON level, not nested under a `case` key.
    #[test]
    fn flattens_pid_alongside_case_fields() {
        let row = BulkCaseRow::with_pid(Uuid::nil(), Case::new("Housing benefit appeal"));
        let v = serde_json::to_value(&row).unwrap();
        assert_eq!(v["pid"], "00000000-0000-0000-0000-000000000000");
        assert_eq!(v["title"], "Housing benefit appeal");
        assert!(v.get("case").is_none(), "must not nest under a `case` key");
    }

    /// A row JSON with no `pid` key at all deserializes to `pid: None` —
    /// no fabricated id, unlike person's `Person::id` default.
    #[test]
    fn missing_pid_key_deserializes_to_none() {
        let row: BulkCaseRow = serde_json::from_str(r#"{"title":"Bare case"}"#).unwrap();
        assert!(row.pid.is_none());
        assert_eq!(row.case.title, "Bare case");
    }

    /// A row with an explicit `pid` round-trips it exactly.
    #[test]
    fn explicit_pid_round_trips() {
        let pid = Uuid::new_v4();
        let row = BulkCaseRow::with_pid(pid, Case::new("Named"));
        let v = serde_json::to_value(&row).unwrap();
        let back: BulkCaseRow = serde_json::from_value(v).unwrap();
        assert_eq!(back.pid, Some(pid));
        assert_eq!(back.case.title, "Named");
    }

    /// `keyless` is exactly the "no explicit pid" constructor.
    #[test]
    fn keyless_carries_no_pid() {
        assert!(BulkCaseRow::keyless(Case::new("X")).pid.is_none());
    }
}
