//! Value frequencies for match evidence (`agents/share/match-evidence.md`
//! §7).
//!
//! The matcher weighs an exact agreement on a frequency component (for
//! this crate only the pathway `name`) by the share of records holding
//! that value, in place of its table `u`: two records both titled
//! "Stroke Pathway" prove far less than two sharing a distinctive title.
//! Only the service sees the population, so it counts the shares here.
//!
//! Two sources:
//!
//! - [`from_pathways`] — counts over a given set of records. The batch
//!   `deduplicate` scan builds one from the rows it loaded.
//! - [`current`] — a process-wide cached table over the active registry,
//!   used by `check-duplicates` and the bulk-import keyless-duplicate
//!   path. It is rebuilt when older than [`FREQUENCY_MAX_AGE`]: **lazily
//!   on use**. A stale table is served while one background task
//!   rebuilds it; only when no table exists yet does the request that
//!   wins the rebuild build it inline, which is one bounded scan, and
//!   concurrent requests meanwhile get [`NoFrequencies`] rather than
//!   waiting.
//!
//! A population smaller than [`MIN_FREQUENCY_POPULATION`] yields
//! [`NoFrequencies`], so every agreement uses the matcher's own table.
//! Frequencies only ever change `evidence`, never `score`, `is_match` or
//! `confidence`. The table holds normalised names in memory and is never
//! serialised into a response.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use care_pathway_matcher::evidence::FREQUENCY_COMPONENTS;
use care_pathway_matcher::{
    CarePathway, FrequencyTable, NoFrequencies, ValueFrequencies, frequency_key,
};
use sqlx::PgPool;

use crate::models::care_pathways::Model as PathwayModel;

/// Smallest population a share is counted from. Below it a share is
/// noise — one record in a population of fifty is "2%", which says
/// nothing about how common the title is — so the source is
/// [`NoFrequencies`] and every agreement uses the matcher's table `u`.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most records the cached table scans. Above this cap the table is a
/// **sample**: the first `FREQUENCY_SCAN_CAP` active records, newest
/// first, and each share is counted over that sample, not the whole
/// registry.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Rows read per page while scanning for the cached table.
pub const FREQUENCY_SCAN_PAGE: u64 = 1_000;

/// Age past which the cached table is rebuilt.
pub const FREQUENCY_MAX_AGE: Duration = Duration::from_mins(10);

/// A frequency source: a counted table, or none at all.
#[derive(Debug, Clone)]
pub enum Frequencies {
    /// The population was too small (or unreadable); use the table `u`.
    None(NoFrequencies),
    /// Shares counted over a population of at least
    /// [`MIN_FREQUENCY_POPULATION`].
    Table(FrequencyTable),
}

impl Frequencies {
    /// Whether this source holds a counted table.
    #[must_use]
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }
}

impl ValueFrequencies for Frequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None(n) => n.share(component, value),
            Self::Table(t) => t.share(component, value),
        }
    }
}

/// The raw value `pathway` holds for a frequency `component`, before
/// the matcher's normalisation.
fn component_value<'a>(pathway: &'a CarePathway, component: &str) -> Option<&'a str> {
    match component {
        "name" => Some(pathway.name.as_str()),
        _ => None,
    }
}

/// Count the shares of every [`FREQUENCY_COMPONENTS`] value over
/// `pathways`, each value keyed with the matcher's [`frequency_key`].
/// The population is the number of records given; below
/// [`MIN_FREQUENCY_POPULATION`] the result is [`Frequencies::None`].
pub fn from_pathways<'a, I>(pathways: I) -> Frequencies
where
    I: IntoIterator<Item = &'a CarePathway>,
{
    let mut counts: std::collections::HashMap<(&'static str, String), u64> =
        std::collections::HashMap::new();
    let mut total: u64 = 0;
    for pathway in pathways {
        total += 1;
        for &component in FREQUENCY_COMPONENTS {
            if let Some(key) =
                component_value(pathway, component).and_then(|raw| frequency_key(component, raw))
            {
                *counts.entry((component, key)).or_default() += 1;
            }
        }
    }
    if total < MIN_FREQUENCY_POPULATION {
        return Frequencies::None(NoFrequencies);
    }
    Frequencies::Table(FrequencyTable::from_counts(
        counts.into_iter().map(|((c, v), n)| (c, v, n)),
        total,
    ))
}

/// Scan the active registry, a page at a time, up to
/// [`FREQUENCY_SCAN_CAP`] records, and count its frequencies.
///
/// # Errors
///
/// When a page read fails. A row whose payload does not parse is
/// skipped and not counted in the population.
pub async fn scan(db: &PgPool) -> axa::Result<Frequencies> {
    let mut pathways: Vec<CarePathway> = Vec::new();
    let mut offset: u64 = 0;
    while offset < FREQUENCY_SCAN_CAP {
        let page = FREQUENCY_SCAN_PAGE.min(FREQUENCY_SCAN_CAP - offset);
        let rows = PathwayModel::list_paged(db, page, offset).await?;
        let read = rows.len() as u64;
        pathways.extend(rows.iter().filter_map(|r| r.to_pathway().ok()));
        offset += read;
        if read < page {
            break;
        }
    }
    if offset >= FREQUENCY_SCAN_CAP {
        tracing::info!(
            cap = FREQUENCY_SCAN_CAP,
            "value-frequency scan hit its cap; the table is a sample of the registry"
        );
    }
    Ok(from_pathways(&pathways))
}

/// The cached table and when it was built.
struct Cached {
    built: Instant,
    frequencies: Arc<Frequencies>,
}

/// The process-wide cached table.
static CACHE: RwLock<Option<Cached>> = RwLock::new(None);

/// Set while a rebuild runs, so at most one scan runs at a time.
static REBUILDING: AtomicBool = AtomicBool::new(false);

/// Clears [`REBUILDING`] when a rebuild ends, however it ends.
struct RebuildGuard;

impl Drop for RebuildGuard {
    fn drop(&mut self) {
        REBUILDING.store(false, Ordering::Release);
    }
}

/// Claim the single rebuild slot, or `None` when one is already running.
fn claim_rebuild() -> Option<RebuildGuard> {
    REBUILDING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()
        .map(|_| RebuildGuard)
}

/// Rebuild the cache with one scan. A failed scan leaves the old table
/// (if any) in place and is logged.
async fn rebuild(db: &PgPool) -> Option<Arc<Frequencies>> {
    match scan(db).await {
        Ok(frequencies) => {
            let frequencies = Arc::new(frequencies);
            if let Ok(mut cache) = CACHE.write() {
                *cache = Some(Cached {
                    built: Instant::now(),
                    frequencies: Arc::clone(&frequencies),
                });
            }
            Some(frequencies)
        }
        Err(e) => {
            tracing::warn!("value-frequency scan failed; evidence uses the table u: {e}");
            None
        }
    }
}

/// The cached frequency source for request-time matching.
///
/// Fresh: returned as is. Stale: returned as is while a background task
/// rebuilds it. Absent: the caller that claims the rebuild scans inline
/// (one bounded scan); any other caller gets [`NoFrequencies`].
pub async fn current(db: &PgPool) -> Arc<Frequencies> {
    let cached = CACHE.read().ok().and_then(|c| {
        c.as_ref()
            .map(|c| (c.built.elapsed(), Arc::clone(&c.frequencies)))
    });
    match cached {
        Some((age, frequencies)) if age < FREQUENCY_MAX_AGE => frequencies,
        Some((_, stale)) => {
            if let Some(guard) = claim_rebuild() {
                let db = db.clone();
                tokio::spawn(async move {
                    let _guard = guard;
                    rebuild(&db).await;
                });
            }
            stale
        }
        None => {
            if let Some(_guard) = claim_rebuild()
                && let Some(frequencies) = rebuild(db).await
            {
                return frequencies;
            }
            Arc::new(Frequencies::None(NoFrequencies))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use care_pathway_matcher::{MatchStrength, MatchingEngine};

    fn population(n: u64, common: u64) -> Vec<CarePathway> {
        (0..n)
            .map(|i| {
                if i < common {
                    CarePathway::new("Stroke Pathway")
                } else {
                    CarePathway::new(format!("Distinct Pathway {i}"))
                }
            })
            .collect()
    }

    #[test]
    fn below_the_minimum_population_there_are_no_frequencies() {
        let pathways = population(MIN_FREQUENCY_POPULATION - 1, 500);
        let f = from_pathways(&pathways);
        assert!(!f.is_table());
        assert_eq!(f.share("name", "stroke pathway"), None);
    }

    #[test]
    fn at_the_minimum_population_shares_are_counted_in_the_matcher_key() {
        let pathways = population(MIN_FREQUENCY_POPULATION, 500);
        let f = from_pathways(&pathways);
        assert!(f.is_table());
        let key = frequency_key("name", "  STROKE Pathway ").expect("key");
        assert_eq!(f.share("name", &key), Some(0.5));
    }

    #[test]
    fn a_common_name_gives_fewer_bits_than_the_table_default() {
        let pathways = population(2_000, 1_000);
        let f = from_pathways(&pathways);
        let engine = MatchingEngine::default_config();
        let a = CarePathway::new("Stroke Pathway");
        let b = CarePathway::new("Stroke Pathway");
        let plain = engine.match_care_pathways(&a, &b);
        let weighed = engine.match_care_pathways_with_frequencies(&a, &b, &f);
        assert!(
            weighed.evidence.weight_bits < plain.evidence.weight_bits,
            "{} vs {}",
            weighed.evidence.weight_bits,
            plain.evidence.weight_bits
        );
        assert!(weighed.evidence.components[0].frequency_adjusted);
        // The score is untouched; only evidence changes.
        assert!((weighed.score - plain.score).abs() < 1e-12);
        assert_eq!(weighed.evidence.strength, MatchStrength::Consistent);
    }
}
