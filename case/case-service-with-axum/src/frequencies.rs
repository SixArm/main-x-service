//! Value frequencies for the matcher's evidence
//! (`agents/share/match-evidence.md` §7).
//!
//! The matcher weighs an exact agreement on a
//! [`case_matcher::evidence::FREQUENCY_COMPONENTS`] value (for case, the
//! `title`) by that value's own share of records instead of the table's
//! average `u`: agreeing on a template title such as "Housing Benefit
//! Claim" proves little, agreeing on a one-off title proves much more.
//! Only the service sees the population, so the counting lives here.
//!
//! Two sources:
//!
//! - **Batch `deduplicate`** builds one from the records its scan loaded
//!   ([`CaseFrequencies::from_cases`]).
//! - **`check-duplicates` and the bulk-import duplicate check** use a
//!   process-wide cached table ([`current`]), built by paging through the
//!   active cases and rebuilt when older than [`FREQUENCY_TTL`]. The
//!   rebuild is **lazy, on use**: the first caller after boot builds it
//!   inline (one bounded scan); a caller who finds it stale gets the stale
//!   table at once and starts one background rebuild, so no request ever
//!   waits on a rebuild after the first.
//!
//! Below [`MIN_FREQUENCY_POPULATION`] records the source is empty, which
//! is exactly the matcher's `NoFrequencies`: a share counted from a small
//! population is noise, and the plain `m`/`u` table is the better guess.

use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use case_matcher::evidence::FREQUENCY_COMPONENTS;
use case_matcher::{Case, FrequencyTable, ValueFrequencies, frequency_key};
use sqlx::PgPool;

use crate::models::cases::Model as CaseModel;

/// Smallest population a share is counted from. Below it every share is
/// unknown (`NoFrequencies`): with fewer than a thousand cases, one
/// title's count moves its share by a tenth of a percent or more, so the
/// "frequency" is mostly which cases happened to be filed so far.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most active cases the cached table counts. Above it the table is a
/// **sample**: the most recent `FREQUENCY_SCAN_CAP` cases (by row id), so
/// a share is the share among those, not the whole registry. Bounds the
/// one scan a rebuild costs.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Page size of the cached table's scan.
pub const FREQUENCY_SCAN_PAGE: u64 = 1_000;

/// Age after which the cached table is rebuilt.
pub const FREQUENCY_TTL: Duration = Duration::from_mins(10);

/// A frequency source over one counted population: the table when the
/// population is at least [`MIN_FREQUENCY_POPULATION`], nothing (every
/// share `None`, the matcher's `NoFrequencies`) otherwise.
#[derive(Debug, Clone, Default)]
pub struct CaseFrequencies {
    /// The counted table, or `None` below the population floor.
    table: Option<FrequencyTable>,
    /// How many records were counted.
    population: u64,
}

impl CaseFrequencies {
    /// The source that knows nothing (`NoFrequencies`).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Count `cases`' [`FREQUENCY_COMPONENTS`] values in the matcher's own
    /// key form ([`frequency_key`]) and divide by the population.
    #[must_use]
    pub fn from_cases<'a>(cases: impl IntoIterator<Item = &'a Case>) -> Self {
        let mut counter = Counter::default();
        for case in cases {
            counter.add(case);
        }
        counter.finish()
    }

    /// Whether a table is in use (the population cleared the floor).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.table.is_some()
    }

    /// How many records were counted.
    #[must_use]
    pub fn population(&self) -> u64 {
        self.population
    }
}

impl ValueFrequencies for CaseFrequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.table.as_ref()?.share(component, value)
    }
}

/// The raw value a frequency component reads from a case.
fn raw_value<'a>(component: &str, case: &'a Case) -> Option<&'a str> {
    match component {
        "title" => Some(case.title.as_str()),
        _ => None,
    }
}

/// Running counts, fed one case at a time so a paged scan never holds
/// more than one page of records.
#[derive(Debug, Default)]
struct Counter {
    /// `(component, key)` → count.
    counts: std::collections::HashMap<(&'static str, String), u64>,
    /// Records counted.
    total: u64,
}

impl Counter {
    fn add(&mut self, case: &Case) {
        self.total += 1;
        for &component in FREQUENCY_COMPONENTS {
            let Some(key) = raw_value(component, case).and_then(|r| frequency_key(component, r))
            else {
                continue;
            };
            *self.counts.entry((component, key)).or_default() += 1;
        }
    }

    fn finish(self) -> CaseFrequencies {
        if self.total < MIN_FREQUENCY_POPULATION {
            return CaseFrequencies {
                table: None,
                population: self.total,
            };
        }
        let triples: Vec<(&str, &str, u64)> = self
            .counts
            .iter()
            .map(|((c, k), n)| (*c, k.as_str(), *n))
            .collect();
        CaseFrequencies {
            table: Some(FrequencyTable::from_counts(&triples, self.total)),
            population: self.total,
        }
    }
}

/// Build the table from the active cases, paging through the repository
/// and stopping at [`FREQUENCY_SCAN_CAP`]. A row whose payload will not
/// parse is skipped, not counted.
///
/// # Errors
///
/// When a page query fails.
pub async fn scan(db: &PgPool) -> crate::models::ModelResult<CaseFrequencies> {
    let mut counter = Counter::default();
    let mut offset = 0;
    while offset < FREQUENCY_SCAN_CAP {
        let page = FREQUENCY_SCAN_PAGE.min(FREQUENCY_SCAN_CAP - offset);
        let rows = CaseModel::list_paged(db, page, offset).await?;
        let n = rows.len() as u64;
        for row in &rows {
            if let Ok(case) = row.to_case() {
                counter.add(&case);
            }
        }
        if n < page {
            break;
        }
        offset += n;
    }
    Ok(counter.finish())
}

/// The cached table and when it was built.
type Cached = Option<(Instant, Arc<CaseFrequencies>)>;

fn cache() -> &'static RwLock<Cached> {
    static CACHE: OnceLock<RwLock<Cached>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(None))
}

/// Serialises rebuilds, so at most one scan runs at a time.
fn rebuilding() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn read_cache() -> Cached {
    cache().read().map_or(None, |c| c.clone())
}

fn store(freqs: CaseFrequencies) -> Arc<CaseFrequencies> {
    let freqs = Arc::new(freqs);
    if let Ok(mut c) = cache().write() {
        *c = Some((Instant::now(), Arc::clone(&freqs)));
    }
    freqs
}

/// The process-wide cached frequency source for request-time matching.
///
/// Fresh ⇒ returned as is. Stale ⇒ returned as is, and one background
/// rebuild is started unless one is already running. Absent (first use)
/// ⇒ built inline, one bounded scan; a caller arriving while another
/// builds it waits for that one build rather than starting a second. A
/// failed scan is logged and gives [`CaseFrequencies::none`], so a
/// frequency problem never fails a duplicate check: it only falls back
/// to the plain `m`/`u` table.
pub async fn current(db: &PgPool) -> Arc<CaseFrequencies> {
    if let Some((built, freqs)) = read_cache() {
        if built.elapsed() >= FREQUENCY_TTL
            && let Ok(guard) = rebuilding().try_lock()
        {
            let db = db.clone();
            tokio::spawn(async move {
                let _guard = guard;
                match scan(&db).await {
                    Ok(f) => {
                        store(f);
                    }
                    Err(err) => tracing::warn!("frequency table rebuild failed: {err}"),
                }
            });
        }
        return freqs;
    }
    let _guard = rebuilding().lock().await;
    if let Some((_, freqs)) = read_cache() {
        return freqs; // built by the caller we waited for
    }
    match scan(db).await {
        Ok(f) => store(f),
        Err(err) => {
            tracing::warn!("frequency table build failed: {err}");
            Arc::new(CaseFrequencies::none())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use case_matcher::evidence::{TITLE_M_U, frequency_bits};

    fn population(n: u64, common_every: u64) -> Vec<Case> {
        (0..n)
            .map(|i| {
                if i % common_every == 0 {
                    Case::new("Housing Benefit Claim")
                } else {
                    Case::new(format!("Unique matter {i}"))
                }
            })
            .collect()
    }

    #[test]
    fn below_the_floor_it_is_no_frequencies() {
        let cases = population(MIN_FREQUENCY_POPULATION - 1, 2);
        let f = CaseFrequencies::from_cases(&cases);
        assert!(!f.is_active());
        assert_eq!(f.population(), MIN_FREQUENCY_POPULATION - 1);
        let key = frequency_key("title", "Housing Benefit Claim").expect("key");
        assert!(f.share("title", &key).is_none());
    }

    #[test]
    fn above_the_floor_a_common_title_gives_fewer_bits_than_the_default() {
        // Every tenth case carries the template title: a 10% share.
        let cases = population(2_000, 10);
        let freqs = CaseFrequencies::from_cases(&cases);
        assert!(freqs.is_active());
        let key = frequency_key("title", "Housing Benefit Claim").expect("key");
        let share = freqs.share("title", &key).expect("counted");
        assert!((share - 0.1).abs() < 1e-12, "share {share}");
        let (m_title, u_title) = TITLE_M_U;
        assert!(frequency_bits(m_title, share) < (m_title / u_title).log2());

        // And the matcher uses it: two cases on the common title weigh
        // less than with no frequencies.
        let engine = case_matcher::MatchingEngine::default_config();
        let first = Case::new("Housing Benefit Claim");
        let second = Case::new("housing benefit claim");
        let plain = engine.match_cases(&first, &second).evidence.weight_bits;
        let adjusted = engine
            .match_cases_with_frequencies(&first, &second, &freqs)
            .evidence
            .weight_bits;
        assert!(adjusted < plain, "{adjusted} vs {plain}");
    }

    #[test]
    fn the_counted_key_is_the_matchers_key() {
        let cases = population(1_000, 1);
        let f = CaseFrequencies::from_cases(&cases);
        // Case and spacing differ; the matcher's normalised key is shared.
        let key = frequency_key("title", "  HOUSING benefit CLAIM ").expect("key");
        assert!((f.share("title", &key).expect("counted") - 1.0).abs() < 1e-12);
    }
}
