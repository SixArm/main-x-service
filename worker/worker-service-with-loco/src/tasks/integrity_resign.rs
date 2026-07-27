//! `integrity_resign` task — re-MAC history under the current key after
//! a rotation, so a retired key can actually be retired.
//!
//! Rotation is additive by design ([`crate::compliance::mac`]): a stored
//! MAC names its key, and retired keys stay available for verification,
//! so rotating never invalidates history. The cost is that the old key
//! must be kept forever — every row still names it. This task closes
//! that loop: it re-computes each row's MAC under the **active** key, so
//! the retired one can be dropped from the configuration and destroyed.
//!
//! ```text
//! # what would change (writes nothing)
//! cargo loco task integrity_resign
//! cargo loco task integrity_resign target:audit
//!
//! # actually re-sign
//! cargo loco task integrity_resign op:apply
//! cargo loco task integrity_resign op:apply target:records limit:5000
//! ```
//!
//! ## The rule that matters: never re-sign what does not verify
//!
//! A row is re-signed **only** when its existing MAC verifies under a key
//! this service holds. Everything else is refused and reported:
//!
//! | Situation | What happens | Why |
//! |---|---|---|
//! | MAC verifies under a held key | **re-signed** under the active key | the content is provably as it was when MACed |
//! | MAC does not verify | **refused**, reported as a finding | re-signing would compute a *valid* MAC over tampered content — laundering the tampering into an assertion of integrity, and destroying the only evidence that it happened |
//! | No MAC at all | **refused**, reported | stamping one would assert "unchanged since it was MACed" about a row that never was. A later verifier would read `Valid` and conclude something untrue |
//! | MAC names a key we do not hold | **refused**, reported | it cannot be verified, so it cannot be re-signed; load the old key first |
//! | Already under the active key and scheme | skipped | idempotent, so a re-run after a partial pass is safe |
//!
//! The refusal to back-fill is deliberate and matches how the digests
//! treat rows that predate them. Adopting a control must not manufacture
//! assurance about history it was not present for. An operator who wants
//! MACs on old rows is asking for a different, weaker claim than the one
//! `Valid` makes, and should not get it under this name.
//!
//! ## Ordering and interruption
//!
//! Re-signing is row-local: the MAC lives in its own column and is not
//! part of the hash chain, so re-signing cannot break chain linkage and
//! rows may be processed in any order. An interrupted run leaves a mix of
//! old-key and new-key rows, which is exactly the state rotation already
//! supports — re-run to finish.

use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::compliance::mac::{self, MacVerdict};

/// Default rows examined in one pass.
pub const DEFAULT_LIMIT: u64 = 10_000;

/// Which tables to walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// `audit_log` only.
    Audit,
    /// `workers` only.
    Records,
    /// Both.
    All,
}

/// What to do with one row's stored MAC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Already carries the active key and current scheme.
    AlreadyCurrent,
    /// Verified under a held key; re-sign under the active key.
    Resign,
    /// The MAC does not verify. **A finding, never re-signed.**
    RefuseUnverified,
    /// No MAC to re-sign, and stamping one would assert what we cannot.
    RefuseAbsent,
    /// Names a key or scheme this service cannot check.
    RefuseUnverifiable,
}

/// Decide what to do with one row, given its stored MAC and the verdict.
///
/// Pure, so the rule that actually protects the trail — never re-sign
/// what does not verify — is unit-testable without a database.
#[must_use]
pub fn decide(stored: Option<&str>, verdict: &MacVerdict, active_prefix: &str) -> Decision {
    match verdict {
        // The load-bearing case. An invalid MAC means the content changed
        // without the key; re-signing it would produce a MAC that
        // verifies, converting evidence of tampering into a clean bill of
        // health. Refuse, loudly.
        MacVerdict::Invalid => Decision::RefuseUnverified,
        MacVerdict::Absent => Decision::RefuseAbsent,
        MacVerdict::UnknownKey(_) | MacVerdict::UnknownScheme(_) | MacVerdict::Malformed => {
            Decision::RefuseUnverifiable
        }
        MacVerdict::Valid => {
            if stored.is_some_and(|s| s.starts_with(active_prefix)) {
                Decision::AlreadyCurrent
            } else {
                Decision::Resign
            }
        }
    }
}

/// What one pass did, or would do.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct ResignReport {
    /// Rows examined.
    pub examined: usize,
    /// Rows re-signed (or that would be, in a dry run).
    pub resigned: usize,
    /// Rows already under the active key and scheme.
    pub already_current: usize,
    /// Rows with no MAC — refused rather than back-filled.
    pub refused_absent: usize,
    /// Rows naming a key or scheme this service cannot check.
    pub refused_unverifiable: usize,
    /// **Rows whose MAC did not verify.** Any non-zero value here is a
    /// tampering finding and must be investigated before the retired key
    /// is destroyed — destroying it removes the ability to reproduce the
    /// evidence.
    pub refused_unverified: usize,
    /// Identifiers of the rows that failed verification, capped so a
    /// wholesale failure cannot produce unbounded output.
    pub unverified_ids: Vec<String>,
}

/// How many failing ids to name before truncating the list.
const MAX_REPORTED_IDS: usize = 50;

impl ResignReport {
    /// Fold one decision in.
    fn record(&mut self, decision: &Decision, id: &str) {
        self.examined += 1;
        match decision {
            Decision::Resign => self.resigned += 1,
            Decision::AlreadyCurrent => self.already_current += 1,
            Decision::RefuseAbsent => self.refused_absent += 1,
            Decision::RefuseUnverifiable => self.refused_unverifiable += 1,
            Decision::RefuseUnverified => {
                self.refused_unverified += 1;
                if self.unverified_ids.len() < MAX_REPORTED_IDS {
                    self.unverified_ids.push(id.to_string());
                }
            }
        }
    }

    /// Whether the retired key can safely be dropped after this pass.
    ///
    /// Only when nothing was left behind: no row still needs re-signing,
    /// and — critically — nothing failed verification. Dropping the old
    /// key while an unverified row exists destroys the ability to
    /// reproduce what that row's MAC was, which is the evidence.
    #[must_use]
    pub fn safe_to_retire_old_key(&self) -> bool {
        self.refused_unverified == 0 && self.refused_unverifiable == 0 && self.resigned == 0
    }
}

/// Parse `op:` / `target:` / `limit:`.
///
/// Dry-run is the default: a task that rewrites integrity metadata should
/// not do so because someone typed its name.
///
/// # Errors
///
/// On an unknown `op:` or `target:`, or an unparseable `limit:`.
pub fn parse(
    vars: &std::collections::BTreeMap<String, String>,
) -> std::result::Result<(bool, Target, u64), String> {
    let apply = match vars.get("op").map_or("dry-run", String::as_str) {
        "dry-run" | "dry_run" | "check" | "plan" => false,
        "apply" | "run" => true,
        other => return Err(format!("unknown op:{other} — expected dry-run or apply")),
    };
    let target = match vars.get("target").map_or("all", String::as_str) {
        "all" => Target::All,
        "audit" | "audit_logs" => Target::Audit,
        "records" | "workers" => Target::Records,
        other => {
            return Err(format!(
                "unknown target:{other} — expected all, audit, records"
            ));
        }
    };
    let limit = match vars.get("limit") {
        None => DEFAULT_LIMIT,
        Some(raw) => raw
            .parse::<u64>()
            .map_err(|_| format!("limit:{raw} is not a number"))?,
    };
    Ok((apply, target, limit))
}

/// The `integrity_resign` CLI task.
pub struct IntegrityResign;

#[async_trait]
impl Task for IntegrityResign {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "integrity_resign".to_string(),
            detail: "Re-MAC history under the current key after a rotation".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let (apply, target, limit) = parse(&vars.cli).map_err(|e| Error::string(&e))?;

        // Without an active key there is nothing to re-sign *to*, and
        // proceeding would report every row as refused, which reads like a
        // data problem rather than a configuration one.
        let Some(active_id) = mac::active_key_id() else {
            return Err(Error::string(
                "no active MAC key configured; set the key before re-signing",
            ));
        };
        let active_prefix = mac::active_prefix().unwrap_or_default();
        println!(
            "{} under key id {active_id} (scheme prefix {active_prefix})",
            if apply { "re-signing" } else { "dry run" }
        );

        let mut report = ResignReport::default();
        if matches!(target, Target::Audit | Target::All) {
            resign_audit(ctx, apply, limit, &active_prefix, &mut report).await?;
        }
        if matches!(target, Target::Records | Target::All) {
            resign_records(ctx, apply, limit, &active_prefix, &mut report).await?;
        }

        println!("{}", serde_json::to_string_pretty(&report)?);
        if report.refused_unverified > 0 {
            println!(
                "\nWARNING: {} row(s) failed MAC verification and were NOT re-signed.\n\
                 These are tampering findings. Investigate before destroying the retired\n\
                 key — destroying it removes the ability to reproduce the evidence.",
                report.refused_unverified
            );
        }
        if !apply && report.resigned > 0 {
            println!("\nre-run with op:apply to write these changes");
        }
        Ok(())
    }
}

/// Walk the audit trail.
async fn resign_audit(
    ctx: &AppContext,
    apply: bool,
    limit: u64,
    active_prefix: &str,
    report: &mut ResignReport,
) -> Result<()> {
    use crate::compliance::audit_chain;
    use crate::db::models::audit_log;
    use sea_orm::ActiveModelTrait as _;
    use sea_orm::ActiveValue::Set;

    let rows = audit_log::Entity::find()
        .filter(audit_log::Column::Mac.is_not_null())
        .order_by_asc(audit_log::Column::Seq)
        .limit(limit)
        .all(&ctx.db)
        .await?;

    for row in rows {
        let preimage =
            audit_chain::preimage(&audit_chain::input_for(&row, row.prev_hash.as_deref()));
        let verdict = mac::verify(mac::Domain::AuditChain, row.mac.as_deref(), &preimage);
        let decision = decide(row.mac.as_deref(), &verdict, active_prefix);
        report.record(&decision, &row.seq.to_string());

        if apply && decision == Decision::Resign {
            let fresh = mac::tag(mac::Domain::AuditChain, &preimage);
            let mut active: audit_log::ActiveModel = row.into();
            active.mac = Set(fresh);
            active.update(&ctx.db).await?;
        }
    }
    Ok(())
}

/// Walk the record table.
///
/// Unlike the loco-shaped services, a record's digest covers its child
/// tables too, so each row must be **assembled through the repository**
/// before its pre-image can be computed — the same assembly
/// `/api/records/verify` does. That is one query per row, which is why
/// the limit matters here more than it does for the audit walk.
async fn resign_records(
    ctx: &AppContext,
    apply: bool,
    limit: u64,
    active_prefix: &str,
    report: &mut ResignReport,
) -> Result<()> {
    use crate::compliance::record_integrity;
    use crate::db::models::workers;
    use crate::db::repositories::{SeaOrmWorkerRepository, WorkerRepository};
    use sea_orm::ActiveModelTrait as _;
    use sea_orm::ActiveValue::Set;

    let repository = SeaOrmWorkerRepository::new(ctx.db.clone());
    let rows = workers::Entity::find()
        .filter(workers::Column::ContentMac.is_not_null())
        .order_by_asc(workers::Column::Id)
        .limit(limit)
        .all(&ctx.db)
        .await?;

    for row in rows {
        let deleted_micros = row
            .deleted_at
            .and_then(|d| i64::try_from(d.unix_timestamp_nanos() / 1_000).ok());
        // A row that vanished between the two queries is skipped rather
        // than reported: it is not a finding.
        let Ok(Some(assembled)) = repository.get_by_id(&row.id).await else {
            continue;
        };
        let Ok(preimage) = record_integrity::preimage_for(&assembled, deleted_micros) else {
            // Unserializable rather than tampered — the same conservative
            // call `verify` makes. Re-signing it would certify bytes we
            // could not reproduce.
            continue;
        };
        let verdict = mac::verify(mac::Domain::Record, row.content_mac.as_deref(), &preimage);
        let decision = decide(row.content_mac.as_deref(), &verdict, active_prefix);
        report.record(&decision, &row.id.to_string());

        if apply && decision == Decision::Resign {
            let fresh = mac::tag(mac::Domain::Record, &preimage);
            let mut active: workers::ActiveModel = row.into();
            active.content_mac = Set(fresh);
            active.update(&ctx.db).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Decision, ResignReport, Target, decide, parse};
    use crate::compliance::mac::MacVerdict;

    fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// **The property this task exists to preserve.** A row whose MAC does
    /// not verify is never re-signed.
    ///
    /// Re-signing it would compute a MAC that *does* verify over tampered
    /// content — laundering the tampering into an assertion of integrity
    /// and destroying the only evidence it happened. This is the one test
    /// that must never be relaxed.
    #[test]
    fn a_row_that_fails_verification_is_never_resigned() {
        let decision = decide(Some("d1.k0:dead"), &MacVerdict::Invalid, "d1.k1:");
        assert_eq!(decision, Decision::RefuseUnverified);
        assert_ne!(decision, Decision::Resign);
    }

    /// A row with no MAC is refused rather than back-filled. Stamping one
    /// would make a later verifier read `Valid` and conclude the row is
    /// unchanged since it was `MAC`ed — about a row that never was.
    #[test]
    fn a_row_with_no_mac_is_not_back_filled() {
        assert_eq!(
            decide(None, &MacVerdict::Absent, "d1.k1:"),
            Decision::RefuseAbsent
        );
    }

    /// A row we cannot check cannot be re-signed: load the old key first.
    #[test]
    fn an_uncheckable_row_is_refused() {
        assert_eq!(
            decide(
                Some("d1.k9:aa"),
                &MacVerdict::UnknownKey("k9".to_string()),
                "d1.k1:"
            ),
            Decision::RefuseUnverifiable
        );
        assert_eq!(
            decide(
                Some("d9.k1:aa"),
                &MacVerdict::UnknownScheme("d9".to_string()),
                "d1.k1:"
            ),
            Decision::RefuseUnverifiable
        );
        assert_eq!(
            decide(Some("junk"), &MacVerdict::Malformed, "d1.k1:"),
            Decision::RefuseUnverifiable
        );
    }

    /// A verified row under an old key is re-signed; under the active key
    /// it is left alone, so a re-run after an interrupted pass is cheap
    /// and idempotent.
    #[test]
    fn verified_rows_are_resigned_once_and_only_once() {
        assert_eq!(
            decide(Some("d1.k0:aa"), &MacVerdict::Valid, "d1.k1:"),
            Decision::Resign,
            "an old key id must be re-signed"
        );
        assert_eq!(
            decide(Some("k0:aa"), &MacVerdict::Valid, "d1.k1:"),
            Decision::Resign,
            "a legacy value with no scheme must be re-signed"
        );
        assert_eq!(
            decide(Some("d1.k1:aa"), &MacVerdict::Valid, "d1.k1:"),
            Decision::AlreadyCurrent,
            "already current is a no-op"
        );
    }

    /// Dry-run is the default: a task that rewrites integrity metadata
    /// must not do so because someone typed its name.
    #[test]
    fn dry_run_is_the_default() {
        let (apply, target, limit) = parse(&vars(&[])).expect("parses");
        assert!(!apply, "must not write without op:apply");
        assert_eq!(target, Target::All);
        assert_eq!(limit, super::DEFAULT_LIMIT);

        let (apply, ..) = parse(&vars(&[("op", "apply")])).expect("parses");
        assert!(apply);
    }

    /// Targets and limits parse, and nonsense is refused rather than
    /// silently treated as a default that does more than asked.
    #[test]
    fn targets_and_limits_parse() {
        assert_eq!(
            parse(&vars(&[("target", "audit")])).expect("parses").1,
            Target::Audit
        );
        assert_eq!(
            parse(&vars(&[("target", "records")])).expect("parses").1,
            Target::Records
        );
        assert_eq!(parse(&vars(&[("limit", "42")])).expect("parses").2, 42);
        assert!(parse(&vars(&[("target", "nope")])).is_err());
        assert!(parse(&vars(&[("op", "nope")])).is_err());
        assert!(parse(&vars(&[("limit", "lots")])).is_err());
    }

    /// The old key is only safe to destroy when nothing was left behind.
    /// A pass that still has rows to re-sign, or any row it could not
    /// verify, means destroying the key loses evidence or strands rows.
    #[test]
    fn retiring_the_old_key_requires_a_completely_clean_pass() {
        let clean = ResignReport {
            examined: 10,
            already_current: 10,
            ..ResignReport::default()
        };
        assert!(clean.safe_to_retire_old_key());

        for spoiled in [
            ResignReport {
                refused_unverified: 1,
                ..clean.clone()
            },
            ResignReport {
                refused_unverifiable: 1,
                ..clean.clone()
            },
            ResignReport {
                resigned: 1,
                ..clean.clone()
            },
        ] {
            assert!(
                !spoiled.safe_to_retire_old_key(),
                "{spoiled:?} must not be called safe"
            );
        }
    }

    /// The failing-id list is capped, so a wholesale failure reports a
    /// usable summary rather than megabytes of ids.
    #[test]
    fn the_failing_id_list_is_capped() {
        let mut report = ResignReport::default();
        for i in 0..500 {
            report.record(&Decision::RefuseUnverified, &i.to_string());
        }
        assert_eq!(report.refused_unverified, 500, "the count is complete");
        assert_eq!(
            report.unverified_ids.len(),
            super::MAX_REPORTED_IDS,
            "the id list is truncated"
        );
    }
}
