//! **Seeded synthetic journey cohorts** (spec `13-tasks.md` T-14m) —
//! deterministic, DB-free generation of pathway instances with
//! segments, steps, events, and a care team, for every T-14 test and
//! the `journeys:seed` CLI task
//! ([`crate::tasks::journeys_seed`]) to persist.
//!
//! **Never real data, never derived from real data.** Every generated
//! `subject_ref` / care-team `member_ref` / `location_ref` carries a
//! fixed, recognizably-fake byte prefix ([`SUBJECT_PREFIX`],
//! [`ACTOR_PREFIX`], [`LOCATION_PREFIX`]) rather than looking like an
//! ordinary random UUID — a human scanning logs, an export, or a
//! database dump can rule this data out on sight, the same discipline
//! IPPA-data's own disclaimer names ("no responsibility to answer any
//! epidemiological question"). This module answers none either.
//!
//! **Reproducibility is the load-bearing property.** [`generate_cohort`]
//! is a pure function of its [`SeedParams`] — no clock read, no
//! `Uuid::new_v4`, no I/O — so the same seed always produces the same
//! cohort, byte for byte (pinned directly by
//! `same_seed_produces_byte_identical_output` below). The pseudo-random
//! source is a small hand-rolled generator (`Rng`, SplitMix64), not
//! the `rand` crate: reproducibility across `rand` versions is not
//! guaranteed by that crate's own docs, and pulling in a dependency
//! (plus this crate's SOUP register, `compliance/soup.tsv`) to generate
//! fixture data was judged not worth it when twenty lines suffice.
//!
//! A clean (no defect requested) instance satisfies every
//! `spec/time-based-analysis.md` §5.1 segment invariant by
//! construction. [`DEFECT_CODES`] injects exactly one violation per
//! requested code, for T-14h's still-unbuilt data-quality report to
//! test against once it lands — see that constant's own doc comment
//! for the one T-14h code this generator cannot yet produce.

use chrono::{DateTime, Duration, NaiveDate, Utc};

use crate::instances as rules;
use crate::tba;

/// First four bytes of every synthetic `subject_ref` UUID this
/// generator mints — chosen to read as "facade" in hex, not to look
/// like an ordinary random UUID. The remaining twelve bytes are drawn
/// from the seeded [`Rng`], so two runs with the same seed mint the
/// same UUID.
pub const SUBJECT_PREFIX: [u8; 4] = [0xfa, 0xca, 0xde, 0x50];

/// Same idea as [`SUBJECT_PREFIX`], for care-team `member_ref` URNs.
pub const ACTOR_PREFIX: [u8; 4] = [0xfa, 0xca, 0xde, 0x51];

/// Same idea as [`SUBJECT_PREFIX`], for `location_ref` URNs.
pub const LOCATION_PREFIX: [u8; 4] = [0xfa, 0xca, 0xde, 0x52];

/// The closed vocabulary of injectable defects, one instance per
/// requested code (spec T-14m, extending T-14h's still-unbuilt report).
/// Each name matches the condition T-14h's own spec text lists.
///
/// T-14h's spec text lists an eighth code, "anchors unreached" — not
/// included here because it needs T-14d's stage-anchor configuration,
/// which does not exist yet. Adding it is a follow-up once T-14d lands,
/// not a silent omission.
pub const DEFECT_CODES: &[&str] = &[
    "no_segments",
    "open_segment_past_closure",
    "terminal_without_clock_stop",
    "step_done_before_enrolled",
    "steps_out_of_order",
    "segment_clipped_by_clock",
    "coverage_below_floor",
];

/// The fixed reference date every seeded cohort's `enrolled_on` spread
/// is anchored to. **Never `Utc::now()`** — a generator whose output
/// depends on when it happened to run is not reproducible, which is
/// this task's one load-bearing property.
///
/// # Panics
///
/// Never in practice: `2026-01-01` is a valid calendar date.
#[must_use]
pub fn default_base_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 1).expect("2026-01-01 is a valid date")
}

/// What to generate.
#[derive(Debug, Clone)]
pub struct SeedParams {
    /// Clean instances to generate, before any defect instances.
    pub n: usize,
    /// The PRNG seed. The same seed + the same other fields always
    /// produces the same cohort.
    pub seed: u64,
    /// Share of clean instances left open (still enrolled) rather than
    /// closed — clamped to `[0, 1]`. This is the raw signal T-14e's
    /// right-censoring statistics need.
    pub open_share: f64,
    /// Defect codes to inject, one extra instance per code, appended
    /// after the `n` clean ones. Must each be a [`DEFECT_CODES`] entry.
    pub defects: Vec<String>,
    /// The fixed date every instance's `enrolled_on` is offset from.
    /// [`default_base_date`] unless a test needs a different anchor.
    pub base_date: NaiveDate,
}

impl Default for SeedParams {
    fn default() -> Self {
        Self {
            n: 10,
            seed: 42,
            open_share: 0.2,
            defects: Vec::new(),
            base_date: default_base_date(),
        }
    }
}

/// One generated journey segment — the plain shape
/// [`crate::tasks::journeys_seed`] inserts as an `instance_segments` row.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedSegment {
    /// Human label.
    pub label: String,
    /// One of [`tba::STAGES`].
    pub stage: String,
    /// One of [`tba::CATEGORIES`].
    pub category: String,
    /// One of [`tba::WASTES`], set only on an unnecessary segment.
    pub waste: Option<String>,
    /// Interval start.
    pub started_at: DateTime<Utc>,
    /// Interval end; `None` while still running.
    pub ended_at: Option<DateTime<Utc>>,
    /// A `worker:`/`organization:` URN, drawn from the instance's own
    /// generated care team.
    pub actor_ref: Option<String>,
    /// A `place:` URN.
    pub location_ref: Option<String>,
}

/// One generated instance step.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedStep {
    /// Human label.
    pub label: String,
    /// Whether it is marked done.
    pub done: bool,
    /// Completion date, when done.
    pub done_on: Option<NaiveDate>,
}

/// One generated instance event.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedEvent {
    /// One of [`rules::EVENT_KINDS`].
    pub kind: String,
    /// When it occurred.
    pub occurred_at: DateTime<Utc>,
}

/// One generated care-team membership.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedTeamMember {
    /// A `worker:`/`organization:` URN.
    pub member_ref: String,
    /// One of [`rules::TEAM_ROLES`].
    pub role: String,
}

/// One generated pathway instance, with its child rows.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedInstance {
    /// A `person:` URN carrying [`SUBJECT_PREFIX`] — never real data.
    pub subject_ref: String,
    /// One of [`rules::URGENCY_LEVELS`].
    pub urgency: String,
    /// One of [`rules::INSTANCE_STATUSES`].
    pub status: String,
    /// Enrolment date.
    pub enrolled_on: NaiveDate,
    /// Closure date, once closed.
    pub closed_on: Option<NaiveDate>,
    /// Recorded outcome, once closed.
    pub outcome: Option<String>,
    /// Time-based-analysis clock start.
    pub clock_start_at: Option<DateTime<Utc>>,
    /// Time-based-analysis clock stop; `None` while the clock runs.
    pub clock_stop_at: Option<DateTime<Utc>>,
    /// The instance's journey segments, in position order.
    pub segments: Vec<GeneratedSegment>,
    /// The instance's checklist steps, in position order.
    pub steps: Vec<GeneratedStep>,
    /// The instance's point-in-time events.
    pub events: Vec<GeneratedEvent>,
    /// The instance's care team.
    pub team: Vec<GeneratedTeamMember>,
    /// Which [`DEFECT_CODES`] entry this instance was made to carry, or
    /// `None` for a clean instance.
    pub defect: Option<String>,
}

/// A generated cohort — every instance for one `journeys:seed` run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GeneratedCohort {
    /// The generated instances, clean ones first.
    pub instances: Vec<GeneratedInstance>,
}

/// A tiny, deterministic, dependency-free PRNG (`SplitMix64`). Not
/// cryptographic — irrelevant for synthetic fixture data, where the
/// only requirement is that the same seed always replays the same
/// stream.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A float in `[0, 1)`.
    #[allow(clippy::cast_precision_loss)] // 53 significant bits into an f64 mantissa, by construction
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// An integer in `[lo, hi]` inclusive. `hi <= lo` returns `lo`.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // span is always small and non-negative here
    fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    #[allow(clippy::cast_possible_truncation)] // reduced mod items.len() first; fixture generation, not a security boundary
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let index = self.next_u64() % (items.len().max(1) as u64);
        &items[index as usize]
    }
}

/// Build one synthetic URN with a fixed, recognizable prefix and a
/// deterministic tail.
fn synthetic_urn(rng: &mut Rng, prefix: [u8; 4], scheme: &str) -> String {
    let mut bytes = [0u8; 16];
    bytes[..4].copy_from_slice(&prefix);
    let a = rng.next_u64().to_be_bytes();
    let b = rng.next_u64().to_be_bytes();
    bytes[4..12].copy_from_slice(&a);
    bytes[12..16].copy_from_slice(&b[..4]);
    format!("{scheme}:{}", uuid::Uuid::from_bytes(bytes))
}

fn pick_category(rng: &mut Rng) -> &'static str {
    let r = rng.next_f64();
    if r < 0.60 {
        tba::CATEGORY_VALUE_ADDING
    } else if r < 0.85 {
        tba::CATEGORY_NECESSARY
    } else {
        tba::CATEGORY_UNNECESSARY
    }
}

fn duration_for(rng: &mut Rng, category: &str) -> Duration {
    if category == tba::CATEGORY_VALUE_ADDING {
        Duration::minutes(rng.range_i64(30, 240))
    } else if category == tba::CATEGORY_NECESSARY {
        Duration::minutes(rng.range_i64(15, 120))
    } else {
        Duration::hours(rng.range_i64(2, 72))
    }
}

fn stage_inclusion_chance(stage: &str) -> f64 {
    match stage {
        "referral" | "treatment" => 0.9,
        "triage" => 0.8,
        "diagnostics" => 0.6,
        "discharge" => 0.5,
        "follow_up" => 0.4,
        _ => 0.1, // "other"
    }
}

/// Midnight UTC of a `NaiveDate`, matching
/// `src/controllers/tba.rs`'s `date_ms`/`resolve_clock` day-resolution
/// convention.
fn midnight(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("00:00:00 is always a valid time")
        .and_utc()
}

/// Generate one clean instance — satisfies every §5.1 invariant by
/// construction: durations are always positive, `waste` is set iff
/// the segment is `unnecessary_non_value_adding`, and at most the last
/// segment is ever left open.
fn generate_team(rng: &mut Rng) -> Vec<GeneratedTeamMember> {
    (0..rng.range_i64(1, 3))
        .map(|_| {
            let scheme = if rng.chance(0.85) {
                "worker"
            } else {
                "organization"
            };
            GeneratedTeamMember {
                member_ref: synthetic_urn(rng, ACTOR_PREFIX, scheme),
                role: (*rng.pick(rules::TEAM_ROLES)).to_string(),
            }
        })
        .collect()
}

fn generate_locations(rng: &mut Rng) -> Vec<String> {
    (0..rng.range_i64(1, 2))
        .map(|_| synthetic_urn(rng, LOCATION_PREFIX, "place"))
        .collect()
}

/// Walk a subset of `tba::STAGES` back-to-back from `enrolled_at`,
/// returning the segments and the cursor just past the last one — the
/// natural close-out point unless the instance stays open.
fn generate_segments(
    rng: &mut Rng,
    enrolled_at: DateTime<Utc>,
    team: &[GeneratedTeamMember],
    locations: &[String],
) -> (Vec<GeneratedSegment>, DateTime<Utc>) {
    let mut included_stages: Vec<&'static str> = tba::STAGES
        .iter()
        .copied()
        .filter(|stage| rng.chance(stage_inclusion_chance(stage)))
        .collect();
    if included_stages.is_empty() {
        included_stages.push("treatment");
    }

    let mut cursor = enrolled_at;
    let mut segments = Vec::new();
    for stage in included_stages {
        for _ in 0..rng.range_i64(1, 2) {
            let category = pick_category(rng);
            let waste = (category == tba::CATEGORY_UNNECESSARY)
                .then(|| (*rng.pick(tba::WASTES)).to_string());
            let started_at = cursor;
            let ended_at = started_at + duration_for(rng, category);
            let actor_ref =
                (!team.is_empty() && rng.chance(0.85)).then(|| rng.pick(team).member_ref.clone());
            let location_ref =
                (!locations.is_empty() && rng.chance(0.7)).then(|| rng.pick(locations).clone());
            segments.push(GeneratedSegment {
                label: format!("{stage} activity"),
                stage: stage.to_string(),
                category: category.to_string(),
                waste,
                started_at,
                ended_at: Some(ended_at),
                actor_ref,
                location_ref,
            });
            cursor = ended_at + Duration::hours(rng.range_i64(0, 24));
        }
    }
    (segments, cursor)
}

/// Decide whether the instance stays open or closes, mutating
/// `segments` (leaving the last one running, sometimes, when it stays
/// open) and returning the instance-level closure fields.
fn resolve_closure(
    rng: &mut Rng,
    open_share: f64,
    cursor: DateTime<Utc>,
    segments: &mut [GeneratedSegment],
) -> (
    String,
    Option<NaiveDate>,
    Option<DateTime<Utc>>,
    Option<String>,
) {
    if rng.chance(open_share.clamp(0.0, 1.0)) {
        if rng.chance(0.5)
            && let Some(last) = segments.last_mut()
        {
            last.ended_at = None;
        }
        let status = if rng.chance(0.85) {
            "active"
        } else {
            "on_hold"
        };
        (status.to_string(), None, None, None)
    } else {
        let status = if rng.chance(0.85) {
            "completed"
        } else {
            "discontinued"
        };
        let outcome = (*rng.pick(rules::OUTCOMES)).to_string();
        (
            status.to_string(),
            Some(cursor.date_naive()),
            Some(cursor),
            Some(outcome),
        )
    }
}

fn generate_steps(rng: &mut Rng, enrolled_on: NaiveDate, is_closed: bool) -> Vec<GeneratedStep> {
    let mut step_cursor = enrolled_on;
    (0..rng.range_i64(0, 3))
        .map(|i| {
            let done = is_closed || rng.chance(0.5);
            let done_on = done.then(|| {
                step_cursor += Duration::days(rng.range_i64(1, 5));
                step_cursor
            });
            GeneratedStep {
                label: format!("step {i}"),
                done,
                done_on,
            }
        })
        .collect()
}

fn generate_events(rng: &mut Rng, enrolled_at: DateTime<Utc>) -> Vec<GeneratedEvent> {
    (0..rng.range_i64(0, 2))
        .map(|_| GeneratedEvent {
            kind: (*rng.pick(rules::EVENT_KINDS)).to_string(),
            occurred_at: enrolled_at + Duration::days(rng.range_i64(0, 10)),
        })
        .collect()
}

fn generate_clean_instance(rng: &mut Rng, params: &SeedParams) -> GeneratedInstance {
    let subject_ref = synthetic_urn(rng, SUBJECT_PREFIX, "person");
    let urgency = (*rng.pick(rules::URGENCY_LEVELS)).to_string();
    let enrolled_on = params.base_date + Duration::days(rng.range_i64(0, 60));
    let enrolled_at = midnight(enrolled_on) + Duration::hours(9);

    let team = generate_team(rng);
    let locations = generate_locations(rng);
    let (mut segments, cursor) = generate_segments(rng, enrolled_at, &team, &locations);
    let (status, closed_on, clock_stop_at, outcome) =
        resolve_closure(rng, params.open_share, cursor, &mut segments);
    let steps = generate_steps(rng, enrolled_on, closed_on.is_some());
    let events = generate_events(rng, enrolled_at);

    GeneratedInstance {
        subject_ref,
        urgency,
        status,
        enrolled_on,
        closed_on,
        outcome,
        clock_start_at: Some(enrolled_at),
        clock_stop_at,
        segments,
        steps,
        events,
        team,
        defect: None,
    }
}

/// A minimal placeholder segment, for a defect that needs at least one
/// segment to corrupt but the clean generation happened to produce none.
fn placeholder_segment(instance: &GeneratedInstance) -> GeneratedSegment {
    let start = instance
        .clock_start_at
        .unwrap_or_else(|| midnight(instance.enrolled_on));
    GeneratedSegment {
        label: "placeholder".to_string(),
        stage: "treatment".to_string(),
        category: tba::CATEGORY_VALUE_ADDING.to_string(),
        waste: None,
        started_at: start,
        ended_at: Some(start + Duration::hours(1)),
        actor_ref: None,
        location_ref: None,
    }
}

/// Mutate an otherwise-clean instance to introduce exactly the named
/// [`DEFECT_CODES`] condition, and nothing else.
fn apply_defect(instance: &mut GeneratedInstance, code: &str) {
    match code {
        "no_segments" => instance.segments.clear(),
        "open_segment_past_closure" => {
            if instance.segments.is_empty() {
                instance.segments.push(placeholder_segment(instance));
            }
            let stop = instance
                .clock_stop_at
                .or(instance.clock_start_at)
                .unwrap_or_else(|| midnight(instance.enrolled_on))
                + Duration::days(30);
            instance.status = "completed".to_string();
            instance.closed_on = Some(stop.date_naive());
            instance.clock_stop_at = Some(stop);
            instance.outcome = Some((*rules::OUTCOMES.first().unwrap_or(&"other")).to_string());
            if let Some(last) = instance.segments.last_mut() {
                last.ended_at = None;
            }
        }
        "terminal_without_clock_stop" => {
            instance.status = "completed".to_string();
            instance.outcome = Some((*rules::OUTCOMES.first().unwrap_or(&"other")).to_string());
            instance.closed_on = None;
            instance.clock_stop_at = None;
        }
        "step_done_before_enrolled" => {
            if instance.steps.is_empty() {
                instance.steps.push(GeneratedStep {
                    label: "consent".to_string(),
                    done: false,
                    done_on: None,
                });
            }
            instance.steps[0].done = true;
            instance.steps[0].done_on = Some(instance.enrolled_on - Duration::days(1));
        }
        "steps_out_of_order" => {
            while instance.steps.len() < 2 {
                let position = instance.steps.len();
                instance.steps.push(GeneratedStep {
                    label: format!("step {position}"),
                    done: false,
                    done_on: None,
                });
            }
            // Position 1's declared pair is position 0; complete the
            // later-declared step first.
            instance.steps[0].done = true;
            instance.steps[0].done_on = Some(instance.enrolled_on + Duration::days(5));
            instance.steps[1].done = true;
            instance.steps[1].done_on = Some(instance.enrolled_on + Duration::days(1));
        }
        "segment_clipped_by_clock" => {
            if instance.segments.is_empty() {
                instance.segments.push(placeholder_segment(instance));
            }
            let clock_start = instance
                .clock_start_at
                .unwrap_or_else(|| midnight(instance.enrolled_on));
            instance.segments[0].started_at = clock_start - Duration::days(2);
            instance.segments[0].ended_at = Some(clock_start - Duration::hours(1));
        }
        "coverage_below_floor" => {
            let start = instance
                .clock_start_at
                .unwrap_or_else(|| midnight(instance.enrolled_on));
            instance.segments = vec![GeneratedSegment {
                label: "brief check-in".to_string(),
                stage: "triage".to_string(),
                category: tba::CATEGORY_VALUE_ADDING.to_string(),
                waste: None,
                started_at: start,
                ended_at: Some(start + Duration::hours(1)),
                actor_ref: None,
                location_ref: None,
            }];
        }
        other => {
            unreachable!("generate_cohort validates codes before calling apply_defect: {other}")
        }
    }
}

/// Generate a synthetic cohort: `params.n` clean instances, followed by
/// one instance per `params.defects` entry, each carrying exactly that
/// defect.
///
/// # Errors
///
/// A code in `params.defects` that is not in [`DEFECT_CODES`].
pub fn generate_cohort(params: &SeedParams) -> Result<GeneratedCohort, String> {
    for code in &params.defects {
        if !DEFECT_CODES.contains(&code.as_str()) {
            return Err(format!(
                "unknown defect code '{code}': expected one of {DEFECT_CODES:?}"
            ));
        }
    }
    let mut rng = Rng::new(params.seed);
    let mut instances = Vec::with_capacity(params.n + params.defects.len());
    for _ in 0..params.n {
        instances.push(generate_clean_instance(&mut rng, params));
    }
    for code in &params.defects {
        let mut instance = generate_clean_instance(&mut rng, params);
        apply_defect(&mut instance, code);
        instance.defect = Some(code.clone());
        instances.push(instance);
    }
    Ok(GeneratedCohort { instances })
}

/// Every `spec/time-based-analysis.md` §5.1 invariant, checked against
/// one clean instance's segments. `#[cfg(test)]`-only: nothing in the
/// non-test build calls it.
#[cfg(test)]
fn assert_segment_invariants(instance: &GeneratedInstance) {
    let mut open_count = 0;
    for segment in &instance.segments {
        if let Some(end) = segment.ended_at {
            assert!(
                end > segment.started_at,
                "reversed or zero-length segment: {segment:?}"
            );
        } else {
            open_count += 1;
        }
        assert!(
            tba::CATEGORIES.contains(&segment.category.as_str()),
            "category not in the closed vocabulary: {segment:?}"
        );
        assert!(
            tba::STAGES.contains(&segment.stage.as_str()),
            "stage not in the closed vocabulary: {segment:?}"
        );
        if segment.category == tba::CATEGORY_VALUE_ADDING {
            assert!(
                segment.waste.is_none(),
                "waste on a value-adding segment: {segment:?}"
            );
        }
        if segment.category == tba::CATEGORY_UNNECESSARY {
            assert!(
                segment.waste.is_some(),
                "unnecessary segment with no waste type: {segment:?}"
            );
        }
    }
    assert!(
        open_count <= 1,
        "more than one open segment on instance: {instance:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn params(defects: &[&str]) -> SeedParams {
        SeedParams {
            n: 20,
            seed: 1234,
            open_share: 0.3,
            defects: defects.iter().map(ToString::to_string).collect(),
            base_date: default_base_date(),
        }
    }

    /// The one property everything else depends on: the same seed (and
    /// the same other params) always produces the same cohort.
    #[test]
    fn same_seed_produces_byte_identical_output() {
        let a = generate_cohort(&params(&[])).expect("valid params");
        let b = generate_cohort(&params(&[])).expect("valid params");
        assert_eq!(a, b);
    }

    /// A different seed produces a different cohort — the generator is
    /// not accidentally constant.
    #[test]
    fn different_seed_produces_different_output() {
        let mut p = params(&[]);
        let a = generate_cohort(&p).expect("valid params");
        p.seed = p.seed.wrapping_add(1);
        let b = generate_cohort(&p).expect("valid params");
        assert_ne!(a, b);
    }

    /// Every clean (no defect) instance satisfies every §5.1 invariant.
    #[test]
    fn clean_instances_satisfy_segment_invariants() {
        let cohort = generate_cohort(&params(&[])).expect("valid params");
        assert_eq!(cohort.instances.len(), 20);
        for instance in &cohort.instances {
            assert!(instance.defect.is_none());
            assert_segment_invariants(instance);
        }
    }

    /// An unknown defect code is rejected, not silently ignored.
    #[test]
    fn unknown_defect_code_is_rejected() {
        let mut p = params(&[]);
        p.defects = vec!["not_a_real_code".to_string()];
        assert!(generate_cohort(&p).is_err());
    }

    /// Requesting every defect code produces exactly one instance per
    /// code, in addition to the `n` clean ones, and every requested
    /// code is actually carried by exactly one instance.
    #[test]
    fn each_requested_defect_produces_exactly_one_instance_carrying_it() {
        let cohort = generate_cohort(&params(DEFECT_CODES)).expect("valid params");
        assert_eq!(cohort.instances.len(), 20 + DEFECT_CODES.len());
        let carried: BTreeSet<&str> = cohort
            .instances
            .iter()
            .filter_map(|i| i.defect.as_deref())
            .collect();
        assert_eq!(carried.len(), DEFECT_CODES.len());
        for code in DEFECT_CODES {
            assert!(carried.contains(code), "no instance carried {code}");
        }
        // The first 20 are still clean — a defect request never
        // perturbs the clean instances that precede it in the stream.
        for instance in &cohort.instances[..20] {
            assert!(instance.defect.is_none());
        }
    }

    /// Each defect actually produces the condition its name claims.
    #[test]
    fn each_defect_produces_its_named_condition() {
        for code in DEFECT_CODES {
            let cohort = generate_cohort(&params(&[*code])).expect("valid params");
            let instance = cohort.instances.last().expect("one defect instance");
            match *code {
                "no_segments" => assert!(instance.segments.is_empty()),
                "open_segment_past_closure" => {
                    assert!(rules::is_terminal(&instance.status));
                    assert!(
                        instance
                            .segments
                            .last()
                            .is_some_and(|s| s.ended_at.is_none()),
                        "expected a still-open segment on a closed instance"
                    );
                }
                "terminal_without_clock_stop" => {
                    assert!(rules::is_terminal(&instance.status));
                    assert!(instance.closed_on.is_none());
                    assert!(instance.clock_stop_at.is_none());
                }
                "step_done_before_enrolled" => {
                    let step = &instance.steps[0];
                    assert!(step.done_on.is_some_and(|d| d < instance.enrolled_on));
                }
                "steps_out_of_order" => {
                    assert!(instance.steps[0].done_on > instance.steps[1].done_on);
                }
                "segment_clipped_by_clock" => {
                    let clock_start = instance.clock_start_at.expect("clock start set");
                    assert!(instance.segments[0].started_at < clock_start);
                }
                "coverage_below_floor" => {
                    assert_eq!(instance.segments.len(), 1);
                    let segment = &instance.segments[0];
                    let span = segment.ended_at.expect("closed") - segment.started_at;
                    assert!(span <= Duration::hours(1));
                }
                other => panic!("unexercised defect code: {other}"),
            }
        }
    }

    /// Every synthetic ref carries its fixed, recognizably-fake prefix
    /// — never a plausible real-looking UUID.
    #[test]
    fn synthetic_refs_are_always_recognizably_fake() {
        let cohort = generate_cohort(&params(DEFECT_CODES)).expect("valid params");
        for instance in &cohort.instances {
            assert!(instance.subject_ref.starts_with("person:facade50"));
            for member in &instance.team {
                assert!(
                    member.member_ref.contains(":facade51"),
                    "{}",
                    member.member_ref
                );
            }
            for segment in &instance.segments {
                if let Some(location) = &segment.location_ref {
                    assert!(location.starts_with("place:facade52"), "{location}");
                }
            }
        }
    }

    /// `open_share` is clamped, never rejected, matching the family's
    /// pagination-style "clamp, don't error" convention for bounded
    /// numeric knobs.
    #[test]
    fn open_share_outside_zero_one_is_clamped_not_rejected() {
        let mut p = params(&[]);
        p.open_share = 5.0;
        assert!(generate_cohort(&p).is_ok());
        p.open_share = -1.0;
        assert!(generate_cohort(&p).is_ok());
    }
}
