//! Match evidence: whether the data **proves** a pair is one person, beside
//! the service's similarity score (`agents/share/match-evidence.md`).
//!
//! The service keeps scoring pairs with its own renormalising scorer
//! ([`super::scoring`]), and that score still ranks candidates and fills
//! the review queue. A score cannot tell "nothing here contradicts a
//! match" from "the data proves a match": two people sharing a name and a
//! birth date score `1.0`, and so do twins. A merge deletes a record, so
//! the decision to merge without a human is taken on evidence instead.
//!
//! Evidence comes from the canonical `person-matcher` engine, reached
//! through [`super::adapter::to_matcher_person`]: [`pair_evidence`] maps
//! both records, calls `match_persons_with_frequencies`, and returns the
//! engine's [`Evidence`] as the wire DTO [`MatchEvidence`]. Only the
//! evidence is taken from the engine; the engine's own score is not used.
//!
//! # What evidence discloses
//!
//! A [`MatchEvidence`] holds component **names** (`"family_name"`,
//! `"identifier"`, `"contradiction:identifier:united_kingdom_..."`), bits,
//! a count and a verdict. It never holds a field value: not a name, not a
//! date, not an identifier, not even a masked one. What it does say is
//! which fields both records held and which identifier scheme conflicts.
//! The score breakdown beside it already says as much (a non-zero
//! `identifier_score` or `birth_date_score`), so evidence adds no new
//! category of disclosure to any path that returns the breakdown.
//!
//! # The auto-merge gate
//!
//! [`auto_merge_permitted`] is the one rule: a pair may be flagged or
//! executed as an automatic merge only when its score clears the
//! threshold **and** its strength is
//! [`EvidenceStrength::Proven`]. `consistent` and `contradicted` pairs
//! stay `pending` for a person to decide, whatever their score (§4).
//!
//! # Value frequencies (§7)
//!
//! Agreeing on "Smith" proves less than agreeing on "Zbigniewska". The
//! matcher can weigh given name, family name and birth date by the
//! value's own share of records, and only the service can count those
//! shares. [`ValueFrequencySource`] is that count; [`FrequencyCache`]
//! keeps one for the interactive paths, rebuilt in the background.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use person_matcher::evidence::{
    FREQUENCY_COMPONENTS, FrequencyTable, NoFrequencies, ValueFrequencies, frequency_key,
};
use person_matcher::{Evidence, MatchConfig, MatchStrength, MatchingEngine};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use super::adapter::to_matcher_person;
use crate::models::{Gender, HumanName, Person};

/// Population below which no frequency table is supplied.
///
/// A share is `count / population`. In a registry of a few hundred
/// records every value is "rare" (a share of 1/300 gives more bits than
/// the table's own `u` for a given name), so a small population would
/// *strengthen* agreement on common names rather than weaken it. Below
/// this the matcher gets [`NoFrequencies`] and uses its table's `u`.
pub const MIN_FREQUENCY_POPULATION: u64 = 1_000;

/// Most active records the cached table counts.
///
/// Above this the table is a **sample**: the first `FREQUENCY_SCAN_CAP`
/// records by id. Ids are random UUIDs, so that is an unbiased sample for
/// shares, and a common value stays common in it. It bounds a rebuild to
/// `FREQUENCY_SCAN_CAP / FREQUENCY_SCAN_PAGE` queries.
pub const FREQUENCY_SCAN_CAP: u64 = 200_000;

/// Records read per query while building the cached table.
pub const FREQUENCY_SCAN_PAGE: u64 = 1_000;

/// Age after which the cached table is rebuilt.
pub const FREQUENCY_MAX_AGE: Duration = Duration::from_mins(10);

// ── The wire DTO ─────────────────────────────────────────────────────

/// The verdict on a pair (`agents/share/match-evidence.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    /// A hard conflict (two different national identifiers under one
    /// scheme) rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears the proof bar with no contradiction. The only
    /// strength that permits an automatic merge.
    Proven,
    /// A match, with no contradiction, but short of proof: review only.
    Consistent,
    /// Not a match on the matcher's own threshold.
    #[default]
    NonMatch,
}

impl From<MatchStrength> for EvidenceStrength {
    fn from(s: MatchStrength) -> Self {
        match s {
            MatchStrength::Contradicted => Self::Contradicted,
            MatchStrength::Proven => Self::Proven,
            MatchStrength::Consistent => Self::Consistent,
            MatchStrength::NonMatch => Self::NonMatch,
        }
    }
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceContradiction {
    /// What conflicts, e.g. `"identifier:united_kingdom_national_health_service_number"`.
    /// A scheme name, never a value.
    pub field: String,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceComponent {
    /// The component, e.g. `"family_name"`, `"identifier"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

/// How much the compared data proves, beside the similarity score.
///
/// Component names and bits only; see the module docs for what that
/// does and does not disclose.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
pub struct MatchEvidence {
    /// Sum of the components' log-likelihood weights, in bits. Each bit
    /// doubles the odds that the pair is one person.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `contradicted`.
    pub contradictions: Vec<EvidenceContradiction>,
    /// The verdict.
    pub strength: EvidenceStrength,
    /// Every term of the sum; the `bits` add up to `weight_bits`.
    #[serde(default)]
    pub components: Vec<EvidenceComponent>,
}

impl MatchEvidence {
    /// The aggregate only: `weight_bits`, `fields_compared` and
    /// `strength`, with `contradictions` and `components` emptied (SEC-G3).
    ///
    /// For a candidate the caller may read only masked. The terms carry
    /// component names, never values, but a name still says what the
    /// masked view hides, e.g. `contradiction:identifier:nhs` says the
    /// hidden record holds an NHS number. The aggregate discloses no more
    /// than the score, and keeps the strength a reviewer needs. The case
    /// service's `redact_evidence` is the reference.
    #[must_use]
    pub fn aggregate_only(self) -> Self {
        Self {
            contradictions: Vec::new(),
            components: Vec::new(),
            ..self
        }
    }
}

impl From<&Evidence> for MatchEvidence {
    fn from(e: &Evidence) -> Self {
        Self {
            weight_bits: e.weight_bits,
            fields_compared: e.fields_compared,
            contradictions: e
                .contradictions
                .iter()
                .map(|c| EvidenceContradiction {
                    field: c.field.clone(),
                })
                .collect(),
            strength: e.strength.into(),
            components: e
                .components
                .iter()
                .map(|c| EvidenceComponent {
                    component: c.component.clone(),
                    bits: c.bits,
                    correlated: c.correlated,
                    frequency_adjusted: c.frequency_adjusted,
                })
                .collect(),
        }
    }
}

impl MatchEvidence {
    /// `true` when the strength is [`EvidenceStrength::Proven`].
    #[must_use]
    pub fn is_proven(&self) -> bool {
        self.strength == EvidenceStrength::Proven
    }

    /// This evidence as JSON, for a `JSONB` column.
    #[must_use]
    pub fn to_json(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self).ok()
    }
}

/// Whether a pair may be flagged or executed as an automatic merge:
/// `score >= auto_merge_threshold` **and** proven evidence
/// (`agents/share/match-evidence.md` §4). Anything else goes to review.
#[must_use]
pub fn auto_merge_permitted(
    score: f64,
    auto_merge_threshold: f64,
    evidence: &MatchEvidence,
) -> bool {
    score >= auto_merge_threshold && evidence.is_proven()
}

// ── Computing evidence ──────────────────────────────────────────────

/// The engine evidence is computed with: the matcher's defaults, so its
/// `is_match` threshold (`0.85`) and proof bar (`30` bits) are the
/// family's. Built once; the engine is immutable and `Sync`.
fn engine() -> &'static MatchingEngine {
    static ENGINE: OnceLock<MatchingEngine> = OnceLock::new();
    ENGINE.get_or_init(|| MatchingEngine::new(MatchConfig::default()))
}

/// The matcher's evidence for `a` / `b`, with agreement weights for
/// given name, family name and birth date taken from `freqs`.
#[must_use]
pub fn pair_evidence(a: &Person, b: &Person, freqs: &dyn ValueFrequencies) -> MatchEvidence {
    let result = engine().match_persons_with_frequencies(
        &to_matcher_person(a),
        &to_matcher_person(b),
        freqs,
    );
    MatchEvidence::from(&result.evidence)
}

// ── Value frequencies ───────────────────────────────────────────────

/// Counts values under each of [`FREQUENCY_COMPONENTS`], in the
/// matcher's own key form ([`frequency_key`]).
#[derive(Debug, Default)]
pub struct FrequencyCounter {
    /// `(component, key)` → number of records holding it.
    counts: HashMap<(&'static str, String), u64>,
    /// Records counted.
    population: u64,
}

impl FrequencyCounter {
    /// An empty counter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Count one person, mapped through the adapter exactly as a match
    /// maps it, so a counted key is the key the matcher looks up.
    pub fn add(&mut self, person: &Person) {
        let m = to_matcher_person(person);
        self.population += 1;
        for &component in FREQUENCY_COMPONENTS {
            let raw = match component {
                "given_name" => m.given_name.clone(),
                "family_name" => m.family_name.clone(),
                "date_of_birth" => m.date_of_birth.map(|d| d.format("%Y-%m-%d").to_string()),
                _ => None,
            };
            if let Some(key) = raw.and_then(|r| frequency_key(component, &r)) {
                *self.counts.entry((component, key)).or_insert(0) += 1;
            }
        }
    }

    /// Records counted so far.
    #[must_use]
    pub fn population(&self) -> u64 {
        self.population
    }

    /// The frequency source these counts support: a table when the
    /// population is at least [`MIN_FREQUENCY_POPULATION`], otherwise
    /// none. `sampled` records that the scan stopped at its cap.
    #[must_use]
    pub fn finish(self, sampled: bool) -> ValueFrequencySource {
        if self.population < MIN_FREQUENCY_POPULATION {
            return ValueFrequencySource::None;
        }
        let population = self.population;
        let table = FrequencyTable::from_counts(
            self.counts.into_iter().map(|((c, k), n)| (c, k, n)),
            population,
        );
        ValueFrequencySource::Table {
            table,
            population,
            sampled,
        }
    }
}

/// Where match evidence takes its value frequencies from.
#[derive(Debug, Clone, Default)]
pub enum ValueFrequencySource {
    /// No frequencies: the population is below
    /// [`MIN_FREQUENCY_POPULATION`], or no table has been built yet.
    #[default]
    None,
    /// Shares counted over `population` active records.
    Table {
        /// The shares.
        table: FrequencyTable,
        /// Records the shares were counted over.
        population: u64,
        /// `true` when the scan stopped at [`FREQUENCY_SCAN_CAP`], so the
        /// table is a sample of the registry.
        sampled: bool,
    },
}

impl ValueFrequencySource {
    /// Count `persons`, e.g. the records a batch scan loaded.
    #[must_use]
    pub fn from_persons(persons: &[Person]) -> Self {
        let mut counter = FrequencyCounter::new();
        for p in persons {
            counter.add(p);
        }
        counter.finish(false)
    }

    /// `true` when this source holds a table.
    #[must_use]
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table { .. })
    }
}

impl ValueFrequencies for ValueFrequencySource {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        match self {
            Self::None => NoFrequencies.share(component, value),
            Self::Table { table, .. } => table.share(component, value),
        }
    }
}

/// Build a frequency source from the active persons in the database,
/// paging [`FREQUENCY_SCAN_PAGE`] at a time up to [`FREQUENCY_SCAN_CAP`].
///
/// # Errors
///
/// Returns the crate database error if a page cannot be read.
pub async fn scan_frequencies(db: &PgPool) -> crate::Result<ValueFrequencySource> {
    let mut counter = FrequencyCounter::new();
    let mut after = None;
    let mut sampled = false;
    loop {
        let remaining = FREQUENCY_SCAN_CAP - counter.population();
        if remaining == 0 {
            sampled = true;
            break;
        }
        let rows =
            crate::db::frequency::page(db, after, remaining.min(FREQUENCY_SCAN_PAGE)).await?;
        let Some(last) = rows.last() else { break };
        after = Some(last.id);
        let full_page = rows.len() as u64 == remaining.min(FREQUENCY_SCAN_PAGE);
        for row in rows {
            let mut person = Person::new(
                HumanName {
                    use_type: None,
                    family: row.family.unwrap_or_default(),
                    given: row.given,
                    prefix: vec![],
                    suffix: vec![],
                },
                Gender::Unknown,
            );
            person.birth_date = row.birth_date;
            counter.add(&person);
        }
        if !full_page {
            break;
        }
    }
    Ok(counter.finish(sampled))
}

/// The frequency source for the interactive paths (create-time duplicate
/// check, `check-duplicates`, `match`): built from the database, held in
/// app state, rebuilt in the **background** when older than
/// [`FREQUENCY_MAX_AGE`].
///
/// [`FrequencyCache::current`] never waits. When the table is missing or
/// stale it starts one rebuild (at most one at a time) on a spawned task
/// and answers with what it holds: the stale table, or, before the first
/// build completes, [`ValueFrequencySource::None`]. A request therefore
/// never blocks on a scan; the cost of that is that the first requests
/// after boot use the matcher's plain table.
#[derive(Debug, Default)]
pub struct FrequencyCache {
    /// The table and when it was built.
    held: RwLock<Option<(Instant, Arc<ValueFrequencySource>)>>,
    /// A rebuild is running.
    rebuilding: AtomicBool,
}

impl FrequencyCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The held source, or none; starts a background rebuild against `db`
    /// when the held one is missing or stale. Outside a Tokio runtime no
    /// rebuild is started.
    pub fn current(self: &Arc<Self>, db: &PgPool) -> Arc<ValueFrequencySource> {
        let (held, stale) = self.snapshot();
        if stale
            && let Ok(handle) = tokio::runtime::Handle::try_current()
            && !self.rebuilding.swap(true, Ordering::AcqRel)
        {
            let cache = Arc::clone(self);
            let db = db.clone();
            handle.spawn(async move {
                match scan_frequencies(&db).await {
                    Ok(source) => cache.store(source),
                    Err(e) => tracing::warn!("value-frequency table rebuild failed: {e}"),
                }
                cache.rebuilding.store(false, Ordering::Release);
            });
        }
        held.unwrap_or_else(|| Arc::new(ValueFrequencySource::None))
    }

    /// Replace the held source, stamped now.
    pub fn store(&self, source: ValueFrequencySource) {
        let mut held = self
            .held
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *held = Some((Instant::now(), Arc::new(source)));
    }

    /// The held source and whether it needs a rebuild.
    fn snapshot(&self) -> (Option<Arc<ValueFrequencySource>>, bool) {
        let held = self
            .held
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match held.as_ref() {
            Some((at, source)) => (Some(Arc::clone(source)), at.elapsed() >= FREQUENCY_MAX_AGE),
            None => (None, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Identifier, IdentifierType};
    use chrono::NaiveDate;

    fn person(family: &str, given: &str, dob: Option<NaiveDate>) -> Person {
        let mut p = Person::new(
            HumanName {
                use_type: None,
                family: family.into(),
                given: vec![given.into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        p.birth_date = dob;
        p
    }

    fn nhs(value: &str) -> Identifier {
        Identifier::new(
            IdentifierType::Other,
            "https://fhir.nhs.uk/Id/nhs-number".into(),
            value.into(),
        )
    }

    fn dob() -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(1980, 1, 15)
    }

    /// Name, birth date and gender agreeing is consistent, not proven:
    /// twins look exactly like this.
    #[test]
    fn name_and_birth_date_alone_are_consistent() {
        let a = person("Hartley", "Margaret", dob());
        let b = person("Hartley", "Margaret", dob());
        let e = pair_evidence(&a, &b, &NoFrequencies);
        assert_eq!(e.strength, EvidenceStrength::Consistent, "{e:?}");
        assert!(!auto_merge_permitted(1.0, 0.95, &e));
    }

    /// A shared NHS number proves the pair.
    #[test]
    fn a_shared_national_identifier_is_proven() {
        let mut a = person("Hartley", "Margaret", dob());
        let mut b = person("Hartley", "Margaret", dob());
        a.identifiers.push(nhs("943 476 5919"));
        b.identifiers.push(nhs("9434765919"));
        let e = pair_evidence(&a, &b, &NoFrequencies);
        assert_eq!(e.strength, EvidenceStrength::Proven, "{e:?}");
        assert!(auto_merge_permitted(0.96, 0.95, &e));
        assert!(!auto_merge_permitted(0.90, 0.95, &e), "score still gates");
    }

    /// Two different NHS numbers contradict, whatever else agrees.
    #[test]
    fn conflicting_identifiers_contradict() {
        let mut a = person("Hartley", "Margaret", dob());
        let mut b = person("Hartley", "Margaret", dob());
        a.identifiers.push(nhs("9434765919"));
        b.identifiers.push(nhs("4010232137"));
        let e = pair_evidence(&a, &b, &NoFrequencies);
        assert_eq!(e.strength, EvidenceStrength::Contradicted, "{e:?}");
        assert!(!e.contradictions.is_empty());
        assert!(!auto_merge_permitted(1.0, 0.95, &e));
    }

    /// Evidence names components and never carries a value.
    #[test]
    fn evidence_carries_no_field_values() {
        let mut a = person("Zbigniewska", "Margaret", dob());
        let mut b = person("Zbigniewska", "Margaret", dob());
        a.identifiers.push(nhs("9434765919"));
        b.identifiers.push(nhs("4010232137"));
        let json = serde_json::to_string(&pair_evidence(&a, &b, &NoFrequencies)).unwrap();
        for value in [
            "Zbigniewska",
            "zbigniewska",
            "Margaret",
            "margaret",
            "1980",
            "9434765919",
            "4010232137",
            "5919",
        ] {
            assert!(!json.contains(value), "{value} leaked into {json}");
        }
    }

    /// Below the population floor, the counted source is none.
    #[test]
    fn a_small_population_gives_no_frequencies() {
        let persons: Vec<Person> = (0..(MIN_FREQUENCY_POPULATION - 1))
            .map(|i| person("Smith", &format!("G{i}"), dob()))
            .collect();
        let source = ValueFrequencySource::from_persons(&persons);
        assert!(!source.is_table());
        assert_eq!(source.share("family_name", "smith"), None);
    }

    /// Above it, a common value gives fewer bits than the table default.
    #[test]
    fn a_common_value_weighs_less_than_the_table_default() {
        // 2 000 records, a fifth of them Smith; the rest unique names.
        let persons: Vec<Person> = (0..2_000u32)
            .map(|i| {
                let family = if i % 5 == 0 {
                    "Smith".to_string()
                } else {
                    format!("F{i}x")
                };
                person(&family, &format!("G{i}y"), None)
            })
            .collect();
        let source = ValueFrequencySource::from_persons(&persons);
        assert!(source.is_table());
        let share = source.share("family_name", "smith").unwrap();
        assert!((share - 0.2).abs() < 1e-9, "{share}");

        let a = person("Smith", "Margaret", dob());
        let b = person("Smith", "Margaret", dob());
        let plain = pair_evidence(&a, &b, &NoFrequencies);
        let adjusted = pair_evidence(&a, &b, &source);
        let bits = |e: &MatchEvidence| {
            e.components
                .iter()
                .find(|c| c.component == "family_name")
                .map(|c| (c.bits, c.frequency_adjusted))
                .unwrap()
        };
        let (plain_bits, plain_adjusted) = bits(&plain);
        let (adj_bits, adj_adjusted) = bits(&adjusted);
        assert!(!plain_adjusted && adj_adjusted);
        assert!(adj_bits < plain_bits, "{adj_bits} !< {plain_bits}");
        assert!(adjusted.weight_bits < plain.weight_bits);
    }

    /// A cache with nothing held answers none and never panics outside a
    /// runtime; a stored table is returned until it is stale.
    #[test]
    fn the_cache_answers_without_waiting() {
        let cache = Arc::new(FrequencyCache::new());
        let (held, stale) = cache.snapshot();
        assert!(held.is_none() && stale);
        cache.store(ValueFrequencySource::None);
        let (held, stale) = cache.snapshot();
        assert!(held.is_some() && !stale);
    }

    /// JSON written before `components` existed still reads.
    #[test]
    fn evidence_json_without_components_deserialises() {
        let e: MatchEvidence = serde_json::from_value(serde_json::json!({
            "weight_bits": 12.5, "fields_compared": 3,
            "contradictions": [], "strength": "consistent"
        }))
        .unwrap();
        assert_eq!(e.strength, EvidenceStrength::Consistent);
        assert!(e.components.is_empty());
    }
}
