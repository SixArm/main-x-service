//! Value frequencies for the matcher's evidence
//! (`agents/share/match-evidence.md` §7).
//!
//! Agreeing on a common event name ("Annual General Meeting", "Open Mic
//! Night") proves much less than agreeing on a rare one. The matcher
//! takes the share of records holding a value from a
//! [`ValueFrequencies`] source; only the service sees the population, so
//! it counts. Event's one frequency-adjusted component is the name
//! ([`event_matcher::evidence::FREQUENCY_COMPONENTS`]): primary and
//! alternate names, normalised by
//! [`event_matcher::evidence::frequency_key`].
//!
//! Two sources:
//!
//! - **Batch `deduplicate`** builds one from the records its scan loaded
//!   ([`FrequencySource::from_events`]).
//! - **Create-time / `check-duplicates` / `match`** read a cached source
//!   from [`FrequencyCache`], rebuilt in the background when older than
//!   [`FREQUENCY_TTL`]. A request never waits for a rebuild: it uses the
//!   table it has (or none, before the first build finishes) and the
//!   rebuild lands for the next one.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use event_matcher::evidence::{FREQUENCY_COMPONENTS, frequency_key};
use event_matcher::{FrequencyTable, NoFrequencies, ValueFrequencies};

use super::adapter::to_matcher_event;
use crate::Result;
use crate::db::EventRepository;
use crate::models::Event;

/// The smallest population a share is counted from. Below it every
/// lookup is unknown ([`NoFrequencies`]) and the matcher's `m`/`u` table
/// applies unchanged: in a registry of a few hundred events a name held
/// by three of them is a 1% share by accident, and a share that noisy
/// would move the evidence more than it informs it.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// The most active records the cached table scans. Above it the table is
/// a **sample**: the first this many records in id order, which is
/// uncorrelated with their names. Shares from 200 000 records are
/// precise to well under a tenth of a percentage point, which is all a
/// frequency weight needs.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Records fetched per page of the cached table's scan.
pub const FREQUENCY_SCAN_PAGE: u64 = 5_000;

/// How long a cached table is used before a rebuild is started.
pub const FREQUENCY_TTL: Duration = Duration::from_mins(10);

/// A frequency source: a counted table, or nothing when the population is
/// below [`MIN_FREQUENCY_POPULATION`].
#[derive(Debug, Clone)]
pub enum FrequencySource {
    /// No shares known; the matcher's plain table applies.
    None(NoFrequencies),
    /// Shares counted over `population` records.
    Table {
        /// The counted shares.
        table: FrequencyTable,
        /// How many records they were counted over.
        population: u64,
    },
}

impl Default for FrequencySource {
    fn default() -> Self {
        Self::None(NoFrequencies)
    }
}

impl ValueFrequencies for FrequencySource {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None(n) => n.share(component, value),
            Self::Table { table, .. } => table.share(component, value),
        }
    }
}

impl FrequencySource {
    /// Whether this source knows no shares.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None(_))
    }

    /// Build a source from the records a scan loaded.
    #[must_use]
    pub fn from_events(events: &[Event]) -> Self {
        let mut counter = Counter::default();
        for e in events {
            counter.add(e);
        }
        counter.finish()
    }
}

/// Counts, per frequency component, how many records hold each key.
#[derive(Debug, Default)]
pub struct Counter {
    /// `(component, key) → records holding it`.
    counts: HashMap<(&'static str, String), u64>,
    /// Records counted.
    population: u64,
}

impl Counter {
    /// Count one record: map it through the adapter, then count each
    /// distinct [`frequency_key`] it holds per component, once per record.
    pub fn add(&mut self, event: &Event) {
        let m = to_matcher_event(event);
        for &component in FREQUENCY_COMPONENTS {
            let raws: Vec<&str> = match component {
                "name" => m
                    .name
                    .iter()
                    .chain(m.alternate_names.iter())
                    .map(String::as_str)
                    .collect(),
                // A component the matcher adds later counts nothing until
                // it is wired here; it then simply has no shares.
                _ => Vec::new(),
            };
            let keys: BTreeSet<String> = raws
                .into_iter()
                .filter_map(|r| frequency_key(component, r))
                .collect();
            for key in keys {
                *self.counts.entry((component, key)).or_insert(0) += 1;
            }
        }
        self.population += 1;
    }

    /// Records counted so far.
    #[must_use]
    pub fn population(&self) -> u64 {
        self.population
    }

    /// The source: [`FrequencySource::None`] below
    /// [`MIN_FREQUENCY_POPULATION`], otherwise the counted table.
    #[must_use]
    pub fn finish(self) -> FrequencySource {
        if self.population < MIN_FREQUENCY_POPULATION {
            return FrequencySource::default();
        }
        let population = self.population;
        let table = FrequencyTable::from_counts(
            self.counts.into_iter().map(|((c, k), n)| (c, k, n)),
            population,
        );
        FrequencySource::Table { table, population }
    }
}

/// Scan up to [`FREQUENCY_SCAN_CAP`] active records, a page at a time,
/// and build their frequency source.
///
/// # Errors
///
/// Returns the repository's error if a page cannot be read.
pub async fn scan(repository: &dyn EventRepository) -> Result<FrequencySource> {
    let mut counter = Counter::default();
    let mut after = None;
    while counter.population() < FREQUENCY_SCAN_CAP {
        let want = FREQUENCY_SCAN_PAGE.min(FREQUENCY_SCAN_CAP - counter.population());
        let page = repository.list_active_names(want, after).await?;
        let Some(last) = page.last() else { break };
        after = Some(last.id);
        let short = (page.len() as u64) < want;
        for record in &page {
            let mut e = Event::new(record.name.clone(), chrono::DateTime::UNIX_EPOCH);
            e.alternate_names.clone_from(&record.alternate_names);
            counter.add(&e);
        }
        if short {
            break;
        }
    }
    Ok(counter.finish())
}

/// The cached frequency source for request-time matching.
pub struct FrequencyCache {
    /// Where the population is read from.
    repository: Arc<dyn EventRepository>,
    /// The current source and when it was built.
    current: RwLock<Option<(Instant, Arc<FrequencySource>)>>,
    /// Set while a rebuild runs, so only one runs at a time.
    rebuilding: AtomicBool,
    /// Age after which a rebuild is started.
    ttl: Duration,
}

impl FrequencyCache {
    /// An empty cache over `repository`, rebuilt every [`FREQUENCY_TTL`].
    #[must_use]
    pub fn new(repository: Arc<dyn EventRepository>) -> Self {
        Self {
            repository,
            current: RwLock::new(None),
            rebuilding: AtomicBool::new(false),
            ttl: FREQUENCY_TTL,
        }
    }

    /// The source to match with now. When there is none yet, or it is
    /// older than the TTL, a rebuild is started in the background (one at
    /// a time) and this call returns what it has, or
    /// [`FrequencySource::None`]. It never waits for a scan.
    pub fn current(self: &Arc<Self>) -> Arc<FrequencySource> {
        let snapshot = self
            .current
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|(at, s)| (*at, Arc::clone(s))));
        let stale = snapshot
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= self.ttl);
        if stale {
            self.spawn_rebuild();
        }
        snapshot.map_or_else(|| Arc::new(FrequencySource::default()), |(_, s)| s)
    }

    /// Start one background rebuild, unless one is running or there is
    /// no Tokio runtime to run it on.
    fn spawn_rebuild(self: &Arc<Self>) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        if self.rebuilding.swap(true, Ordering::AcqRel) {
            return;
        }
        let this = Arc::clone(self);
        handle.spawn(async move {
            if let Err(error) = this.rebuild().await {
                tracing::warn!(%error, "value-frequency table rebuild failed; keeping the last one");
            }
            this.rebuilding.store(false, Ordering::Release);
        });
    }

    /// Rebuild now: one scan, then store the result.
    ///
    /// # Errors
    ///
    /// Returns the repository's error; the previous source is kept.
    pub async fn rebuild(&self) -> Result<Arc<FrequencySource>> {
        let source = Arc::new(scan(self.repository.as_ref()).await?);
        if let Ok(mut g) = self.current.write() {
            *g = Some((Instant::now(), Arc::clone(&source)));
        }
        Ok(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_matcher::evidence::NAME;
    use event_matcher::{Event as MEvent, MatchingEngine};

    fn population(n: usize, common: &str, every: usize) -> Vec<Event> {
        (0..n)
            .map(|i| {
                let name = if i % every == 0 {
                    common.to_string()
                } else {
                    format!("Unique Event Number {i}")
                };
                Event::new(name, chrono::DateTime::UNIX_EPOCH)
            })
            .collect()
    }

    #[test]
    fn below_the_minimum_population_there_are_no_frequencies() {
        let events = population(999, "Annual General Meeting", 2);
        let source = FrequencySource::from_events(&events);
        assert!(source.is_none());
        assert_eq!(source.share("name", "annual general meeting"), None);
    }

    #[test]
    fn at_the_minimum_a_common_name_has_its_share() {
        let events = population(1_000, "Annual General Meeting", 10);
        let source = FrequencySource::from_events(&events);
        let FrequencySource::Table { population, .. } = &source else {
            panic!("expected a table");
        };
        assert_eq!(*population, 1_000);
        assert_eq!(source.share("name", "annual general meeting"), Some(0.1));
    }

    #[test]
    fn a_record_holding_a_name_twice_counts_once() {
        let mut events = population(1_000, "Open Mic Night", 1_000);
        events[0].alternate_names.push("OPEN MIC NIGHT!".into());
        let source = FrequencySource::from_events(&events);
        assert_eq!(source.share("name", "open mic night"), Some(0.001));
    }

    #[test]
    fn a_common_name_gives_fewer_bits_than_the_table_default() {
        let events = population(2_000, "Annual General Meeting", 5);
        let source = FrequencySource::from_events(&events);
        let e = MEvent::builder()
            .name("Annual General Meeting")
            .start_date("2031-03-14T09:00:00Z")
            .build();
        let engine = MatchingEngine::default_config();
        let plain = engine.match_events(&e, &e);
        let adjusted = engine.match_events_with_frequencies(&e, &e, &source);
        let bits = |r: &event_matcher::MatchResult| {
            r.evidence
                .components
                .iter()
                .find(|c| c.component == "name")
                .map(|c| (c.bits, c.frequency_adjusted))
                .unwrap()
        };
        let (plain_bits, plain_adjusted) = bits(&plain);
        let (adjusted_bits, adjusted_flag) = bits(&adjusted);
        assert!(!plain_adjusted && adjusted_flag);
        assert!((plain_bits - (NAME.m / NAME.u).log2()).abs() < 1e-9);
        assert!(
            adjusted_bits < plain_bits,
            "{adjusted_bits} vs {plain_bits}"
        );
    }
}
