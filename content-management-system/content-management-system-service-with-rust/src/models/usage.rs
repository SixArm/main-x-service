//! "Where used" (CMS-R5, CMS-D8) — resolving the extracted reference
//! index back to the live content that carries each edge.
//!
//! Shared by the entry and asset controllers so there is **one**
//! definition of what counts as usage. The rule matters more than it
//! looks: usage means *a reference from the current revision of a live
//! variant*. A reference from a superseded revision is history, and
//! counting history as usage would make every asset permanently
//! undeletable — the CMS equivalent of a library that can never
//! withdraw a book because an old catalogue mentions it.

use loco_rs::prelude::*;
use sea_orm::QuerySelect;
use uuid::Uuid;

use super::_entities::{content_references, entries, entry_variants};

/// One live referrer of a target.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Referrer {
    /// The referring entry.
    pub entry_pid: Uuid,
    /// Its author-facing key.
    pub entry_key: String,
    /// The referring variant's locale.
    pub locale: String,
    /// That variant's editorial status.
    pub status: String,
    /// Whether that variant currently has a published revision — a
    /// broken reference on a published page is worse than one on a
    /// draft, and a caller usually wants to triage by that.
    pub published: bool,
    /// Where in the document the reference sits (`hero`,
    /// `blocks[3].asset`).
    pub field_key: String,
}

/// Maximum referrers resolved for one target. A cap is a `DoS` boundary
/// as much as a page size; the count is reported separately so a caller
/// is never shown a truncated list as if it were complete.
pub const MAX_REFERRERS: u64 = 500;

/// The live referrers of `target`, matched on `column`
/// (`ToEntryPid` or `ToAssetPid`).
///
/// # Errors
///
/// When a query fails.
pub async fn live_referrers(
    db: &DatabaseConnection,
    column: content_references::Column,
    target: Uuid,
) -> Result<Vec<Referrer>> {
    let rows = content_references::Entity::find()
        .filter(column.eq(target))
        .limit(MAX_REFERRERS)
        .all(db)
        .await?;
    let mut referrers = Vec::new();
    for row in rows {
        let Some(variant) = entry_variants::Entity::find()
            .filter(entry_variants::Column::Pid.eq(row.from_variant_pid))
            .filter(entry_variants::Column::DeletedAt.is_null())
            .one(db)
            .await?
        else {
            continue;
        };
        // The load-bearing line: only the *current* revision counts.
        if variant.current_revision_pid != Some(row.from_revision_pid) {
            continue;
        }
        let Some(entry) = entries::Entity::find()
            .filter(entries::Column::Pid.eq(variant.entry_pid))
            .filter(entries::Column::DeletedAt.is_null())
            .one(db)
            .await?
        else {
            continue;
        };
        referrers.push(Referrer {
            entry_pid: entry.pid,
            entry_key: entry.key,
            locale: variant.locale,
            status: variant.status,
            published: variant.published_revision_pid.is_some(),
            field_key: row.field_key,
        });
    }
    Ok(referrers)
}
