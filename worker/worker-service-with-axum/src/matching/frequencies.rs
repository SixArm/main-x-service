//! Value frequencies for match evidence (`agents/share/match-evidence.md` §7).
//!
//! Agreeing on "Smith" proves much less than agreeing on "Zbigniewska",
//! and agreeing on a placeholder birth date such as `1900-01-01` proves
//! almost nothing. The matcher adjusts the agreement weight of its
//! [`FREQUENCY_COMPONENTS`] (given name, family name, date of birth) by
//! the agreed value's share of records, but only the service sees the
//! population, so the service counts.
//!
//! - [`FrequencySource::from_workers`] counts a slice of workers: each is
//!   mapped through the adapter, each frequency component's value through
//!   the matcher's [`frequency_key`], and every count divided by the
//!   number of workers. The batch scan builds one from the records it
//!   loaded.
//! - [`FrequencyCache`] holds one for the request paths (create-time
//!   duplicate check, `check-duplicates`, `match`). It is rebuilt when
//!   older than [`FREQUENCY_TABLE_TTL`], **in a background task**: the
//!   request that finds it stale starts the rebuild and is served the
//!   table it already had, so no request ever waits on a scan. Until the
//!   first build finishes, requests get [`FrequencySource::None`].

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use worker_matcher::evidence::{
    FREQUENCY_COMPONENTS, FrequencyTable, NoFrequencies, ValueFrequencies, frequency_key,
};

use crate::db::WorkerRepository;
use crate::matching::adapter::to_matcher_worker;
use crate::models::Worker;

/// The smallest population a frequency table is built from.
///
/// A share is `count / population`. Over a few hundred records one shared
/// value moves it by a whole percentage point, so a "common" value is
/// mostly chance; below a thousand the service supplies no frequencies and
/// the matcher's average `u` stands (`match-evidence.md` §7: "a service
/// should not supply one until the population is large enough").
pub const MIN_FREQUENCY_POPULATION: usize = 1_000;

/// The most active workers a [`FrequencyCache`] rebuild reads.
///
/// The scan pages through the repository and stops here, so a rebuild is
/// bounded in time and memory. Above the cap the table is a **sample**:
/// the first `MAX_FREQUENCY_SCAN` active workers in id order, which is
/// uniform enough (ids are random UUIDs) for a share to be a fair
/// estimate, but not an exact count.
pub const MAX_FREQUENCY_SCAN: usize = 200_000;

/// Page size of a rebuild scan.
pub const FREQUENCY_SCAN_PAGE: usize = 1_000;

/// How long a cached table serves before a request triggers a rebuild.
pub const FREQUENCY_TABLE_TTL: Duration = Duration::from_mins(10);

/// A frequency source: none (population too small) or a counted table.
#[derive(Debug, Clone, Default)]
pub enum FrequencySource {
    /// No frequencies; the matcher's plain `m`/`u` table applies.
    #[default]
    None,
    /// Shares counted over `population` workers.
    Table {
        /// The counted shares.
        table: FrequencyTable,
        /// How many workers were counted.
        population: usize,
    },
}

impl FrequencySource {
    /// Count the frequency components' values over `workers`.
    ///
    /// Returns [`FrequencySource::None`] below
    /// [`MIN_FREQUENCY_POPULATION`].
    #[must_use]
    pub fn from_workers(workers: &[Worker]) -> Self {
        let population = workers.len();
        if population < MIN_FREQUENCY_POPULATION {
            return Self::None;
        }
        let mut counts: HashMap<(&'static str, String), u64> = HashMap::new();
        for worker in workers {
            for (component, key) in keys_of(worker) {
                *counts.entry((component, key)).or_insert(0) += 1;
            }
        }
        let table = FrequencyTable::from_counts(
            counts
                .iter()
                .map(|((component, value), count)| (*component, value.as_str(), *count)),
            population as u64,
        );
        Self::Table { table, population }
    }

    /// The population counted, or `0` for [`FrequencySource::None`].
    #[must_use]
    pub fn population(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Table { population, .. } => *population,
        }
    }

    /// Whether this source supplies no frequencies.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl ValueFrequencies for FrequencySource {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None => NoFrequencies.share(component, value),
            Self::Table { table, .. } => table.share(component, value),
        }
    }
}

/// The `(component, key)` pairs a worker contributes, one per frequency
/// component it holds, in the matcher's own key form.
fn keys_of(worker: &Worker) -> Vec<(&'static str, String)> {
    let m = to_matcher_worker(worker);
    let dob = m.date_of_birth.map(|d| d.format("%Y-%m-%d").to_string());
    FREQUENCY_COMPONENTS
        .iter()
        .filter_map(|&component| {
            let raw = match component {
                "given_name" => m.given_name.clone(),
                "family_name" => m.family_name.clone(),
                "date_of_birth" => dob.clone(),
                _ => None,
            }?;
            frequency_key(component, &raw).map(|k| (component, k))
        })
        .collect()
}

/// A cached [`FrequencySource`] for the request paths, rebuilt in the
/// background when older than [`FREQUENCY_TABLE_TTL`].
#[derive(Debug, Default)]
pub struct FrequencyCache {
    state: RwLock<CacheState>,
}

#[derive(Debug, Default)]
struct CacheState {
    source: Arc<FrequencySource>,
    built_at: Option<Instant>,
    rebuilding: bool,
}

impl FrequencyCache {
    /// An empty cache: serves [`FrequencySource::None`] until built.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current source, starting a background rebuild first if the
    /// cache is empty or older than [`FREQUENCY_TABLE_TTL`] and none is
    /// running. Never waits on the rebuild.
    ///
    /// Needs a Tokio runtime when a rebuild is due.
    pub fn current(
        self: &Arc<Self>,
        repository: &Arc<dyn WorkerRepository>,
    ) -> Arc<FrequencySource> {
        let (source, due) = {
            let mut state = self
                .state
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let expired = state
                .built_at
                .is_none_or(|at| at.elapsed() >= FREQUENCY_TABLE_TTL);
            let due = expired && !state.rebuilding;
            if due {
                state.rebuilding = true;
            }
            (state.source.clone(), due)
        };
        if due {
            let cache = Arc::clone(self);
            let repository = Arc::clone(repository);
            tokio::spawn(async move {
                let built = scan(repository.as_ref()).await;
                cache.store(built);
            });
        }
        source
    }

    /// Install a freshly built source (or, on `None`, just end the
    /// rebuild so the next stale read retries).
    fn store(&self, built: Option<FrequencySource>) {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.rebuilding = false;
        if let Some(source) = built {
            state.source = Arc::new(source);
            state.built_at = Some(Instant::now());
        }
    }

    /// Replace the cached source directly (tests, warm start).
    pub fn set(&self, source: FrequencySource) {
        self.store(Some(source));
    }
}

/// Page through the active workers, up to [`MAX_FREQUENCY_SCAN`], and
/// count them. `None` when a page fails: the old table keeps serving.
async fn scan(repository: &dyn WorkerRepository) -> Option<FrequencySource> {
    let mut workers = Vec::new();
    let mut offset = 0_u64;
    while workers.len() < MAX_FREQUENCY_SCAN {
        let want = FREQUENCY_SCAN_PAGE.min(MAX_FREQUENCY_SCAN - workers.len());
        let page = match repository.list_active(want as u64, offset).await {
            Ok(page) => page,
            Err(error) => {
                tracing::warn!(%error, "frequency table rebuild failed; keeping the old table");
                return None;
            }
        };
        let got = page.len();
        workers.extend(page);
        if got < want {
            break;
        }
        offset += got as u64;
    }
    let source = FrequencySource::from_workers(&workers);
    tracing::debug!(
        population = source.population(),
        sampled = workers.len() >= MAX_FREQUENCY_SCAN,
        "frequency table rebuilt"
    );
    Some(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::matcher_lib::MatchingEngine;
    use crate::models::{Gender, HumanName};
    use chrono::NaiveDate;

    fn worker(family: &str, given: &str, dob: NaiveDate) -> Worker {
        let mut w = Worker::new(
            HumanName {
                use_type: None,
                family: family.into(),
                given: vec![given.into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        );
        w.birth_date = Some(dob);
        w
    }

    /// `n` workers, every one a distinct name and birth date except that
    /// a tenth of them are called Smith.
    fn population(n: usize) -> Vec<Worker> {
        let base = NaiveDate::from_ymd_opt(1950, 1, 1).unwrap();
        (0..n)
            .map(|i| {
                let family = if i % 10 == 0 {
                    "Smith".to_string()
                } else {
                    format!("Family{i}")
                };
                let dob = base + chrono::Duration::days(i64::try_from(i).unwrap());
                worker(&family, &format!("Given{i}"), dob)
            })
            .collect()
    }

    #[test]
    fn below_the_minimum_population_there_are_no_frequencies() {
        let source = FrequencySource::from_workers(&population(MIN_FREQUENCY_POPULATION - 1));
        assert!(source.is_none());
        assert_eq!(source.share("family_name", "smith"), None);
    }

    #[test]
    fn at_the_minimum_population_shares_are_counted() {
        let source = FrequencySource::from_workers(&population(MIN_FREQUENCY_POPULATION));
        assert_eq!(source.population(), MIN_FREQUENCY_POPULATION);
        let share = source.share("family_name", "smith").unwrap();
        assert!((share - 0.1).abs() < 1e-12, "{share}");
        assert!(source.share("family_name", "family7").is_some());
        assert!(source.share("date_of_birth", "1950-01-01").is_some());
        assert!(source.share("given_name", "given3").is_some());
    }

    /// A common value gives fewer bits than the table's default `u`; the
    /// score is unchanged.
    #[test]
    fn a_common_value_gives_fewer_bits_than_the_table_default() {
        let source = FrequencySource::from_workers(&population(2_000));
        let dob = NaiveDate::from_ymd_opt(1990, 6, 1).unwrap();
        let a = to_matcher_worker(&worker("Smith", "Ann", dob));
        let b = to_matcher_worker(&worker("Smith", "Ann", dob));
        let engine = MatchingEngine::default_config();
        let plain = engine.match_workers(&a, &b);
        let adjusted = engine.match_workers_with_frequencies(&a, &b, &source);
        assert!((plain.score - adjusted.score).abs() < f64::EPSILON);
        let bits = |r: &worker_matcher::MatchResult| {
            r.evidence
                .components
                .iter()
                .find(|c| c.component == "family_name")
                .map(|c| (c.bits, c.frequency_adjusted))
                .unwrap()
        };
        let (plain_bits, plain_adjusted) = bits(&plain);
        let (common_bits, common_adjusted) = bits(&adjusted);
        assert!(!plain_adjusted);
        assert!(common_adjusted);
        assert!(
            common_bits < plain_bits,
            "Smith at 10% should weigh less: {common_bits} vs {plain_bits}"
        );
    }

    #[test]
    fn a_fresh_cache_serves_no_frequencies() {
        let cache = FrequencyCache::new();
        let state = cache.state.read().unwrap();
        assert!(state.source.is_none());
        assert!(state.built_at.is_none());
    }

    #[test]
    fn set_installs_a_table() {
        let cache = FrequencyCache::new();
        cache.set(FrequencySource::from_workers(&population(1_000)));
        let state = cache.state.read().unwrap();
        assert_eq!(state.source.population(), 1_000);
        assert!(state.built_at.is_some());
        assert!(!state.rebuilding);
    }
}
