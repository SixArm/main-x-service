//! Value frequencies for the match evidence (`agents/share/match-evidence.md`
//! §7).
//!
//! The matcher's agreement weight for a component assumes an average
//! coincidence rate. A chain name ("Starbucks", "Main Street Surgery") is
//! held by many places, so two records agreeing on it prove far less than
//! two agreeing on a rare name. Only this service sees the population, so
//! it counts: for each of the matcher's
//! [`FREQUENCY_COMPONENTS`](place_matcher::evidence::FREQUENCY_COMPONENTS)
//! (for place, `name`), the share of active records holding each
//! normalised value, keyed by the matcher's own
//! [`frequency_key`](place_matcher::evidence::frequency_key) so the
//! service counts in exactly the form the matcher looks up.
//!
//! Two sources use this:
//!
//! - **Batch `deduplicate`** counts over the records the scan loaded
//!   ([`Frequencies::from_places`]).
//! - **Create-time / `check-duplicates`** read a cached table in app state
//!   ([`FrequencyCache`]), rebuilt lazily in the background once older than
//!   [`FREQUENCY_TTL`]. A request never waits on a rebuild: it uses the
//!   table it finds (`NoFrequencies` until the first build completes) and,
//!   if that table is stale, starts one rebuild, of which at most one runs
//!   at a time.
//!
//! Frequencies change only `evidence`; the service's `score` is untouched.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use place_matcher::evidence::{
    FREQUENCY_COMPONENTS, FrequencyTable, NoFrequencies, ValueFrequencies, frequency_key,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::adapter::to_matcher_place;
use crate::models::place::Place;

/// Smallest population a share is taken from. Below it the source is
/// [`NoFrequencies`] and the matcher's plain `u` table applies: a share
/// counted over a few hundred places is noise (one extra "Starbucks" moves
/// it by a large fraction), and a noisy share below the table's `u` would
/// hand out bits the population does not support.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most active records the cached table counts. Above this the table is a
/// **sample**: the first `FREQUENCY_SCAN_CAP` active places in `id` order
/// (UUID v4 ids, so effectively a random sample), not the whole registry.
/// It bounds one rebuild's work and memory.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Rows fetched per page while rebuilding the cached table.
pub const FREQUENCY_PAGE_SIZE: u64 = 1_000;

/// Age at which the cached table is rebuilt.
pub const FREQUENCY_TTL: Duration = Duration::from_mins(10);

/// A frequency source: either counted shares or none at all.
#[derive(Debug, Clone, Default)]
pub enum Frequencies {
    /// Too small a population (or not built yet): the plain table applies.
    #[default]
    None,
    /// Shares counted over `population` records.
    Table {
        /// The counted shares.
        table: FrequencyTable,
        /// How many records they were counted over.
        population: u64,
    },
}

impl Frequencies {
    /// Count over `places`, each mapped through the matcher adapter.
    #[must_use]
    pub fn from_places<'a>(places: impl IntoIterator<Item = &'a Place>) -> Self {
        let mut counter = FrequencyCounter::default();
        for place in places {
            counter.add(place);
        }
        counter.finish()
    }

    /// Whether counted shares are in use.
    #[must_use]
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table { .. })
    }
}

impl ValueFrequencies for Frequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None => NoFrequencies.share(component, value),
            Self::Table { table, .. } => table.share(component, value),
        }
    }
}

/// Counts records holding each `(component, value)`.
#[derive(Debug, Default)]
pub struct FrequencyCounter {
    total: u64,
    counts: HashMap<(String, String), u64>,
}

impl FrequencyCounter {
    /// Count one record. A record holding a value under several of its
    /// names (primary and alternate) counts once for that value.
    pub fn add(&mut self, place: &Place) {
        self.total += 1;
        let m = to_matcher_place(place);
        for &component in FREQUENCY_COMPONENTS {
            let values: BTreeSet<String> = match component {
                "name" => m
                    .name
                    .iter()
                    .chain(m.alternate_names.iter())
                    .filter_map(|n| frequency_key(component, n))
                    .collect(),
                _ => BTreeSet::new(),
            };
            for value in values {
                *self
                    .counts
                    .entry((component.to_string(), value))
                    .or_insert(0) += 1;
            }
        }
    }

    /// Records counted so far.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// The shares, or [`Frequencies::None`] below
    /// [`MIN_FREQUENCY_POPULATION`].
    #[must_use]
    pub fn finish(self) -> Frequencies {
        if self.total < MIN_FREQUENCY_POPULATION {
            return Frequencies::None;
        }
        let mut table = FrequencyTable::new();
        for ((component, value), count) in self.counts {
            if let Some(share) = FrequencyTable::from_counts(count, self.total) {
                table.insert(component, value, share);
            }
        }
        Frequencies::Table {
            table,
            population: self.total,
        }
    }
}

/// Count over active places in the database, paging by `id`, up to
/// [`FREQUENCY_SCAN_CAP`] records.
///
/// # Errors
/// Propagates any database error from a page read.
pub async fn scan_frequencies(db: &PgPool) -> crate::Result<Frequencies> {
    let mut counter = FrequencyCounter::default();
    let mut after: Option<Uuid> = None;
    while counter.total() < FREQUENCY_SCAN_CAP {
        let want = FREQUENCY_PAGE_SIZE.min(FREQUENCY_SCAN_CAP - counter.total());
        let rows = crate::db::active_names_page(db, after, want).await?;
        let Some(last) = rows.last() else { break };
        after = Some(last.id);
        let short = (rows.len() as u64) < want;
        for row in rows {
            let mut place = Place::new(&row.name);
            place.alternate_name = row.alternate_name;
            counter.add(&place);
        }
        if short {
            break;
        }
    }
    Ok(counter.finish())
}

/// The cached create-time / `check-duplicates` frequency table.
#[derive(Debug, Default)]
pub struct FrequencyCache {
    current: RwLock<(Option<Instant>, Arc<Frequencies>)>,
    rebuilding: AtomicBool,
}

impl FrequencyCache {
    /// An empty cache: [`Frequencies::None`] until the first build.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current table, without waiting. When it is missing or older
    /// than [`FREQUENCY_TTL`], starts a background rebuild (at most one at
    /// a time) and returns the table it has; the next request after the
    /// rebuild finishes sees the new one. A failed rebuild logs a warning
    /// and keeps the old table.
    #[must_use]
    pub fn get(self: &Arc<Self>, db: &PgPool) -> Arc<Frequencies> {
        let (built_at, table) = {
            let guard = self
                .current
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (guard.0, Arc::clone(&guard.1))
        };
        let stale = built_at.is_none_or(|t| t.elapsed() >= FREQUENCY_TTL);
        if stale
            && self
                .rebuilding
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            let cache = Arc::clone(self);
            let db = db.clone();
            tokio::spawn(async move {
                match scan_frequencies(&db).await {
                    Ok(fresh) => cache.store(fresh),
                    Err(e) => tracing::warn!(error = %e, "value-frequency rebuild failed"),
                }
                cache.rebuilding.store(false, Ordering::Release);
            });
        }
        table
    }

    /// Replace the table, stamped now.
    pub fn store(&self, frequencies: Frequencies) {
        let mut guard = self
            .current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = (Some(Instant::now()), Arc::new(frequencies));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use place_matcher::evidence::NAME;
    use place_matcher::{MatchingEngine, Place as MPlace};

    fn population(n: u64, common: &str, every: u64) -> Vec<Place> {
        (0..n)
            .map(|i| {
                if i % every == 0 {
                    Place::new(common)
                } else {
                    Place::new(&format!("Unique Place {i}"))
                }
            })
            .collect()
    }

    #[test]
    fn below_the_minimum_population_is_no_frequencies() {
        let places = population(MIN_FREQUENCY_POPULATION - 1, "Starbucks", 2);
        let f = Frequencies::from_places(&places);
        assert!(!f.is_table());
        assert_eq!(f.share("name", "starbucks"), None);
    }

    #[test]
    fn at_the_minimum_population_shares_are_counted_in_matcher_form() {
        let places = population(MIN_FREQUENCY_POPULATION, "  Starbucks! ", 4);
        let f = Frequencies::from_places(&places);
        assert!(f.is_table());
        let share = f.share("name", "starbucks").expect("counted");
        assert!((share - 0.25).abs() < 1e-9, "{share}");
    }

    #[test]
    fn a_record_counts_once_for_a_value_it_holds_twice() {
        let mut places = population(MIN_FREQUENCY_POPULATION, "Other", 1_000_000);
        places[1].name = "Starbucks".into();
        places[1].alternate_name = Some("STARBUCKS".into());
        let f = Frequencies::from_places(&places);
        let share = f.share("name", "starbucks").expect("counted");
        assert!((share - 1.0 / 1_000.0).abs() < 1e-12, "{share}");
    }

    #[test]
    fn a_common_name_gives_fewer_bits_than_the_table_default() {
        let places = population(2_000, "Starbucks", 4);
        let f = Frequencies::from_places(&places);
        let engine = MatchingEngine::default_config();
        let a = MPlace::builder().name("Starbucks").build();
        let b = MPlace::builder().name("Starbucks").build();
        let plain = engine.match_places(&a, &b);
        let adjusted = engine.match_places_with_frequencies(&a, &b, &f);
        assert!((plain.score - adjusted.score).abs() < f64::EPSILON);
        assert!(
            adjusted.evidence.weight_bits < plain.evidence.weight_bits,
            "{} !< {}",
            adjusted.evidence.weight_bits,
            plain.evidence.weight_bits
        );
        let default_bits = (NAME.m / NAME.u).log2();
        assert!(adjusted.evidence.weight_bits < default_bits);
        assert!(
            adjusted
                .evidence
                .components
                .iter()
                .any(|c| c.component == "name" && c.frequency_adjusted)
        );
    }

    #[tokio::test]
    async fn the_cache_returns_what_was_stored_without_waiting() {
        let cache = Arc::new(FrequencyCache::new());
        cache.store(Frequencies::from_places(&population(1_000, "Starbucks", 2)));
        // A lazy pool never connects unless queried; a fresh table means
        // no rebuild is started.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://nobody@127.0.0.1:1/none")
            .expect("lazy pool");
        let f = cache.get(&pool);
        assert!(f.is_table());
        assert!(!cache.rebuilding.load(Ordering::Acquire));
    }
}
