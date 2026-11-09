//! Value frequencies for the matcher's evidence (spec §6.1,
//! `agents/share/match-evidence.md` §7).
//!
//! Agreeing on a common course name ("Introduction to Programming") is
//! weaker evidence than the matcher's average `u` assumes, and agreeing
//! on a rare one stronger. Only the service sees the population, so it
//! counts: each active course is mapped through the adapter, keyed with
//! the matcher's own `frequency_key` for each of `FREQUENCY_COMPONENTS`
//! (today just `name`), and every share is `count / population`.
//!
//! - Below [`MIN_FREQUENCY_POPULATION`] records the source is empty
//!   ([`CourseFrequencies::none`]): a share counted from a few hundred
//!   records is noise, and the matcher's table then stands.
//! - A scan reads at most [`FREQUENCY_SCAN_CAP`] records. Above the cap
//!   the table is a sample of the newest records (the repository lists
//!   newest first), not a census.
//! - [`FrequencyCache`] holds the table for the create-time and
//!   `check-duplicates` paths and rebuilds it once it is older than
//!   [`FREQUENCY_MAX_AGE`], lazily on use: a stale table keeps serving
//!   while one background task rebuilds it, so only the very first
//!   request after boot ever waits, and then for one bounded scan.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use course_matcher::ValueFrequencies;
use course_matcher::evidence::{FREQUENCY_COMPONENTS, FrequencyTable, frequency_key};

use crate::db::CourseRepository;
use crate::matching::adapter;
use crate::models::Course;

/// Smallest population a share is counted from. Below it the source
/// supplies nothing and the matcher's `m`/`u` table stands: in a few
/// hundred records one repeated title is already a "1 %" value, which
/// says more about the sample than about the name.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most records one frequency scan reads. Above it the table is a sample
/// of the newest records rather than the whole population; a sample of
/// this size still ranks common titles correctly, and the cap keeps a
/// rebuild bounded in time and memory.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Age after which the cached table is rebuilt.
pub const FREQUENCY_MAX_AGE: Duration = Duration::from_mins(10);

/// Repository page size for a frequency scan.
const FREQUENCY_SCAN_PAGE: u64 = 500;

/// A frequency source counted from this service's own records.
#[derive(Debug, Clone, Default)]
pub struct CourseFrequencies {
    /// The shares, or `None` below [`MIN_FREQUENCY_POPULATION`].
    table: Option<FrequencyTable>,
    /// Records the shares were counted over.
    population: u64,
}

impl CourseFrequencies {
    /// No frequencies: every share is unknown (the matcher's
    /// `NoFrequencies`).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// `true` when the population was large enough to supply shares.
    #[must_use]
    pub fn is_counted(&self) -> bool {
        self.table.is_some()
    }

    /// Records the source was counted over.
    #[must_use]
    pub fn population(&self) -> u64 {
        self.population
    }

    /// Count `courses` in one pass.
    pub fn from_courses<'a>(courses: impl IntoIterator<Item = &'a Course>) -> Self {
        let mut counter = FrequencyCounter::default();
        for c in courses {
            counter.add(c);
        }
        counter.finish()
    }

    /// Page through the repository's active courses, at most
    /// [`FREQUENCY_SCAN_CAP`] of them, and count.
    ///
    /// # Errors
    ///
    /// Returns the repository's error if a page fails to load.
    pub async fn scan(repo: &dyn CourseRepository) -> crate::Result<Self> {
        let mut counter = FrequencyCounter::default();
        let mut offset = 0;
        while offset < FREQUENCY_SCAN_CAP {
            let size = FREQUENCY_SCAN_PAGE.min(FREQUENCY_SCAN_CAP - offset);
            let page = repo.list(size, offset).await?;
            let n = page.len() as u64;
            for c in &page {
                counter.add(c);
            }
            offset += n;
            if n < size {
                break;
            }
        }
        Ok(counter.finish())
    }
}

impl ValueFrequencies for CourseFrequencies {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.table.as_ref()?.share(component, value)
    }
}

/// Accumulates value counts across a scan.
#[derive(Debug, Default)]
pub struct FrequencyCounter {
    /// `(component, key)` → records holding it.
    counts: HashMap<(&'static str, String), u64>,
    /// Records added.
    population: u64,
}

impl FrequencyCounter {
    /// Count one course's values, in the matcher's key form.
    pub fn add(&mut self, course: &Course) {
        let m = adapter::to_matcher_course(course);
        self.population += 1;
        for &component in FREQUENCY_COMPONENTS {
            let raw = match component {
                "name" => m.name.as_str(),
                // A component this service does not route has no value.
                _ => continue,
            };
            if let Some(key) = frequency_key(component, raw) {
                *self.counts.entry((component, key)).or_insert(0) += 1;
            }
        }
    }

    /// Turn the counts into shares, or nothing below
    /// [`MIN_FREQUENCY_POPULATION`].
    #[must_use]
    pub fn finish(self) -> CourseFrequencies {
        if self.population < MIN_FREQUENCY_POPULATION {
            return CourseFrequencies {
                table: None,
                population: self.population,
            };
        }
        let table = FrequencyTable::from_counts(
            self.counts.into_iter().map(|((c, k), n)| (c, k, n)),
            self.population,
        );
        CourseFrequencies {
            table: Some(table),
            population: self.population,
        }
    }
}

/// One cached table and when it was built.
#[derive(Debug, Clone)]
struct Cached {
    /// When the scan finished.
    built_at: Instant,
    /// The table.
    frequencies: Arc<CourseFrequencies>,
}

/// The create-time / `check-duplicates` frequency table, rebuilt lazily
/// once older than [`FREQUENCY_MAX_AGE`].
#[derive(Debug, Default)]
pub struct FrequencyCache {
    /// The last table built, if any.
    cached: Mutex<Option<Cached>>,
    /// Set while a rebuild runs, so only one runs at a time.
    rebuilding: AtomicBool,
}

impl FrequencyCache {
    /// An empty cache; the first use builds the table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The last table built, or `None`.
    fn snapshot(&self) -> Option<Cached> {
        self.cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Replace the cached table.
    fn store(&self, frequencies: CourseFrequencies) -> Arc<CourseFrequencies> {
        let frequencies = Arc::new(frequencies);
        *self
            .cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Cached {
            built_at: Instant::now(),
            frequencies: Arc::clone(&frequencies),
        });
        frequencies
    }

    /// Scan and store; a failed scan stores an empty table, so a broken
    /// database is retried after [`FREQUENCY_MAX_AGE`] rather than on
    /// every request.
    async fn rebuild(&self, repo: &dyn CourseRepository) -> Arc<CourseFrequencies> {
        let built = match CourseFrequencies::scan(repo).await {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("value-frequency scan failed; matching without frequencies: {e}");
                CourseFrequencies::none()
            }
        };
        let out = self.store(built);
        self.rebuilding.store(false, Ordering::Release);
        out
    }

    /// The table to match with now.
    ///
    /// Fresh: returned as is. Stale: returned as is while one background
    /// task rebuilds it. None yet: this request builds it (one bounded
    /// scan), unless another request already is, in which case it
    /// matches without frequencies rather than wait.
    pub async fn current(
        self: &Arc<Self>,
        repo: &Arc<dyn CourseRepository>,
    ) -> Arc<CourseFrequencies> {
        let snapshot = self.snapshot();
        if let Some(c) = &snapshot
            && c.built_at.elapsed() < FREQUENCY_MAX_AGE
        {
            return Arc::clone(&c.frequencies);
        }
        let claimed = self
            .rebuilding
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        match snapshot {
            Some(stale) => {
                if claimed {
                    let cache = Arc::clone(self);
                    let repo = Arc::clone(repo);
                    tokio::spawn(async move {
                        cache.rebuild(repo.as_ref()).await;
                    });
                }
                stale.frequencies
            }
            None if claimed => self.rebuild(repo.as_ref()).await,
            None => Arc::new(CourseFrequencies::none()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MatchingConfig;
    use crate::matching::CourseMatcher;

    /// A population of `n` courses, `common` of them named
    /// "Introduction to Programming" and the rest uniquely named.
    fn population(n: usize, common: usize) -> Vec<Course> {
        (0..n)
            .map(|i| {
                if i < common {
                    Course::new("Introduction to Programming")
                } else {
                    Course::new(format!("Rare course number {i}"))
                }
            })
            .collect()
    }

    /// Below the minimum population the source supplies nothing.
    #[test]
    fn below_minimum_population_is_no_frequencies() {
        let courses = population(usize::try_from(MIN_FREQUENCY_POPULATION).unwrap() - 1, 500);
        let f = CourseFrequencies::from_courses(&courses);
        assert!(!f.is_counted());
        assert_eq!(f.population(), MIN_FREQUENCY_POPULATION - 1);
        assert_eq!(f.share("name", "introduction to programming"), None);
    }

    /// At the minimum, shares are `count / population` in the matcher's
    /// key form.
    #[test]
    fn at_minimum_population_counts_shares() {
        let courses = population(2_000, 500);
        let f = CourseFrequencies::from_courses(&courses);
        assert!(f.is_counted());
        let share = f.share("name", "introduction to programming").unwrap();
        assert!((share - 0.25).abs() < 1e-12, "{share}");
    }

    /// A common name gives fewer evidence bits than the table default;
    /// the score is unchanged.
    #[test]
    fn common_value_gives_fewer_bits_than_the_default() {
        let courses = population(2_000, 500);
        let f = CourseFrequencies::from_courses(&courses);
        let m = CourseMatcher::new(MatchingConfig {
            threshold_score: 0.85,
        });
        let a = Course::new("Introduction to Programming");
        let b = Course::new("Introduction to Programming");
        let plain = m.match_courses(&a, &b);
        let adjusted = m.match_courses_with_frequencies(&a, &b, &f);
        assert!((plain.score - adjusted.score).abs() < 1e-12);
        assert!(
            adjusted.evidence.weight_bits < plain.evidence.weight_bits,
            "{} !< {}",
            adjusted.evidence.weight_bits,
            plain.evidence.weight_bits
        );
        assert!(
            adjusted
                .evidence
                .components
                .iter()
                .any(|c| c.component == "name" && c.frequency_adjusted)
        );
    }
}
