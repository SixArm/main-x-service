//! The value-frequency scan: the fields the matcher weighs by their own
//! frequency (`agents/share/match-evidence.md` §7), for one page of active
//! persons.
//!
//! [`crate::matching::evidence::FrequencyCache`] pages through this to
//! count how often each given name, family name and birth date occurs. It
//! reads three columns per person instead of reassembling whole records,
//! so a scan of 200 000 persons is 200 queries, not millions.

use axa::sqlx::Db;
use chrono::NaiveDate;
use uuid::Uuid;

use super::to_i64;
use crate::Result;

/// The frequency-weighted fields of one active person: its primary name
/// and birth date.
#[derive(Debug, Clone)]
pub struct FrequencyRow {
    /// The person's id, the keyset cursor for the next page.
    pub id: Uuid,
    /// Primary name's family name, if the person has a primary name.
    pub family: Option<String>,
    /// Primary name's given names, in order.
    pub given: Vec<String>,
    /// Birth date, if recorded.
    pub birth_date: Option<NaiveDate>,
}

/// One page of active persons after `after` (exclusive), ordered by id.
///
/// Keyset paging on the primary key: stable while rows are inserted or
/// deleted between pages, which offset paging is not.
///
/// # Errors
///
/// Returns the crate database error if the query fails.
pub async fn page(
    conn: impl Into<Db<'_>>,
    after: Option<Uuid>,
    limit: u64,
) -> Result<Vec<FrequencyRow>> {
    let mut conn = conn.into().acquire().await?;
    let rows = sqlx::query!(
        r#"SELECT DISTINCT ON (p.id)
                  p.id AS "id!",
                  n.family AS "family?",
                  n.given AS "given?",
                  p.birth_date
           FROM persons p
           LEFT JOIN person_names n ON n.person_id = p.id AND n.is_primary
           WHERE p.deleted_at IS NULL AND p.active = true
             AND ($1::uuid IS NULL OR p.id > $1)
           ORDER BY p.id, n.id
           LIMIT $2"#,
        after,
        to_i64(limit),
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| FrequencyRow {
            id: r.id,
            family: r.family,
            given: r.given.unwrap_or_default(),
            birth_date: r.birth_date,
        })
        .collect())
}
