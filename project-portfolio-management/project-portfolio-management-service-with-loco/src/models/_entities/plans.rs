//! `SeaORM` Entity — `plans`. The full `project_portfolio_management_matcher::Plan`
//! payload is stored in `data` (JSONB); `pid`, `kind`, `name`, and
//! `parent_pid` are denormalised for lookup, listing, and roll-up.
//!
//! All plans live in this one table as a single recursive collection.
//! `kind` is an optional descriptive label (metadata only — it no longer
//! scopes queries); `parent_pid` is the containment link.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted plan row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "plans")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp.
    pub updated_at: DateTimeWithTimeZone,
    /// Internal auto-increment primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Public, externally exposed identifier (UUID v4).
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// Optional descriptive kind label (`Portfolio` / `Project` /
    /// `Product` / `Program`), denormalised from the payload; `None` when
    /// unset. Metadata only — it does not scope queries.
    pub kind: Option<String>,
    /// Denormalised plan name, for lookup and listing.
    pub name: String,
    /// The full `project_portfolio_management_matcher::Plan` payload (JSONB).
    pub data: Json,
    /// Denormalised parent plan pid (any plan may contain any other), for
    /// cheap roll-up of a plan's children; `None` for a root plan.
    pub parent_pid: Option<Uuid>,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Operational phase-gate stage: the highest gate passed
    /// (`g0_concept` … `g5_benefits`), `None` before the first
    /// approved gate review. Governance state, not matcher payload.
    pub stage: Option<String>,
    /// The sequential project phase (`initiating` … `closing`),
    /// denormalised from the payload so a list or funnel read need not
    /// open every JSONB blob. `None` until an operator sets one — never
    /// back-filled, because inventing `initiating` for a plan already in
    /// delivery would be a fabricated history.
    ///
    /// Distinct from `stage` above: that is the last approved
    /// *governance* decision. The two are deliberately uncoupled
    /// (entity spec §1.5.1).
    pub phase: Option<String>,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTimeWithTimeZone>,
    /// SHA-256 (FIPS 180-4) over this row's integrity pre-image.
    ///
    /// `None` on a row written before the column existed — reported as
    /// unhashed, never as a mismatch, and never back-filled.
    pub content_hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub content_hash_sha3: Option<String>,
    /// HMAC-SHA256 over the same pre-image, as `"<scheme>.<key id>:<hex>"`.
    ///
    /// The only one of the three an adversary holding just this database
    /// cannot recompute: the digests are unkeyed and their pre-image
    /// format is published.
    pub content_mac: Option<String>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
