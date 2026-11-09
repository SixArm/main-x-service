//! Match evidence and value frequencies for the service
//! (`agents/share/match-evidence.md`, spec §6 FR-MATCH-EVIDENCE).
//!
//! The service keeps its own weighted [`score`](super::scoring::MatchResult::score);
//! beside it, every scored pair carries the sibling `thing-matcher`
//! crate's [`Evidence`]: how much the data *proves*, in Fellegi–Sunter
//! bits, and whether any identifier scheme contradicts the pair. The
//! evidence is computed by projecting both service records through
//! [`to_matcher_thing`](super::adapter::to_matcher_thing) and calling
//! `MatchingEngine::match_things_with_frequencies` on the default matcher
//! configuration (so its `proof_bits` bar is the family default, 30).
//!
//! Auto-merge is gated on [`MatchStrength::Proven`] (§4 of the contract);
//! see [`permits_auto_merge`].
//!
//! # Value frequencies (contract §7)
//!
//! Agreeing on a common name ("USB Cable") proves less than agreeing on a
//! rare one. [`frequencies_from`] counts, over a population of active
//! records, the share holding each name key (the matcher's own
//! [`frequency_key`], over the primary name and every alternate name,
//! each record counted once per key). Below
//! [`MIN_FREQUENCY_POPULATION`] records the result is [`Frequencies::None`]
//! (a share from a small population is noise), so the plain `m`/`u`
//! table applies.
//!
//! [`FrequencyCache`] holds the table the create-time and
//! `check-duplicates` paths use: rebuilt when older than
//! [`FREQUENCY_MAX_AGE`] by paging the repository, at most
//! [`FREQUENCY_SCAN_CAP`] records (above the cap the table is a sample
//! of the most recently created records).
//!
//! # Masking
//!
//! Evidence holds component names and bits only, never field values. The
//! one caller-supplied string it can carry is an identifier **scheme**
//! label (`contradiction:identifier:<scheme>`), which names the kind of
//! identifier, not its value, so evidence is safe on masked paths.

use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant};

use thing_matcher::evidence::{FREQUENCY_COMPONENTS, frequency_key};
use thing_matcher::{FrequencyTable, MatchingEngine, NoFrequencies, ValueFrequencies};

pub use thing_matcher::{ComponentBits, Contradiction, Evidence, MatchStrength};

use crate::db::ThingRepository;
use crate::models::thing::Thing;

use super::adapter::to_matcher_thing;

/// Smallest population whose value shares are used. Below it a share is
/// noise (one record in fifty holding "Widget" says little about how
/// common "Widget" is), so the matcher's average `u` stands instead.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most active records one frequency rebuild scans. Above it the table
/// is a sample: the shares are counted over the most recently created
/// `FREQUENCY_SCAN_CAP` records, not the whole registry.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Page size of a frequency rebuild's repository scan.
pub const FREQUENCY_SCAN_PAGE: u64 = 1_000;

/// Age after which the cached frequency table is rebuilt.
pub const FREQUENCY_MAX_AGE: Duration = Duration::from_mins(10);

/// The engine evidence is computed with: the matcher's default
/// configuration. Built once; the engine is immutable and `Send + Sync`.
static ENGINE: LazyLock<MatchingEngine> = LazyLock::new(MatchingEngine::default_config);

/// A frequency source: either none (population too small) or a counted
/// table.
#[derive(Debug, Clone)]
pub enum Frequencies {
    /// No shares: the plain `m`/`u` table applies.
    None,
    /// Shares counted over at least [`MIN_FREQUENCY_POPULATION`] records.
    Table(FrequencyTable),
}

impl Frequencies {
    /// Whether this source carries counted shares.
    #[must_use]
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }
}

impl ValueFrequencies for Frequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None => NoFrequencies.share(component, value),
            Self::Table(table) => table.share(component, value),
        }
    }
}

/// The frequency keys one record holds under `component`, deduplicated so
/// a record counts once per key.
fn record_keys(thing: &Thing, component: &str) -> std::collections::BTreeSet<String> {
    std::iter::once(thing.name.as_str())
        .chain(thing.alternate_names.iter().map(String::as_str))
        .filter_map(|raw| frequency_key(component, raw))
        .collect()
}

/// Count value shares over `things`, the population. Below
/// [`MIN_FREQUENCY_POPULATION`] records, [`Frequencies::None`].
#[must_use]
pub fn frequencies_from(things: &[Thing]) -> Frequencies {
    let total = things.len() as u64;
    if total < MIN_FREQUENCY_POPULATION {
        return Frequencies::None;
    }
    let mut counts: std::collections::BTreeMap<(&str, String), u64> =
        std::collections::BTreeMap::new();
    for thing in things {
        for component in FREQUENCY_COMPONENTS {
            for key in record_keys(thing, component) {
                *counts.entry((component, key)).or_default() += 1;
            }
        }
    }
    Frequencies::Table(FrequencyTable::from_counts(
        counts.into_iter().map(|((c, k), n)| (c, k, n)),
        total,
    ))
}

/// The matcher's evidence for one pair of service records.
#[must_use]
pub fn evidence_for(a: &Thing, b: &Thing, frequencies: &dyn ValueFrequencies) -> Evidence {
    ENGINE
        .match_things_with_frequencies(&to_matcher_thing(a), &to_matcher_thing(b), frequencies)
        .evidence
}

/// Whether a pair may be marked or executed as an automatic merge: its
/// service score clears `auto_merge_threshold` **and** the evidence is
/// [`MatchStrength::Proven`] (match-evidence.md §4). `Consistent` and
/// `Contradicted` pairs go to review whatever their score.
#[must_use]
pub fn permits_auto_merge(score: f64, auto_merge_threshold: f64, evidence: &Evidence) -> bool {
    score >= auto_merge_threshold && evidence.strength == MatchStrength::Proven
}

/// Scan active records through `repo`, page by page, up to
/// [`FREQUENCY_SCAN_CAP`], and count their shares.
///
/// # Errors
///
/// Returns the repository's error if a page cannot be read.
pub async fn scan_frequencies(repo: &dyn ThingRepository) -> crate::Result<Frequencies> {
    let mut things = Vec::new();
    let mut offset = 0;
    while offset < FREQUENCY_SCAN_CAP {
        let limit = FREQUENCY_SCAN_PAGE.min(FREQUENCY_SCAN_CAP - offset);
        let page = repo.list(limit, offset).await?;
        let len = page.len() as u64;
        things.extend(page);
        if len < limit {
            break;
        }
        offset += len;
    }
    Ok(frequencies_from(&things))
}

/// The cached frequency table the create-time and `check-duplicates`
/// paths use.
///
/// Rebuilt **lazily on use**: a request finding no table builds one
/// inline (one bounded scan); a request finding a table older than
/// [`FREQUENCY_MAX_AGE`] keeps using it and starts one background
/// rebuild. At most one rebuild runs at a time, so no request waits on
/// more than one scan. A failed scan keeps the previous table (or none).
#[derive(Debug, Default)]
pub struct FrequencyCache {
    current: RwLock<Option<(Instant, Arc<Frequencies>)>>,
    rebuilding: tokio::sync::Mutex<()>,
}

impl FrequencyCache {
    /// An empty cache: the first use builds the table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The table currently held, with its build time.
    fn snapshot(&self) -> Option<(Instant, Arc<Frequencies>)> {
        self.current
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|(t, f)| (*t, Arc::clone(f))))
    }

    /// Replace the held table.
    fn store(&self, frequencies: Frequencies) -> Arc<Frequencies> {
        let frequencies = Arc::new(frequencies);
        if let Ok(mut guard) = self.current.write() {
            *guard = Some((Instant::now(), Arc::clone(&frequencies)));
        }
        frequencies
    }

    /// Run one scan, if no other is running, and store its result.
    async fn rebuild(&self, repo: &dyn ThingRepository) -> Option<Arc<Frequencies>> {
        let _guard = self.rebuilding.try_lock().ok()?;
        match scan_frequencies(repo).await {
            Ok(frequencies) => Some(self.store(frequencies)),
            Err(error) => {
                tracing::warn!(%error, "value-frequency rebuild failed; keeping the previous table");
                None
            }
        }
    }

    /// The frequencies to score with now.
    pub async fn current(self: &Arc<Self>, repo: &Arc<dyn ThingRepository>) -> Arc<Frequencies> {
        match self.snapshot() {
            Some((built, frequencies)) => {
                if built.elapsed() >= FREQUENCY_MAX_AGE {
                    let cache = Arc::clone(self);
                    let repo = Arc::clone(repo);
                    tokio::spawn(async move {
                        let _ = cache.rebuild(repo.as_ref()).await;
                    });
                }
                frequencies
            }
            None => match self.rebuild(repo.as_ref()).await {
                Some(frequencies) => frequencies,
                // Another request is building the first table, or the
                // scan failed: score with no frequencies this once.
                None => Arc::new(Frequencies::None),
            },
        }
    }

    /// Seed the cache with a ready table (tests, or a boot-time warm-up).
    pub fn set(&self, frequencies: Frequencies) {
        self.store(frequencies);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;
    use thing_matcher::evidence::NAME;

    fn named(name: &str) -> Thing {
        Thing::new(name)
    }

    #[test]
    fn below_the_minimum_population_there_are_no_frequencies() {
        let things: Vec<Thing> = (0..MIN_FREQUENCY_POPULATION - 1)
            .map(|i| named(&format!("Thing {i}")))
            .collect();
        assert!(!frequencies_from(&things).is_table());
    }

    #[test]
    fn a_common_name_gives_fewer_bits_than_the_table_default() {
        // 1_000 records, a tenth of them named "USB Cable".
        let things: Vec<Thing> = (0..MIN_FREQUENCY_POPULATION)
            .map(|i| {
                if i % 10 == 0 {
                    named("USB Cable")
                } else {
                    named(&format!("Unique Gadget {i}"))
                }
            })
            .collect();
        let frequencies = frequencies_from(&things);
        assert!(frequencies.is_table());
        let key = frequency_key("name", "USB Cable").unwrap();
        let share = frequencies.share("name", &key).unwrap();
        assert!((share - 0.1).abs() < 1e-9, "{share}");

        let a = named("USB Cable");
        let plain = evidence_for(&a, &a, &NoFrequencies);
        let adjusted = evidence_for(&a, &a, &frequencies);
        let table_default = (NAME.0 / NAME.1).log2();
        let name_bits = |e: &Evidence| {
            e.components
                .iter()
                .find(|c| c.component == "name")
                .map(|c| c.bits)
                .unwrap()
        };
        assert!((name_bits(&plain) - table_default).abs() < 1e-9);
        assert!(name_bits(&adjusted) < table_default);
        assert!(adjusted.weight_bits < plain.weight_bits);
    }

    #[test]
    fn a_record_counts_once_per_key() {
        let mut t = named("Widget");
        t.alternate_names = vec!["widget".into(), "WIDGET ".into()];
        assert_eq!(record_keys(&t, "name").len(), 1);
    }

    #[test]
    fn a_name_only_pair_is_consistent_not_proven() {
        let a = named("Pride and Prejudice");
        let evidence = evidence_for(&a, &a, &NoFrequencies);
        assert_eq!(evidence.strength, MatchStrength::Consistent);
        assert!(!permits_auto_merge(1.0, 0.95, &evidence));
    }

    #[test]
    fn a_shared_identifier_is_proven() {
        let mut a = named("Pride and Prejudice");
        a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let mut b = named("Stolz und Vorurteil");
        b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let evidence = evidence_for(&a, &b, &NoFrequencies);
        assert_eq!(evidence.strength, MatchStrength::Proven);
        assert!(permits_auto_merge(1.0, 0.95, &evidence));
        assert!(!permits_auto_merge(0.9, 0.95, &evidence));
    }

    #[test]
    fn conflicting_identifiers_are_contradicted() {
        let mut a = named("Pride and Prejudice");
        a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let mut b = named("Pride and Prejudice");
        b.identifiers = vec![ThingIdentifier::isbn("9781503290563")];
        let evidence = evidence_for(&a, &b, &NoFrequencies);
        assert_eq!(evidence.strength, MatchStrength::Contradicted);
        assert_eq!(evidence.contradictions[0].field, "identifier:isbn");
        assert!(!permits_auto_merge(1.0, 0.0, &evidence));
    }

    #[test]
    fn evidence_holds_no_field_values() {
        let mut a = named("Secret Name");
        a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        a.description = Some("confidential text".into());
        let mut b = a.clone();
        b.identifiers = vec![ThingIdentifier::isbn("9781503290563")];
        let json = serde_json::to_string(&evidence_for(&a, &b, &NoFrequencies)).unwrap();
        for value in [
            "Secret",
            "secret",
            "confidential",
            "9780141439518",
            "9781503290563",
        ] {
            assert!(!json.contains(value), "{json}");
        }
    }
}
