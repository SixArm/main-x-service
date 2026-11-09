//! Value frequencies for the matcher's evidence
//! (`agents/share/match-evidence.md` §7).
//!
//! Agreeing on a common organization name ("City Council", "ABC Ltd")
//! proves much less than agreeing on a rare one. The matcher can weigh
//! an exact agreement by the value's own share of records, but only the
//! service sees the population, so the service counts it here.
//!
//! - **What is counted.** For each component in the matcher's
//!   [`FREQUENCY_COMPONENTS`] (for this crate, `name`), each active
//!   record contributes each distinct [`frequency_key`] it holds once.
//!   For `name` that is the legal-suffix-stripped form of `name`,
//!   `legal_name` and every `alternate_names` entry — the same keys the
//!   matcher compares and looks up. A share is `count / population`.
//! - **Small populations.** Below [`MIN_FREQUENCY_POPULATION`] records
//!   the source is [`Frequencies::None`], which behaves exactly like the
//!   matcher's `NoFrequencies`: a share counted over a few hundred
//!   records is noise.
//! - **Batch `deduplicate`** builds a table from the records the scan
//!   loaded ([`from_records`]).
//! - **Create-time, `check-duplicates` and bulk import** use a
//!   process-wide cached table ([`current`]). It is process-wide rather
//!   than on `AppContext` because the bulk worker holds only a
//!   `PgPool`, and it sits beside the search index (also a process
//!   global, `crate::search::engine()`). It is rebuilt lazily on use:
//!   the first call builds it inline (one bounded scan; concurrent first
//!   callers wait for that one scan rather than each running their
//!   own), and a call that finds it older than [`FREQUENCY_MAX_AGE`]
//!   keeps answering from the old table while one background task
//!   rebuilds it. So no request ever waits longer than one scan.
//! - **Bound.** The scan pages through the repository and stops at
//!   [`FREQUENCY_SCAN_CAP`] records. Above the cap the table is a
//!   sample (the oldest records), not a census.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use organization_matcher::evidence::FREQUENCY_COMPONENTS;
use organization_matcher::{FrequencyTable, Organization, ValueFrequencies, frequency_key};
use sqlx::PgPool;

use crate::models::organizations::Model as OrgModel;

/// Smallest population a share is counted from. Below it the source is
/// [`Frequencies::None`], because a share from a small population is
/// noise: in a registry of 200 organizations, every name is either
/// "held by one record" or "held by two", and neither says how common
/// the name is in the world.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most records the cached table scans. Above it the table is a sample
/// of the oldest records rather than a count over the whole registry,
/// which bounds one rebuild's time and memory.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Age after which the cached table is rebuilt (in the background,
/// while the old one keeps answering).
pub const FREQUENCY_MAX_AGE: Duration = Duration::from_mins(10);

/// Page size of the rebuild scan.
const SCAN_PAGE: u64 = 1_000;

/// A frequency source: none (too small a population, or not built), or
/// a counted table.
#[derive(Debug, Clone, Default)]
pub enum Frequencies {
    /// No shares; the matcher's plain §2 table applies.
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
    /// True when this source supplies no shares.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl ValueFrequencies for Frequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None => None,
            Self::Table { table, .. } => table.share(component, value),
        }
    }
}

/// The raw values `org` holds under a frequency `component`.
fn raw_values<'a>(component: &str, org: &'a Organization) -> Vec<&'a str> {
    match component {
        "name" => std::iter::once(org.name.as_str())
            .chain(org.legal_name.as_deref())
            .chain(org.alternate_names.iter().map(String::as_str))
            .collect(),
        _ => Vec::new(),
    }
}

/// Count the keys of one record into `counts`, each distinct key once
/// per record and component.
fn count_record(
    org: &Organization,
    counts: &mut std::collections::BTreeMap<(&'static str, String), u64>,
) {
    for &component in FREQUENCY_COMPONENTS {
        let mut keys: Vec<String> = raw_values(component, org)
            .into_iter()
            .filter_map(|raw| frequency_key(component, raw))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            *counts.entry((component, key)).or_default() += 1;
        }
    }
}

/// Build a frequency source from a population of records: a counted
/// table at or above [`MIN_FREQUENCY_POPULATION`] records, else
/// [`Frequencies::None`].
pub fn from_records<'a>(orgs: impl IntoIterator<Item = &'a Organization>) -> Frequencies {
    let mut counts = std::collections::BTreeMap::new();
    let mut population: u64 = 0;
    for org in orgs {
        population += 1;
        count_record(org, &mut counts);
    }
    if population < MIN_FREQUENCY_POPULATION {
        return Frequencies::None;
    }
    let table =
        FrequencyTable::from_counts(counts.into_iter().map(|((c, v), n)| (c, v, n)), population);
    Frequencies::Table { table, population }
}

/// Scan active records (oldest first, paged, up to
/// [`FREQUENCY_SCAN_CAP`]) and count them.
async fn scan(db: &PgPool) -> crate::models::ModelResult<Frequencies> {
    let mut orgs = Vec::new();
    let mut offset = 0;
    while offset < FREQUENCY_SCAN_CAP {
        let page = SCAN_PAGE.min(FREQUENCY_SCAN_CAP - offset);
        let rows = OrgModel::list_active_oldest_first(db, page, offset).await?;
        let fetched = rows.len() as u64;
        // A row whose payload no longer parses is skipped, not fatal: it
        // would be skipped by every matching path too.
        orgs.extend(rows.iter().filter_map(|r| r.to_org().ok()));
        if fetched < page {
            break;
        }
        offset += fetched;
    }
    if offset >= FREQUENCY_SCAN_CAP {
        tracing::info!(
            cap = FREQUENCY_SCAN_CAP,
            "value-frequency scan hit its cap; the table is a sample"
        );
    }
    Ok(from_records(&orgs))
}

/// The cached table and when it was built.
struct Cached {
    built: Instant,
    freqs: Arc<Frequencies>,
}

static CACHE: RwLock<Option<Cached>> = RwLock::new(None);
static FIRST_BUILD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static REFRESHING: AtomicBool = AtomicBool::new(false);

/// The cached table and whether it is still fresh.
fn peek() -> Option<(Arc<Frequencies>, bool)> {
    let guard = CACHE.read().unwrap_or_else(PoisonError::into_inner);
    guard
        .as_ref()
        .map(|c| (Arc::clone(&c.freqs), c.built.elapsed() < FREQUENCY_MAX_AGE))
}

fn store(freqs: Arc<Frequencies>) {
    let mut guard = CACHE.write().unwrap_or_else(PoisonError::into_inner);
    *guard = Some(Cached {
        built: Instant::now(),
        freqs,
    });
}

/// Start one background rebuild unless one is already running.
fn spawn_refresh(db: PgPool) {
    if REFRESHING.swap(true, Ordering::AcqRel) {
        return;
    }
    tokio::spawn(async move {
        match scan(&db).await {
            Ok(freqs) => store(Arc::new(freqs)),
            Err(e) => tracing::warn!("value-frequency rebuild failed; keeping the old table: {e}"),
        }
        REFRESHING.store(false, Ordering::Release);
    });
}

/// The cached frequency source for create-time duplicate detection,
/// `check-duplicates` and bulk-import duplicate detection.
///
/// A fresh table is returned as is. A stale one is returned while a
/// background rebuild runs. With none built yet, the first caller scans
/// inline and the rest wait for that scan. A failed first scan answers
/// [`Frequencies::None`] (logged) and is retried on the next call.
pub async fn current(db: &PgPool) -> Arc<Frequencies> {
    if let Some((freqs, fresh)) = peek() {
        if !fresh {
            spawn_refresh(db.clone());
        }
        return freqs;
    }
    let _first = FIRST_BUILD.lock().await;
    if let Some((freqs, _)) = peek() {
        return freqs;
    }
    match scan(db).await {
        Ok(freqs) => {
            let freqs = Arc::new(freqs);
            store(Arc::clone(&freqs));
            freqs
        }
        Err(e) => {
            tracing::warn!("value-frequency scan failed; matching without frequencies: {e}");
            Arc::new(Frequencies::None)
        }
    }
}

/// Unit tests: the population floor, the counting rule, and the effect
/// on the matcher's evidence.
#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{MatchingEngine, NoFrequencies};

    /// `n` records with distinct names, plus `common` records named
    /// "City Council".
    fn population(n: usize, common: usize) -> Vec<Organization> {
        let mut orgs: Vec<Organization> = (0..n)
            .map(|i| Organization::new(format!("Unique Org Number {i}")))
            .collect();
        orgs.extend((0..common).map(|_| Organization::new("City Council")));
        orgs
    }

    #[test]
    fn below_the_floor_is_no_frequencies() {
        let orgs = population(500, 400);
        assert_eq!(orgs.len() as u64, 900);
        assert!(from_records(&orgs).is_none());
        let orgs = population(0, 0);
        assert!(from_records(&orgs).is_none());
    }

    #[test]
    fn at_the_floor_a_table_is_counted() {
        let orgs = population(800, 200);
        let freqs = from_records(&orgs);
        let Frequencies::Table { population, .. } = &freqs else {
            panic!("expected a table");
        };
        assert_eq!(*population, MIN_FREQUENCY_POPULATION);
        let key = frequency_key("name", "City Council").unwrap();
        let share = freqs.share("name", &key).unwrap();
        assert!((share - 0.2).abs() < 1e-12);
    }

    #[test]
    fn a_record_counts_each_key_once() {
        // name and legal_name normalise to the same key: one count.
        let mut org = Organization::new("Acme");
        org.legal_name = Some("Acme Ltd".into());
        org.alternate_names = vec!["Acme Inc.".into(), "Acme Widgets".into()];
        let mut orgs = population(999, 0);
        orgs.push(org);
        let freqs = from_records(&orgs);
        let acme = frequency_key("name", "Acme").unwrap();
        let widgets = frequency_key("name", "Acme Widgets").unwrap();
        assert!((freqs.share("name", &acme).unwrap() - 1e-3).abs() < 1e-12);
        assert!((freqs.share("name", &widgets).unwrap() - 1e-3).abs() < 1e-12);
    }

    #[test]
    fn a_common_name_gives_fewer_bits_than_the_table_default() {
        let orgs = population(800, 200);
        let freqs = from_records(&orgs);
        let engine = MatchingEngine::default_config();
        let a = Organization::new("City Council");
        let b = Organization::new("City Council");
        let with = engine.match_organizations_with_frequencies(&a, &b, &freqs);
        let plain = engine.match_organizations_with_frequencies(&a, &b, &NoFrequencies);
        assert!(with.evidence.weight_bits < plain.evidence.weight_bits);
        assert!((with.score - plain.score).abs() < f64::EPSILON);
        assert!(
            with.evidence
                .components
                .iter()
                .any(|c| c.frequency_adjusted)
        );
    }

    #[test]
    fn no_frequencies_variant_equals_the_plain_match() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("City Council");
        let with = engine.match_organizations_with_frequencies(&a, &a, &Frequencies::None);
        let plain = engine.match_organizations(&a, &a);
        assert_eq!(with.evidence, plain.evidence);
    }
}
