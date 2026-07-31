//! `search_reindex` task — rebuild the Tantivy index from Postgres.
//!
//! The database is the source of truth and the index is a derived
//! artifact ([`crate::search`]), so it must be reconstructible from
//! scratch. This is the repair path for every way the two can diverge:
//! a fresh deployment whose index directory is empty, an index lost with
//! an ephemeral disk, a write whose best-effort indexing failed, or a
//! schema change to the indexed field set.
//!
//! ```text
//! cargo loco task search_reindex            # rebuild everything
//! cargo loco task search_reindex batch:1000 # tune the page size
//! ```

use loco_rs::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::models::_entities::organizations;

/// Rows fetched per page when replaying the table. Bounds peak memory:
/// a reindex must not need the whole table resident.
pub const DEFAULT_BATCH: u64 = 500;

/// Counts from one rebuild pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReindexReport {
    /// Rows read from the database.
    pub scanned: usize,
    /// Rows written to the index.
    pub indexed: usize,
    /// Rows skipped because their stored payload would not deserialize.
    pub skipped: usize,
}

/// Rebuild the index from every active row, oldest first.
///
/// Clears first, so a record deleted while the index was stale does not
/// survive the rebuild — the point of a rebuild is that the result
/// depends only on the database, never on what the index held before.
///
/// A row whose payload will not deserialize is **skipped and counted**,
/// not fatal: one corrupt row must not stop the other 99 999 from being
/// searchable.
///
/// # Errors
///
/// When the index is unavailable or a database page cannot be read.
pub async fn reindex(db: &DatabaseConnection, batch: u64) -> Result<ReindexReport> {
    let engine = crate::search::engine().ok_or_else(|| {
        Error::string("search index unavailable; check ORGANIZATION_SEARCH_INDEX_PATH")
    })?;
    engine.clear()?;

    let mut report = ReindexReport::default();
    // Paginate rather than `all()`: the table can be far larger than the
    // process's memory budget.
    let mut pages = organizations::Entity::find()
        .filter(organizations::Column::DeletedAt.is_null())
        .order_by_asc(organizations::Column::Id)
        .paginate(db, batch.max(1));

    while let Some(rows) = pages.fetch_and_next().await? {
        for row in rows {
            report.scanned += 1;
            match row.to_org() {
                Ok(org) => {
                    engine.index_organization(row.pid, &org)?;
                    report.indexed += 1;
                }
                Err(err) => {
                    report.skipped += 1;
                    tracing::error!(pid = %row.pid, error = %err, "reindex: unreadable payload; skipped");
                }
            }
        }
    }
    tracing::info!(
        scanned = report.scanned,
        indexed = report.indexed,
        skipped = report.skipped,
        "search reindex complete"
    );
    Ok(report)
}

/// Environment switch for the boot-time rebuild ([`boot_reindex_enabled`]).
pub const BOOT_REINDEX_ENV: &str = "ORGANIZATION_SEARCH_BOOT_REINDEX";

/// Whether the boot-time rebuild is enabled. Default **on**, because the
/// failure it prevents (a live deployment serving empty search results
/// after an upgrade or a lost volume) is silent. An operator with a very
/// large table can set the variable to a falsy value and rebuild by hand
/// instead.
#[must_use]
pub fn boot_reindex_enabled() -> bool {
    boot_reindex_enabled_for(std::env::var(BOOT_REINDEX_ENV).ok().as_deref())
}

/// The pure decision behind [`boot_reindex_enabled`]: unset ⇒ on, and
/// only an explicitly falsy spelling turns it off.
#[must_use]
fn boot_reindex_enabled_for(value: Option<&str>) -> bool {
    value.is_none_or(|v| {
        !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        )
    })
}

/// Rebuild the index **only if it is empty** while the table is not —
/// the upgrade path for a service whose records predate the index (or
/// whose index directory did not survive a restart).
///
/// Returns `None` when no rebuild was needed (or possible), so a caller
/// — and a test — can tell "skipped" from "rebuilt nothing".
///
/// # Errors
///
/// When the row count fails, or the rebuild itself does.
pub async fn reindex_if_empty(db: &DatabaseConnection) -> Result<Option<ReindexReport>> {
    let Some(engine) = crate::search::engine() else {
        // The error was already logged at engine init; nothing to rebuild
        // into.
        return Ok(None);
    };
    if engine.stats()?.num_docs > 0 {
        return Ok(None);
    }
    let rows = organizations::Entity::find()
        .filter(organizations::Column::DeletedAt.is_null())
        .count(db)
        .await?;
    // An empty index over an empty table is the normal fresh-install
    // case, not a rebuild.
    if rows == 0 {
        return Ok(None);
    }
    tracing::info!(rows, "search index is empty but records exist; reindexing");
    reindex(db, DEFAULT_BATCH).await.map(Some)
}

/// Run [`reindex_if_empty`] in the background, so boot is not blocked by
/// a rebuild proportional to table size. Failures are logged, never
/// fatal — a service that cannot rebuild its index should still serve
/// everything else.
pub fn spawn_reindex_if_empty(db: DatabaseConnection) {
    if !boot_reindex_enabled() {
        tracing::debug!("boot reindex disabled by {BOOT_REINDEX_ENV}");
        return;
    }
    tokio::spawn(async move {
        if let Err(err) = reindex_if_empty(&db).await {
            tracing::error!(error = %err, "boot reindex failed; run `cargo loco task search_reindex`");
        }
    });
}

/// The `search_reindex` CLI task.
pub struct SearchReindex;

#[async_trait]
impl Task for SearchReindex {
    /// Task metadata (name + one-line description) for the loco CLI.
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "search_reindex".to_string(),
            detail: "Rebuild the full-text search index from the database. Args: [batch:<rows-per-page>]."
                .to_string(),
        }
    }

    /// Rebuild the index and print the resulting counts.
    ///
    /// # Errors
    ///
    /// When `batch:` is not a positive integer, the index is
    /// unavailable, or a database read fails.
    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let batch = match vars.cli.get("batch") {
            Some(raw) => raw
                .parse::<u64>()
                .map_err(|e| Error::string(&format!("batch must be a positive integer: {e}")))?,
            None => DEFAULT_BATCH,
        };
        let report = reindex(&ctx.db, batch).await?;
        println!(
            "reindexed {} of {} organizations ({} skipped)",
            report.indexed, report.scanned, report.skipped
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh report is all zeros, so a rebuild that indexes nothing is
    /// distinguishable from one that was never run.
    #[test]
    fn report_starts_empty() {
        let report = ReindexReport::default();
        assert_eq!(report.scanned, 0);
        assert_eq!(report.indexed, 0);
        assert_eq!(report.skipped, 0);
    }

    /// The task advertises the name operators type.
    #[test]
    fn task_is_named_search_reindex() {
        assert_eq!(SearchReindex.task().name, "search_reindex");
    }

    /// The boot rebuild is on unless explicitly switched off, and only
    /// the falsy spellings switch it off — a stray value must not
    /// silently disable the self-heal.
    #[test]
    fn boot_reindex_defaults_on() {
        assert!(boot_reindex_enabled_for(None));
        for on in ["1", "true", "yes", "on", "", "junk"] {
            assert!(boot_reindex_enabled_for(Some(on)), "value {on:?}");
        }
        for off in ["0", "false", "no", "off", " OFF "] {
            assert!(!boot_reindex_enabled_for(Some(off)), "value {off:?}");
        }
    }
}
