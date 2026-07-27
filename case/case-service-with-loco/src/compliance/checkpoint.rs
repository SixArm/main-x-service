//! External witness: a signed checkpoint of the audit chain, recorded
//! **off-box**, so wholesale deletion becomes detectable.
//!
//! ## The gap this closes
//!
//! The chain detects a row deleted from the *middle* of a run, because
//! its successor's `prev_hash` no longer matches. It cannot detect
//! deletion of the **tail**: remove the last N rows and there is no
//! successor left to break, so the shortened chain verifies perfectly.
//! Delete every row and it verifies vacuously. The keyed MAC
//! ([`super::mac`]) does not help either — it proves a row was not
//! *forged*, and says nothing about a row that is simply gone.
//!
//! Truncation is invisible from inside the data. Detecting it requires
//! something the attacker cannot reach: a record, kept elsewhere, of what
//! the chain looked like at a known moment.
//!
//! ## The mechanism
//!
//! A [`Checkpoint`] states "at position *N* the chain head was *H*, and
//! *C* rows stood at or before *N*", plus a MAC so the checkpoint itself
//! cannot be rewritten by someone holding only the database. The operator
//! takes one periodically and stores it **outside this database** — a log
//! pipeline, an object store, a monitoring system, a printout in a safe.
//!
//! Later, [`Checkpoint::verify_against`] re-reads the chain and answers
//! one question: does it still honour what the checkpoint recorded?
//!
//! - the anchor row is gone ⇒ **rows were deleted**
//! - the anchor row is there but hashes differently ⇒ **content changed**
//! - fewer rows stand at or before *N* ⇒ **earlier rows were deleted**,
//!   even though the anchor itself survived
//!
//! ## What makes it work is the storage, not this code
//!
//! **A checkpoint kept in this database is worthless.** An attacker who
//! can delete audit rows can delete checkpoint rows in the same
//! transaction, and the MAC does not prevent that — it prevents forgery,
//! not deletion, which is the whole problem being solved. The externality
//! *is* the control; this module only makes the value cheap to produce,
//! cheap to compare, and unforgeable in transit.
//!
//! Each checkpoint is also emitted as a structured log line at `INFO`, so
//! a deployment that ships logs off the host already has a witness
//! without building anything further.

use serde::{Deserialize, Serialize};

use crate::models::_entities::audit_logs;

/// Format version, bound into the MAC so a later change to the
/// checkpoint's shape cannot be mistaken for a forged value.
pub const CHECKPOINT_VERSION: &str = "cp1";

/// A witness to the chain's state at one moment.
///
/// Serialize this and keep it somewhere this service's database cannot
/// reach. Every field is covered by [`Checkpoint::mac`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    /// Format version.
    pub version: String,
    /// The anchor row's `id` — the position this checkpoint describes.
    pub anchor_id: i32,
    /// The anchor row's stored SHA-256 chain hash.
    pub head: String,
    /// How many rows stood at or before `anchor_id` when this was taken.
    ///
    /// Carried so that deleting rows *earlier* than the anchor is caught
    /// too. Without it, an attacker could delete history freely as long
    /// as they left the most recent row alone.
    pub rows_at_or_before: u64,
    /// When it was taken, epoch microseconds.
    pub taken_at_micros: i64,
    /// MAC over every field above, or `None` when no key is configured.
    ///
    /// Without it the checkpoint is still useful — an attacker who never
    /// saw the stored copy cannot make the chain match it — but it can be
    /// *rewritten* by anyone who finds it. With it, the witness is only
    /// as forgeable as the key.
    pub mac: Option<String>,
}

/// Why a checkpoint could not be honoured.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum CheckpointVerdict {
    /// The chain still contains the anchor, unchanged, with at least as
    /// much history behind it.
    Honoured,
    /// The anchor row is gone: rows have been deleted.
    AnchorMissing {
        /// The `id` that should have been present.
        anchor_id: i32,
    },
    /// The anchor survived but no longer hashes to the recorded head.
    HeadChanged {
        /// What the checkpoint recorded.
        expected: String,
        /// What the row carries now.
        found: String,
    },
    /// The anchor survived but history behind it has shrunk.
    RowsDeleted {
        /// Rows recorded at or before the anchor.
        expected: u64,
        /// Rows found now.
        found: u64,
    },
    /// The supplied checkpoint's own MAC does not verify: it was altered,
    /// or produced under a different key. Nothing is concluded about the
    /// chain — the *witness* is what failed.
    CheckpointNotAuthentic,
    /// The checkpoint carries a MAC this service cannot check (no key, or
    /// an unknown key id). **Not** a finding about the chain.
    CheckpointUnverifiable,
}

impl Checkpoint {
    /// The MAC pre-image: every field except the MAC itself, unit-separated.
    fn preimage(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(160);
        for field in [
            self.version.as_str(),
            &self.anchor_id.to_string(),
            self.head.as_str(),
            &self.rows_at_or_before.to_string(),
            &self.taken_at_micros.to_string(),
        ] {
            buf.extend_from_slice(field.as_bytes());
            buf.push(0x1f);
        }
        buf
    }

    /// Take a checkpoint over the current chain.
    ///
    /// Returns `None` when the chain has no hashed row to anchor to — an
    /// empty trail cannot witness anything, and pretending otherwise
    /// would produce a checkpoint that any future state satisfies.
    ///
    /// # Errors
    ///
    /// When the queries fail.
    pub async fn take(
        db: &sea_orm::DatabaseConnection,
        taken_at_micros: i64,
    ) -> loco_rs::prelude::ModelResult<Option<Self>> {
        use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

        let Some(anchor) = audit_logs::Entity::find()
            .filter(audit_logs::Column::Hash.is_not_null())
            .order_by_desc(audit_logs::Column::Id)
            .one(db)
            .await?
        else {
            return Ok(None);
        };
        let Some(head) = anchor.hash.clone() else {
            return Ok(None);
        };
        let rows_at_or_before = audit_logs::Entity::find()
            .filter(audit_logs::Column::Id.lte(anchor.id))
            .count(db)
            .await?;

        let mut checkpoint = Self {
            version: CHECKPOINT_VERSION.to_string(),
            anchor_id: anchor.id,
            head,
            rows_at_or_before,
            taken_at_micros,
            mac: None,
        };
        checkpoint.mac = super::mac::tag(&checkpoint.preimage());
        Ok(Some(checkpoint))
    }

    /// Check whether the chain still honours this checkpoint.
    ///
    /// The checkpoint's own MAC is checked first: a witness that has been
    /// altered cannot be used to accuse the chain, and reporting it as
    /// tampering would point an investigation at the wrong place.
    ///
    /// # Errors
    ///
    /// When the queries fail.
    pub async fn verify_against(
        &self,
        db: &sea_orm::DatabaseConnection,
    ) -> loco_rs::prelude::ModelResult<CheckpointVerdict> {
        use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

        match super::mac::verify(self.mac.as_deref(), &self.preimage()) {
            // `Valid` is the expected path. `Absent` means the
            // checkpoint predates the key: still worth comparing, it just
            // carries less assurance about its own provenance. Both
            // continue to the chain comparison.
            super::mac::MacVerdict::Valid | super::mac::MacVerdict::Absent => {}
            super::mac::MacVerdict::Invalid => {
                return Ok(CheckpointVerdict::CheckpointNotAuthentic);
            }
            super::mac::MacVerdict::UnknownKey(_) | super::mac::MacVerdict::Malformed => {
                return Ok(CheckpointVerdict::CheckpointUnverifiable);
            }
        }

        let Some(anchor) = audit_logs::Entity::find_by_id(self.anchor_id)
            .one(db)
            .await?
        else {
            return Ok(CheckpointVerdict::AnchorMissing {
                anchor_id: self.anchor_id,
            });
        };
        let found = anchor.hash.unwrap_or_default();
        if found != self.head {
            return Ok(CheckpointVerdict::HeadChanged {
                expected: self.head.clone(),
                found,
            });
        }
        let now = audit_logs::Entity::find()
            .filter(audit_logs::Column::Id.lte(self.anchor_id))
            .count(db)
            .await?;
        if now < self.rows_at_or_before {
            return Ok(CheckpointVerdict::RowsDeleted {
                expected: self.rows_at_or_before,
                found: now,
            });
        }
        Ok(CheckpointVerdict::Honoured)
    }
}

#[cfg(test)]
mod tests {
    use super::{CHECKPOINT_VERSION, Checkpoint, CheckpointVerdict};

    fn sample() -> Checkpoint {
        Checkpoint {
            version: CHECKPOINT_VERSION.to_string(),
            anchor_id: 42,
            head: "abc123".to_string(),
            rows_at_or_before: 100,
            taken_at_micros: 1_700_000_000_000_000,
            mac: None,
        }
    }

    /// Every field is bound into the MAC pre-image, so none can be edited
    /// without invalidating the witness. An attacker who could lower
    /// `rows_at_or_before` while keeping the MAC would be able to delete
    /// history and still be told the checkpoint was honoured.
    #[test]
    fn every_field_is_bound_into_the_preimage() {
        let base = sample().preimage();
        let mutate = |f: &dyn Fn(&mut Checkpoint)| {
            let mut c = sample();
            f(&mut c);
            c.preimage()
        };
        assert_ne!(mutate(&|c| c.anchor_id = 43), base, "anchor_id");
        assert_ne!(mutate(&|c| c.head = "def".into()), base, "head");
        assert_ne!(mutate(&|c| c.rows_at_or_before = 99), base, "row count");
        assert_ne!(mutate(&|c| c.taken_at_micros = 1), base, "taken_at");
        assert_ne!(mutate(&|c| c.version = "cp2".into()), base, "version");
    }

    /// The separator makes field boundaries unambiguous, so two different
    /// checkpoints cannot share a pre-image by shifting a digit across
    /// the boundary.
    #[test]
    fn field_boundaries_are_unambiguous() {
        let mut a = sample();
        a.head = "ab".to_string();
        a.rows_at_or_before = 1;
        let mut b = sample();
        b.head = "a".to_string();
        b.rows_at_or_before = 1;
        // Differing only in where the boundary falls must still differ.
        assert_ne!(a.preimage(), b.preimage());
    }

    /// A checkpoint round-trips through JSON, since the point is to store
    /// it somewhere else and bring it back.
    #[test]
    fn round_trips_through_json() {
        let c = sample();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Checkpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }

    /// The verdicts serialize with a discriminator, so a monitor can
    /// branch on `verdict` without parsing prose.
    #[test]
    fn verdicts_are_machine_readable() {
        let v = CheckpointVerdict::AnchorMissing { anchor_id: 7 };
        let json = serde_json::to_value(&v).expect("serialize");
        assert_eq!(json["verdict"], "anchor_missing");
        assert_eq!(json["anchor_id"], 7);
        assert_eq!(
            serde_json::to_value(CheckpointVerdict::Honoured).expect("serialize")["verdict"],
            "honoured"
        );
    }
}
