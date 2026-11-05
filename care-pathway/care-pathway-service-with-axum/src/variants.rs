//! **Journey variants (pathway strings)** — spec `13-tasks.md` T-14c.
//! Pure, DB-free: transforms one instance's segments into a compact
//! "pathway string" (`referral-diagnostics-treatment+follow_up-…`)
//! through a chain of **named, defaulted, echoed** parameters
//! ([`VariantParams`]), then aggregates a cohort of such strings into a
//! frequency/coverage Pareto plus per-position ("line") duration
//! quantiles ([`summarize_variants`]).
//!
//! No suppression decision is baked in beyond the one the acceptance
//! criterion itself asks for — folding small variants into a disclosed
//! `suppressed_instances` count ([`summarize_variants`]'s
//! `min_cell_count` parameter, always supplied by the caller, never
//! read from the environment here, so this module stays pure and its
//! tests stay deterministic). `src/controllers/tba.rs`'s `variants`
//! handler is what actually calls `suppression::min_cell_count()`.
//!
//! ## The pipeline, in order
//!
//! 1. [`eras_from_segments`] — one [`Era`] per segment, sorted by start.
//!    A still-open segment's end is stood in by the caller's `as_of_ms`.
//! 2. **`min_segment_days`** — eras shorter than this are dropped
//!    outright (never merged into a neighbour).
//! 3. **`collapse_gap_days`** — chronologically adjacent eras of the
//!    *same* stage separated by a gap no larger than this become one
//!    era. An actual time overlap (gap `<= 0`) always collapses too.
//! 4. **`combination_window_days`** — [`combine_overlaps`] resolves
//!    every remaining time overlap between *different* stages into up
//!    to three non-overlapping pieces (see that function's own doc
//!    comment for the FRFS/LRFS geometry this crate uses, adapted from
//!    `TreatmentPatterns`' own two overlap shapes). An overlap at least
//!    this long becomes a canonical alphabetical combination era
//!    (`a+b`, converging to `a+b+c` under a three-way overlap since the
//!    resolution iterates to a fixed point); a shorter one is a
//!    "handoff" and is attributed to the incoming (later-starting)
//!    stage instead.
//! 5. **`min_post_combination_days`** — every *solo* era emerging from
//!    step 4 that is still shorter than this is dropped as a stub. A
//!    combination era (its label contains `+`) is exempt — the name is
//!    "post-combination," what is left *around* a combination, not the
//!    combination block itself (see `drop_post_combination_stubs`).
//! 6. **`filter`** ([`FilterMode`]) — `all` (default, nothing
//!    collapsed), `changes` (consecutive duplicate labels collapsed),
//!    or `first` (only each label's first occurrence anywhere in the
//!    sequence survives). `all` is this crate's default, not
//!    `TreatmentPatterns`' own `"First"` — the family's standing bias
//!    is never to silently drop detail unless asked.
//! 7. **`max_path_length`** — the era list is truncated to at most this
//!    many entries.
//!
//! The final era list's stage labels, joined by `-`, are the variant
//! string; the same era list's per-position durations feed the
//! cohort-level "line" quantiles.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::analytics::SegmentInput;
use crate::tba;

/// `TreatmentPatterns`' `minEraDuration`, unnamed default: no filtering.
pub const DEFAULT_MIN_SEGMENT_DAYS: f64 = 0.0;
/// `TreatmentPatterns`' `eraCollapseSize` default.
pub const DEFAULT_COLLAPSE_GAP_DAYS: f64 = 30.0;
/// `TreatmentPatterns`' `combinationWindow` default.
pub const DEFAULT_COMBINATION_WINDOW_DAYS: f64 = 30.0;
/// `TreatmentPatterns`' `minPostCombinationDuration` default.
pub const DEFAULT_MIN_POST_COMBINATION_DAYS: f64 = 30.0;
/// This crate's own default — see [`FilterMode`]'s doc comment for why
/// it is not `TreatmentPatterns`' own default (`"First"`).
pub const DEFAULT_FILTER: FilterMode = FilterMode::All;
/// Bounds how long a variant string (and the request that renders it)
/// can grow; also this crate's own choice, not a reproduced default.
pub const DEFAULT_MAX_PATH_LENGTH: usize = 10;

/// How repeated stage labels are represented in the final sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    /// Only each label's first occurrence anywhere survives —
    /// `TreatmentPatterns`' `"First"`.
    First,
    /// Consecutive duplicate labels collapse to one — `a-a-b` becomes
    /// `a-b`, but `a-b-a` (not consecutive) stays as three.
    Changes,
    /// Nothing is collapsed. This crate's default: the family's
    /// standing bias is never to silently drop detail unless the
    /// caller asks for it.
    All,
}

impl FilterMode {
    /// Parse a `?filter=` query value.
    ///
    /// # Errors
    ///
    /// The value is present and is none of `first`/`changes`/`all`.
    pub fn parse(raw: Option<&str>) -> Result<Self, String> {
        match raw.map(str::to_ascii_lowercase).as_deref() {
            None | Some("all") => Ok(Self::All),
            Some("first") => Ok(Self::First),
            Some("changes") => Ok(Self::Changes),
            Some(other) => Err(format!(
                "filter must be \"first\", \"changes\", or \"all\", got \"{other}\""
            )),
        }
    }

    /// The wire token this mode round-trips through `?filter=`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Changes => "changes",
            Self::All => "all",
        }
    }
}

/// Every knob the variant transform takes — named, defaulted, and
/// always echoed back in the response so a caller can run a sensitivity
/// sweep (IPPA's `run_sens.py`, adopted here as a rule rather than a
/// dedicated endpoint).
#[derive(Debug, Clone)]
pub struct VariantParams {
    /// Eras shorter than this many days are dropped.
    pub min_segment_days: f64,
    /// Same-stage eras separated by no more than this many days
    /// collapse into one.
    pub collapse_gap_days: f64,
    /// An overlap at least this long becomes a canonical combination.
    pub combination_window_days: f64,
    /// An era shorter than this, after combination resolution, is
    /// dropped as a stub.
    pub min_post_combination_days: f64,
    /// How repeated labels are represented in the final sequence.
    pub filter: FilterMode,
    /// The final era list is truncated to at most this many entries.
    pub max_path_length: usize,
}

impl Default for VariantParams {
    fn default() -> Self {
        Self {
            min_segment_days: DEFAULT_MIN_SEGMENT_DAYS,
            collapse_gap_days: DEFAULT_COLLAPSE_GAP_DAYS,
            combination_window_days: DEFAULT_COMBINATION_WINDOW_DAYS,
            min_post_combination_days: DEFAULT_MIN_POST_COMBINATION_DAYS,
            filter: DEFAULT_FILTER,
            max_path_length: DEFAULT_MAX_PATH_LENGTH,
        }
    }
}

/// One interval of a single stage label (which may already be a
/// combination, e.g. `"triage+treatment"`) in an instance's journey.
#[derive(Debug, Clone, PartialEq)]
pub struct Era {
    /// The stage name, or an alphabetically-sorted `+`-joined
    /// combination of stage names.
    pub stage: String,
    /// Interval start, epoch milliseconds.
    pub start_ms: i64,
    /// Interval end, epoch milliseconds.
    pub end_ms: i64,
}

impl Era {
    #[must_use]
    fn duration_days(&self) -> f64 {
        to_days((self.end_ms - self.start_ms).max(0))
    }
}

#[allow(clippy::cast_precision_loss)] // display-only day count
fn to_days(ms: i64) -> f64 {
    ms as f64 / tba::DAY_MS as f64
}

/// Days between two instants, never negative (a genuine overlap reads
/// as a zero gap, which always collapses).
fn days_between(from_ms: i64, to_ms: i64) -> f64 {
    to_days((to_ms - from_ms).max(0))
}

/// Build the raw, sorted era list from one instance's segments. A
/// still-open segment's end is stood in by `as_of_ms`.
#[must_use]
pub fn eras_from_segments(segments: &[SegmentInput], as_of_ms: i64) -> Vec<Era> {
    let mut eras: Vec<Era> = segments
        .iter()
        .map(|segment| Era {
            stage: segment.stage.clone(),
            start_ms: segment.start_ms,
            end_ms: segment.end_ms.unwrap_or(as_of_ms),
        })
        .collect();
    eras.sort_by_key(|era| era.start_ms);
    eras
}

fn filter_min_duration(eras: Vec<Era>, min_days: f64) -> Vec<Era> {
    eras.into_iter()
        .filter(|era| era.duration_days() >= min_days)
        .collect()
}

/// Merge chronologically adjacent same-stage eras separated by no more
/// than `gap_days` (input must already be sorted by start).
fn collapse_same_stage_gaps(eras: Vec<Era>, gap_days: f64) -> Vec<Era> {
    let mut out: Vec<Era> = Vec::with_capacity(eras.len());
    for era in eras {
        if let Some(last) = out.last_mut() {
            let gap = days_between(last.end_ms, era.start_ms);
            if last.stage == era.stage && gap <= gap_days {
                last.end_ms = last.end_ms.max(era.end_ms);
                continue;
            }
        }
        out.push(era);
    }
    out
}

/// Union the `+`-separated stage sets of two labels into one
/// alphabetically-sorted, deduplicated combination label — so a
/// three-way overlap converges to `a+b+c` regardless of which pair
/// combines first.
fn combined_label(a: &str, b: &str) -> String {
    let mut parts: Vec<&str> = a.split('+').chain(b.split('+')).collect();
    parts.sort_unstable();
    parts.dedup();
    parts.join("+")
}

/// Resolve every time overlap between adjacent (by start) eras into
/// non-overlapping pieces, iterating to a fixed point.
///
/// Two overlap shapes exist, named after `TreatmentPatterns`' own
/// vocabulary (confirmed against its CRAN documentation, not guessed):
/// for eras `a` (starts first) and `b` (starts second),
/// **FRFS** ("first received, first stopped") is the shape where `a`
/// also *ends* first or exactly with `b` (`a.end <= b.end`) — a
/// genuine crossing overlap; **LRFS** ("last received, first stopped")
/// is the shape where `b` ends first, i.e. `b`'s whole span is nested
/// inside `a`'s (`a.end > b.end`).
///
/// Both shapes decompose into up to three pieces: `a`'s solo lead-in
/// (if `b` starts after `a`), the contested middle stretch, and a
/// trailing solo piece — `b`'s remainder under FRFS, `a`'s resumption
/// under LRFS. This crate's own choice, not reproduced from
/// `TreatmentPatterns`' source (not available to verify against in
/// this environment): the middle stretch becomes the canonical
/// combination label when it is at least `combination_window_days`
/// long, or is attributed to the incoming (later-starting, `b`) stage
/// as a "handoff" when it is shorter.
#[must_use]
pub fn combine_overlaps(mut eras: Vec<Era>, combination_window_days: f64) -> Vec<Era> {
    // Bounded: a successful decomposition replaces 2 eras with up to 3,
    // so the list can only grow by 1 net per era pair per pass, and a
    // pass that changes nothing ends the loop.
    for _ in 0..=eras.len() {
        eras.sort_by_key(|era| era.start_ms);
        let mut next: Vec<Era> = Vec::with_capacity(eras.len());
        let mut changed = false;
        let mut i = 0;
        while i < eras.len() {
            let overlaps_next = i + 1 < eras.len() && eras[i + 1].start_ms < eras[i].end_ms;
            if overlaps_next {
                let a = eras[i].clone();
                let b = eras[i + 1].clone();
                for piece in decompose_pair(&a, &b, combination_window_days) {
                    next.push(piece);
                }
                i += 2;
                changed = true;
            } else {
                next.push(eras[i].clone());
                i += 1;
            }
        }
        eras = next;
        if !changed {
            break;
        }
    }
    eras
}

/// Decompose one overlapping pair into up to three non-overlapping
/// pieces (zero-length pieces are dropped). `a` must start no later
/// than `b`.
fn decompose_pair(a: &Era, b: &Era, combination_window_days: f64) -> Vec<Era> {
    let frfs = a.end_ms <= b.end_ms;
    let (overlap_start, overlap_end) = if frfs {
        (b.start_ms, a.end_ms)
    } else {
        (b.start_ms, b.end_ms)
    };
    let combine = days_between(overlap_start, overlap_end) >= combination_window_days;
    let overlap_label = if combine {
        combined_label(&a.stage, &b.stage)
    } else {
        b.stage.clone()
    };

    let before = (b.start_ms > a.start_ms).then(|| Era {
        stage: a.stage.clone(),
        start_ms: a.start_ms,
        end_ms: b.start_ms,
    });
    let overlap = Era {
        stage: overlap_label,
        start_ms: overlap_start,
        end_ms: overlap_end,
    };
    let after = if frfs {
        (b.end_ms > a.end_ms).then(|| Era {
            stage: b.stage.clone(),
            start_ms: a.end_ms,
            end_ms: b.end_ms,
        })
    } else {
        (a.end_ms > b.end_ms).then(|| Era {
            stage: a.stage.clone(),
            start_ms: b.end_ms,
            end_ms: a.end_ms,
        })
    };

    [before, Some(overlap), after]
        .into_iter()
        .flatten()
        .filter(|era| era.end_ms > era.start_ms)
        .collect()
}

/// Drop a solo era left over from overlap resolution once it is too
/// short to be meaningful. A **combination** era (its stage contains
/// `+`) is exempt: `min_post_combination_days` names what is left
/// *around* a combination, not the combination block itself — a real
/// 40-day co-treatment must not disappear because it happens to be
/// shorter than some solo-duration floor unrelated to it.
fn drop_post_combination_stubs(eras: Vec<Era>, min_days: f64) -> Vec<Era> {
    eras.into_iter()
        .filter(|era| era.stage.contains('+') || era.duration_days() >= min_days)
        .collect()
}

fn apply_filter_mode(eras: Vec<Era>, mode: FilterMode) -> Vec<Era> {
    match mode {
        FilterMode::All => eras,
        FilterMode::Changes => {
            let mut out: Vec<Era> = Vec::with_capacity(eras.len());
            for era in eras {
                let merge = out.last().is_some_and(|last: &Era| last.stage == era.stage);
                if merge {
                    if let Some(last) = out.last_mut() {
                        last.end_ms = last.end_ms.max(era.end_ms);
                    }
                } else {
                    out.push(era);
                }
            }
            out
        }
        FilterMode::First => {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            eras.into_iter()
                .filter(|era| seen.insert(era.stage.clone()))
                .collect()
        }
    }
}

fn truncate_path(eras: Vec<Era>, max_len: usize) -> Vec<Era> {
    eras.into_iter().take(max_len.max(1)).collect()
}

/// One instance's fully-transformed journey: the variant string plus
/// the era list it was built from (kept for the cohort's per-position
/// line-duration statistics).
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceVariant {
    /// The stage labels of `eras`, joined by `-`.
    pub variant: String,
    /// The final era list, in order.
    pub eras: Vec<Era>,
}

/// Run the full pipeline (§ this module's own doc comment) for one
/// instance's segments.
#[must_use]
pub fn build_variant(
    segments: &[SegmentInput],
    as_of_ms: i64,
    params: &VariantParams,
) -> InstanceVariant {
    let eras = eras_from_segments(segments, as_of_ms);
    let eras = filter_min_duration(eras, params.min_segment_days);
    let eras = collapse_same_stage_gaps(eras, params.collapse_gap_days);
    let eras = combine_overlaps(eras, params.combination_window_days);
    let eras = drop_post_combination_stubs(eras, params.min_post_combination_days);
    let eras = apply_filter_mode(eras, params.filter);
    let eras = truncate_path(eras, params.max_path_length);
    let variant = eras
        .iter()
        .map(|era| era.stage.as_str())
        .collect::<Vec<_>>()
        .join("-");
    InstanceVariant { variant, eras }
}

/// One row of the frequency/coverage Pareto.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VariantSummary {
    /// The pathway string.
    pub variant: String,
    /// Instances following exactly this variant.
    pub frequency: usize,
    /// This variant's share, **renormalised over the unsuppressed
    /// instances only** — so the visible variants' shares sum to 1.0
    /// even though `suppressed_instances` is excluded from the list.
    /// This is a deliberate choice, not `TreatmentPatterns`' own
    /// convention: folding suppressed variants' *mass* silently into
    /// the visible ones' shares would inflate them without saying so;
    /// this way the visible Pareto is self-consistent and the
    /// suppressed count is reported alongside it, not hidden inside it.
    pub share: f64,
    /// Running sum of `share` in frequency order.
    pub cumulative_share: f64,
}

/// One duration line: a position in the (post-pipeline) sequence, or
/// the `"overall"` pseudo-line summing every instance's whole journey.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LineStat {
    /// `"1"`, `"2"`, … or `"overall"`.
    pub position: String,
    /// Contributing instances.
    pub n: usize,
    /// Nearest-rank median duration, days.
    pub median_days: f64,
    /// Nearest-rank p90 duration, days.
    pub p90_days: f64,
}

/// The full cohort-level report.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct VariantsReport {
    /// Every instance considered (visible + suppressed).
    pub instances: usize,
    /// Instances folded out of `variants` because their variant's
    /// frequency was below the floor. Disclosed as a count, never as
    /// which variants they were — see security invariant 5.
    pub suppressed_instances: usize,
    /// The visible Pareto, most frequent first.
    pub variants: Vec<VariantSummary>,
    /// Per-position and `"overall"` duration quantiles.
    pub lines: Vec<LineStat>,
}

/// Aggregate a cohort of already-built [`InstanceVariant`]s. Pure: the
/// floor is a parameter, never read from the environment here — see
/// this module's own doc comment.
#[must_use]
pub fn summarize_variants(
    instance_variants: &[InstanceVariant],
    min_cell_count: usize,
) -> VariantsReport {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for iv in instance_variants {
        *counts.entry(iv.variant.clone()).or_default() += 1;
    }

    let floor = min_cell_count.max(1);
    let mut visible: Vec<(String, usize)> = Vec::new();
    let mut suppressed_instances = 0usize;
    for (variant, count) in counts {
        if count < floor {
            suppressed_instances += count;
        } else {
            visible.push((variant, count));
        }
    }
    visible.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let visible_total: usize = visible.iter().map(|(_, count)| *count).sum();

    let mut cumulative_share = 0.0;
    let variants: Vec<VariantSummary> = visible
        .into_iter()
        .map(|(variant, frequency)| {
            #[allow(clippy::cast_precision_loss)]
            let share = if visible_total > 0 {
                frequency as f64 / visible_total as f64
            } else {
                0.0
            };
            cumulative_share += share;
            VariantSummary {
                variant,
                frequency,
                share,
                cumulative_share,
            }
        })
        .collect();

    let mut by_position: BTreeMap<usize, Vec<i64>> = BTreeMap::new();
    let mut overall: Vec<i64> = Vec::new();
    for iv in instance_variants {
        let mut total_ms = 0i64;
        for (index, era) in iv.eras.iter().enumerate() {
            let duration_ms = (era.end_ms - era.start_ms).max(0);
            by_position.entry(index + 1).or_default().push(duration_ms);
            total_ms += duration_ms;
        }
        overall.push(total_ms);
    }
    let mut lines: Vec<LineStat> = by_position
        .into_iter()
        .map(|(position, mut ms)| {
            ms.sort_unstable();
            LineStat {
                position: position.to_string(),
                n: ms.len(),
                median_days: tba::percentile(&ms, 0.5).map_or(0.0, to_days),
                p90_days: tba::percentile(&ms, 0.9).map_or(0.0, to_days),
            }
        })
        .collect();
    overall.sort_unstable();
    lines.push(LineStat {
        position: "overall".to_string(),
        n: overall.len(),
        median_days: tba::percentile(&overall, 0.5).map_or(0.0, to_days),
        p90_days: tba::percentile(&overall, 0.9).map_or(0.0, to_days),
    });

    VariantsReport {
        instances: instance_variants.len(),
        suppressed_instances,
        variants,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn era(stage: &str, start_days: i64, end_days: i64) -> Era {
        Era {
            stage: stage.to_string(),
            start_ms: start_days * tba::DAY_MS,
            end_ms: end_days * tba::DAY_MS,
        }
    }

    // -----------------------------------------------------------------
    // combine_overlaps / decompose_pair
    // -----------------------------------------------------------------

    /// A genuine crossing overlap (`a` ends before `b`) is the FRFS
    /// shape and decomposes into exactly three pieces.
    #[test]
    fn frfs_overlap_becomes_three_intervals() {
        let eras = vec![era("triage", 0, 10), era("treatment", 5, 15)];
        let out = combine_overlaps(eras, 100.0); // below the window: a handoff, not a combination
        assert_eq!(out.len(), 3, "{out:?}");
        assert_eq!(out[0], era("triage", 0, 5));
        // The contested middle goes to the incoming stage under a handoff.
        assert_eq!(out[1].stage, "treatment");
        assert_eq!(
            (out[1].start_ms, out[1].end_ms),
            (5 * tba::DAY_MS, 10 * tba::DAY_MS)
        );
        assert_eq!(out[2], era("treatment", 10, 15));
    }

    /// A nested overlap (`b` fully inside `a`) is the LRFS shape and
    /// also decomposes into exactly three pieces — `a` resumes after
    /// `b` ends.
    #[test]
    fn lrfs_overlap_becomes_three_intervals() {
        let eras = vec![era("triage", 0, 20), era("treatment", 5, 10)];
        let out = combine_overlaps(eras, 100.0);
        assert_eq!(out.len(), 3, "{out:?}");
        assert_eq!(out[0], era("triage", 0, 5));
        assert_eq!(out[1].stage, "treatment");
        assert_eq!(
            (out[1].start_ms, out[1].end_ms),
            (5 * tba::DAY_MS, 10 * tba::DAY_MS)
        );
        assert_eq!(out[2], era("triage", 10, 20));
    }

    /// An overlap at least as long as the combination window becomes a
    /// canonical alphabetical `a+b` label, and the join is
    /// order-independent: `b+a` normalises to the same string as `a+b`.
    #[test]
    fn overlap_at_or_above_the_window_combines_canonically() {
        let eras = vec![era("triage", 0, 10), era("treatment", 5, 15)];
        let out = combine_overlaps(eras, 5.0); // the overlap is exactly 5 days
        let combo = out
            .iter()
            .find(|e| e.stage.contains('+'))
            .expect("a combination era");
        assert_eq!(
            combo.stage, "treatment+triage",
            "alphabetical, not arrival order"
        );
        assert_eq!(
            combined_label("treatment", "triage"),
            combined_label("triage", "treatment")
        );
    }

    /// A three-way overlap converges to `a+b+c` regardless of which
    /// pair the fixed-point iteration resolves first.
    #[test]
    fn three_way_overlap_converges_to_a_plus_b_plus_c() {
        let eras = vec![
            era("diagnostics", 0, 10),
            era("treatment", 2, 12),
            era("follow_up", 4, 14),
        ];
        let out = combine_overlaps(eras, 1.0);
        assert!(
            out.iter()
                .any(|e| e.stage == "diagnostics+follow_up+treatment"),
            "{out:?}"
        );
    }

    // -----------------------------------------------------------------
    // build_variant: the full pipeline
    // -----------------------------------------------------------------

    /// `min_segment_days` drops a short era outright.
    #[test]
    fn short_segments_are_dropped() {
        let segments = vec![
            segment("triage", 0, Some(1)),     // 1 day, dropped
            segment("treatment", 5, Some(10)), // 5 days, kept
        ];
        let params = VariantParams {
            min_segment_days: 2.0,
            collapse_gap_days: 0.0,
            combination_window_days: 0.0,
            min_post_combination_days: 0.0,
            ..VariantParams::default()
        };
        let variant = build_variant(&segments, 20 * tba::DAY_MS, &params);
        assert_eq!(variant.variant, "treatment");
    }

    /// A stub shorter than `min_post_combination_days` disappears after
    /// overlap resolution.
    #[test]
    fn a_post_combination_stub_disappears() {
        // triage[0,10) and treatment[3,15): a 7-day overlap combines
        // under a 5-day window into three pieces — triage-only[0,3)
        // (3 days), the combination[3,10) (7 days), treatment-only
        // [10,15) (5 days). A 4-day floor drops only the 3-day stub.
        let segments = vec![
            segment("triage", 0, Some(10)),
            segment("treatment", 3, Some(15)),
        ];
        let params = VariantParams {
            collapse_gap_days: 0.0,
            combination_window_days: 5.0,
            min_post_combination_days: 4.0,
            ..VariantParams::default()
        };
        let variant = build_variant(&segments, 20 * tba::DAY_MS, &params);
        assert_eq!(variant.eras.len(), 2, "{:?}", variant.eras);
        assert_eq!(variant.eras[0].stage, "treatment+triage");
        assert_eq!(variant.eras[1].stage, "treatment");
        assert!(
            variant.eras.iter().all(|e| e.stage != "triage"),
            "the solo triage stub must not survive: {:?}",
            variant.eras
        );
    }

    /// A combination era is exempt from the `min_post_combination_days`
    /// floor even when the combination itself is short: the floor
    /// names what is left *around* a combination, not the combination
    /// block itself.
    #[test]
    fn a_short_combination_era_survives_the_stub_floor() {
        let segments = vec![
            segment("triage", 0, Some(10)),
            segment("treatment", 5, Some(15)),
        ];
        let params = VariantParams {
            collapse_gap_days: 0.0,
            combination_window_days: 1.0, // the 5-day overlap combines
            min_post_combination_days: 30.0, // far longer than any piece here
            ..VariantParams::default()
        };
        let variant = build_variant(&segments, 20 * tba::DAY_MS, &params);
        assert_eq!(variant.eras.len(), 1, "{:?}", variant.eras);
        assert_eq!(variant.eras[0].stage, "treatment+triage");
    }

    /// `filter=changes` collapses `a-a-b` to `a-b`; `filter=all` keeps
    /// every entry.
    #[test]
    fn changes_collapses_consecutive_duplicates_all_does_not() {
        let segments = vec![
            segment("triage", 0, Some(1)),
            segment("triage", 3, Some(4)), // same stage, big gap: stays separate before filtering
            segment("treatment", 6, Some(7)),
        ];
        let base = VariantParams {
            collapse_gap_days: 0.0, // do not pre-collapse via the gap rule
            combination_window_days: 0.0,
            min_post_combination_days: 0.0,
            ..VariantParams::default()
        };

        let all_params = VariantParams {
            filter: FilterMode::All,
            ..base.clone()
        };
        let all = build_variant(&segments, 20 * tba::DAY_MS, &all_params);
        assert_eq!(all.variant, "triage-triage-treatment");

        let changes_params = VariantParams {
            filter: FilterMode::Changes,
            ..base
        };
        let changes = build_variant(&segments, 20 * tba::DAY_MS, &changes_params);
        assert_eq!(changes.variant, "triage-treatment");
    }

    /// `filter=first` keeps only each label's first occurrence, even a
    /// non-consecutive later repeat.
    #[test]
    fn first_keeps_only_each_labels_first_occurrence() {
        let segments = vec![
            segment("triage", 0, Some(1)),
            segment("treatment", 2, Some(3)),
            segment("triage", 4, Some(5)),
        ];
        let params = VariantParams {
            collapse_gap_days: 0.0,
            combination_window_days: 0.0,
            min_post_combination_days: 0.0,
            filter: FilterMode::First,
            ..VariantParams::default()
        };
        let variant = build_variant(&segments, 20 * tba::DAY_MS, &params);
        assert_eq!(variant.variant, "triage-treatment");
    }

    /// `max_path_length` truncates the final sequence.
    #[test]
    fn max_path_length_truncates() {
        let segments = vec![
            segment("referral", 0, Some(1)),
            segment("triage", 2, Some(3)),
            segment("treatment", 4, Some(5)),
        ];
        let params = VariantParams {
            collapse_gap_days: 0.0,
            combination_window_days: 0.0,
            min_post_combination_days: 0.0,
            max_path_length: 2,
            ..VariantParams::default()
        };
        let variant = build_variant(&segments, 20 * tba::DAY_MS, &params);
        assert_eq!(variant.variant, "referral-triage");
    }

    fn segment(stage: &str, start_days: i64, end_days: Option<i64>) -> SegmentInput {
        SegmentInput {
            stage: stage.to_string(),
            category: tba::CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms: start_days * tba::DAY_MS,
            end_ms: end_days.map(|d| d * tba::DAY_MS),
            actor_ref: None,
            location_ref: None,
        }
    }

    // -----------------------------------------------------------------
    // summarize_variants
    // -----------------------------------------------------------------

    fn instance_variant(variant: &str) -> InstanceVariant {
        InstanceVariant {
            variant: variant.to_string(),
            eras: vec![era("treatment", 0, 3)],
        }
    }

    /// Coverage sums to 1 over the unsuppressed variants, and the
    /// suppressed count is disclosed rather than hidden.
    #[test]
    fn coverage_sums_to_one_and_suppression_is_disclosed() {
        // "a" x6, "b" x3, "c" x1 (below a floor of 2 -> suppressed).
        let mut instances: Vec<InstanceVariant> = Vec::new();
        instances.extend((0..6).map(|_| instance_variant("a")));
        instances.extend((0..3).map(|_| instance_variant("b")));
        instances.push(instance_variant("c"));

        let report = summarize_variants(&instances, 2);
        assert_eq!(report.instances, 10);
        assert_eq!(report.suppressed_instances, 1, "the lone 'c' instance");
        assert_eq!(report.variants.len(), 2, "only a and b are visible");

        let total_share: f64 = report.variants.iter().map(|v| v.share).sum();
        assert!((total_share - 1.0).abs() < 1e-9, "{total_share}");
        let last_cumulative = report
            .variants
            .last()
            .expect("at least one variant")
            .cumulative_share;
        assert!((last_cumulative - 1.0).abs() < 1e-9, "{last_cumulative}");

        // Most frequent first.
        assert_eq!(report.variants[0].variant, "a");
        assert_eq!(report.variants[0].frequency, 6);
    }

    /// The `overall` pseudo-line sums every instance's whole journey;
    /// per-position lines report only the instances that reach them.
    #[test]
    fn lines_include_overall_and_per_position_quantiles() {
        let instances = vec![
            InstanceVariant {
                variant: "a-b".to_string(),
                eras: vec![era("a", 0, 2), era("b", 2, 5)],
            },
            InstanceVariant {
                variant: "a".to_string(),
                eras: vec![era("a", 0, 4)],
            },
        ];
        let report = summarize_variants(&instances, 1);
        let overall = report
            .lines
            .iter()
            .find(|l| l.position == "overall")
            .expect("overall line");
        assert_eq!(overall.n, 2);
        let position_1 = report
            .lines
            .iter()
            .find(|l| l.position == "1")
            .expect("position 1");
        assert_eq!(position_1.n, 2, "both instances have a first era");
        let position_2 = report.lines.iter().find(|l| l.position == "2");
        assert!(
            position_2.is_some_and(|l| l.n == 1),
            "only the first instance has a second era"
        );
    }
}
