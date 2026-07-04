//! `MatchingEngine` — the public entry point.
//!
//! The algorithm has two phases:
//!
//! 1. **Deterministic short-circuit.** If both records share a value
//!    on a deterministic identifier scheme (DOI, Wikidata, LOM, OER,
//!    URI, UUID) OR share `provider_id` + normalised `course_code` OR
//!    overlap on a `same_as` URL, return score `1.0`.
//! 2. **Probabilistic scoring.** Compute per-component scores; take a
//!    weighted average over the *present* components.
//!
//! The full per-component formula lives in
//! `AGENTS/matching-algorithm.md`.

use strsim::jaro_winkler;

use crate::config::MatchConfig;
use crate::course::{Course, CourseIdentifier, EducationalLevel};
use crate::normalize;
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

/// Soundex bonus applied to `name_score` when the two name codes
/// match and the underlying Jaro-Winkler hasn't already cleared the
/// High-confidence band (≥ 0.95). The cap at 0.95 means a phonetic
/// match nudges a Medium-band score upward but never single-handedly
/// classifies a record pair as High confidence.
const PHONETIC_BONUS: f64 = 0.05;
const PHONETIC_CEILING: f64 = 0.95;

/// The course matcher: holds a [`MatchConfig`] and scores course pairs.
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::{MatchConfig, MatchingEngine};
    ///
    /// let engine = MatchingEngine::new(MatchConfig::strict());
    /// assert_eq!(engine.config().threshold, 0.95);
    /// ```
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with `MatchConfig::default()`. Convenience for the common path.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    #[must_use]
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two courses. Always returns a result (never errs).
    ///
    /// Runs the deterministic short-circuit first; on a miss it falls
    /// back to the renormalised probabilistic weighted average.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::{Course, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = Course::new("Introduction to Computer Science");
    /// let b = Course::new("Intro to Computer Science");
    /// let result = engine.match_courses(&a, &b);
    /// assert!((0.0..=1.0).contains(&result.score));
    /// ```
    #[must_use]
    pub fn match_courses(&self, a: &Course, b: &Course) -> MatchResult {
        // ── Deterministic short-circuit ──────────────────────────
        if deterministic_match(a, b) {
            return MatchResult {
                score: 1.0,
                is_match: true,
                confidence: Confidence::High,
                breakdown: MatchBreakdown {
                    deterministic_match: true,
                    ..Default::default()
                },
            };
        }

        // ── Probabilistic scoring ────────────────────────────────
        let name_score = Some(name_score(a, b));
        let course_code_score = course_code_score(a, b);
        let provider_score = provider_score(a, b);
        let educational_level_score = educational_level_score(a, b);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);
        let teaches_score = set_jaccard(&a.teaches, &b.teaches);

        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (course_code_score, self.config.course_code_weight),
            (provider_score, self.config.provider_weight),
            (
                educational_level_score,
                self.config.educational_level_weight,
            ),
            (keywords_score, self.config.keywords_weight),
            (teaches_score, self.config.teaches_weight),
        ]);

        let is_match = score >= self.config.threshold;
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: MatchBreakdown {
                name_score,
                course_code_score,
                provider_score,
                educational_level_score,
                keywords_score,
                teaches_score,
                deterministic_match: false,
            },
        }
    }

    /// One-to-many: score `query` against each `candidate` and return
    /// results **in the same order as `candidates`** (no rank, no
    /// filter). Mirrors `person_matcher::MatchingEngine::match_one_to_many`
    /// so callers that work across the matcher family share a single
    /// call shape.
    ///
    /// Use [`MatchingEngine::rank`] when you want the results sorted
    /// by descending score, or [`MatchingEngine::find_matches`] when
    /// you also want the filter applied.
    #[must_use]
    pub fn match_one_to_many(&self, query: &Course, candidates: &[Course]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_courses(query, c))
            .collect()
    }

    /// One-to-many: score `query` against each `candidate`, return
    /// `(index, result)` sorted by score descending. The original
    /// candidate index is preserved in the tuple so callers can map
    /// back to their input slice after sorting.
    #[must_use]
    pub fn rank(&self, query: &Course, candidates: &[Course]) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_courses(query, c)))
            .collect();
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Convenience filter: rank then drop everything below
    /// `MatchConfig::threshold`.
    #[must_use]
    pub fn find_matches(&self, query: &Course, candidates: &[Course]) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

/// Decide whether the two courses share a globally-unique identity
/// signal that pins the score to `1.0` without scoring.
///
/// Returns `true` if any of three rules fire:
///
/// - **R-0** — both carry the same value under the same *deterministic*
///   identifier scheme (DOI / Wikidata / LOM / OER / URI / UUID). These
///   schemes are unique by construction, so an equal value is identity.
/// - **R-1** — both share a non-empty `provider_id` AND a normalised
///   `course_code`. Course codes are only unique *within* a provider,
///   so the provider must match too (CS101 differs between schools).
/// - **R-2** — their `same_as` URL sets overlap after folding. A shared
///   cross-system URL (Wikidata page, OER entry) is an asserted identity.
///
/// Empty / blank values are skipped throughout so two absent
/// identifiers never collide into a false positive — the worst-case
/// bug for a rule that pins to `1.0`.
fn deterministic_match(a: &Course, b: &Course) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value.
    for ai in &a.identifiers {
        // Provider-scoped schemes (CourseCode, LmsCourseId, …) are not
        // globally unique, so they never trigger R-0.
        if !ai.scheme.is_deterministic() {
            continue;
        }
        let av = normalize::fold(&ai.value);
        // A blank value is not an identity — skip it so "" != "" here.
        if av.is_empty() {
            continue;
        }
        for bi in &b.identifiers {
            // Same scheme AND same folded value ⇒ deterministic identity.
            if ai.scheme == bi.scheme && av == normalize::fold(&bi.value) {
                return true;
            }
        }
    }

    // R-1 — same provider + same normalised course_code. The `let …
    // && …` chain requires all four fields present before comparing;
    // `!ap.is_empty()` guards against two records that both carry a
    // blank provider id accidentally matching.
    if let (Some(ap), Some(bp), Some(ac), Some(bc)) = (
        a.provider_id.as_deref(),
        b.provider_id.as_deref(),
        a.course_code.as_deref(),
        b.course_code.as_deref(),
    ) && !ap.is_empty()
        && ap == bp
        && normalize::course_code(ac) == normalize::course_code(bc)
    {
        return true;
    }

    // R-2 — any same_as URL overlaps (case-folded host+path).
    for au in &a.same_as {
        let an = normalize::fold(au);
        // Skip blanks so two empty same_as entries don't "overlap".
        if an.is_empty() {
            continue;
        }
        for bu in &b.same_as {
            if an == normalize::fold(bu) {
                return true;
            }
        }
    }

    false
}

// ─── Probabilistic components ────────────────────────────────────

/// Best name similarity in `[0.0, 1.0]` across primary + alternate
/// names, with an optional Soundex bonus.
///
/// Why "best across alternates": a course can be catalogued under
/// several titles (`schema.org/alternateName`), and a hit on any of
/// them is evidence of the same course. We therefore take the maximum
/// Jaro-Winkler over the primary-vs-primary pair plus each side's
/// alternates against the other's primary.
///
/// The phonetic bonus (`PHONETIC_BONUS`) is added only when the two
/// primary names share a Soundex code and the score is still below
/// `PHONETIC_CEILING`, so a homophone (`Smyth`/`Smith`) is nudged up
/// but a phonetic coincidence can never single-handedly mint a
/// High-confidence match.
///
/// Always returns a value (name is the one component both records
/// always carry), so the caller wraps it in `Some` unconditionally.
fn name_score(a: &Course, b: &Course) -> f64 {
    let an = normalize::fold(&a.name);
    let bn = normalize::fold(&b.name);
    // Baseline: folded primary names compared via Jaro-Winkler.
    let mut best = jaro_winkler(&an, &bn);
    // Each of a's alternate titles gets a shot against b's primary;
    // keep the best score seen so far.
    for alt in &a.alternate_names {
        let alt_n = normalize::fold(alt);
        best = best.max(jaro_winkler(&alt_n, &bn));
    }
    // Symmetric: each of b's alternates against a's primary.
    for alt in &b.alternate_names {
        let alt_n = normalize::fold(alt);
        best = best.max(jaro_winkler(&an, &alt_n));
    }
    // T-6: Soundex bonus on the primary names. Capped so a phonetic
    // hit can lift a Medium-band score but not single-handedly mint a
    // High-confidence match.
    if best < PHONETIC_CEILING && phonetic::same(&an, &bn) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

/// Same-provider course-code similarity, or `None` when the component
/// can't be scored.
///
/// Returns:
/// - `None` when either side lacks a (non-empty) course code, or when
///   the two records don't share a provider — across providers a code
///   like `CS101` is noise, so it is *skipped* (not scored zero) to
///   avoid diluting the weighted average with a meaningless signal.
/// - `Some(1.0)` when same provider and the normalised codes are equal.
/// - `Some(0.0)` when same provider but the codes differ.
fn course_code_score(a: &Course, b: &Course) -> Option<f64> {
    let (ac, bc) = (a.course_code.as_deref(), b.course_code.as_deref());
    let (ap, bp) = (a.provider_id.as_deref(), b.provider_id.as_deref());

    // Need a non-empty course code on both sides, else skip the component.
    let (ac, bc) = match (ac, bc) {
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };

    // Across-provider course codes are noise (CS101 exists at every
    // university). Only contribute when both records share a provider.
    match (ap, bp) {
        (Some(ap), Some(bp)) if !ap.is_empty() && ap == bp => {
            if normalize::course_code(ac) == normalize::course_code(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        _ => None,
    }
}

/// Provider similarity, or `None` when neither record names a provider.
///
/// Prefers the structured `provider_id`: when both carry a non-empty
/// id the result is a crisp `1.0` (equal) or `0.0` (differ). When no
/// usable id is present it falls back to fuzzy Jaro-Winkler on the
/// free-text `provider_name` (so "Stanford University" ≈ "stanford
/// university"). Returns `None` only when there is no provider signal
/// at all, so the component is skipped rather than scored zero.
fn provider_score(a: &Course, b: &Course) -> Option<f64> {
    // Structured id is authoritative: exact equality decides 1.0 / 0.0.
    if let (Some(ap), Some(bp)) = (a.provider_id.as_deref(), b.provider_id.as_deref())
        && !ap.is_empty()
    {
        return Some(if ap == bp { 1.0 } else { 0.0 });
    }
    // No id ⇒ fall back to fuzzy matching on the provider's display name.
    match (a.provider_name.as_deref(), b.provider_name.as_deref()) {
        (Some(an), Some(bn)) if !an.is_empty() && !bn.is_empty() => {
            Some(jaro_winkler(&normalize::fold(an), &normalize::fold(bn)))
        }
        _ => None,
    }
}

/// Educational-level similarity, or `None` when either side is unset.
///
/// Levels rarely match exactly across catalogues, so adjacency earns
/// partial credit: identical levels score `1.0`, levels one rung apart
/// on the same ladder (e.g. Beginner↔Intermediate) score `0.5`, and
/// everything else scores `0.0`. The half-credit reflects that an
/// "Intermediate" and "Beginner" listing of the same subject are
/// plausibly the same offering re-graded, whereas two rungs apart is
/// most likely a genuinely different course.
fn educational_level_score(a: &Course, b: &Course) -> Option<f64> {
    // Both sides must declare a level; otherwise skip the component.
    let (Some(al), Some(bl)) = (&a.educational_level, &b.educational_level) else {
        return None;
    };
    Some(if al == bl {
        1.0
    } else if educational_level_one_off(al, bl) {
        // Adjacent on a ladder ⇒ half credit.
        0.5
    } else {
        0.0
    })
}

/// `true` when `a` and `b` sit exactly one rung apart on the same
/// educational ladder.
///
/// Three independent ladders are recognised — a generic skill ladder,
/// a schooling-stage ladder, and a degree-level ladder. A pair counts
/// as "one off" only when both levels appear on the *same* ladder and
/// their indices differ by 1. Levels on different ladders (or two-plus
/// rungs apart) are not adjacent, so the caller scores them `0.0`.
fn educational_level_one_off(a: &EducationalLevel, b: &EducationalLevel) -> bool {
    use EducationalLevel::{
        Advanced, Beginner, Expert, Graduate, HigherEducation, Intermediate, Postgraduate,
        PrimaryEducation, SecondaryEducation, Undergraduate,
    };
    // Each ladder lists adjacent levels; a pair is "one off" when both
    // sit on the same ladder exactly one rung apart.
    let ladders: [&[EducationalLevel]; 3] = [
        &[Beginner, Intermediate, Advanced, Expert],
        &[PrimaryEducation, SecondaryEducation, HigherEducation],
        &[Undergraduate, Graduate, Postgraduate],
    ];
    ladders.iter().any(|ladder| {
        match (
            ladder.iter().position(|x| x == a),
            ladder.iter().position(|x| x == b),
        ) {
            (Some(ai), Some(bi)) => ai.abs_diff(bi) == 1,
            _ => false,
        }
    })
}

/// Jaccard similarity of two string sets (used for keywords and
/// teaches), or `None` when neither side has any items.
///
/// Jaccard = |intersection| / |union| after folding/deduping both
/// sides. Semantics of the edge cases:
/// - both empty (before or after folding) ⇒ `None`, so the component
///   is skipped, not scored — there is simply no evidence.
/// - exactly one side empty ⇒ `Some(0.0)`: one course tags keywords,
///   the other doesn't; that is weak negative evidence, scored zero.
/// - otherwise the true Jaccard ratio in `[0.0, 1.0]`.
fn set_jaccard(a: &[String], b: &[String]) -> Option<f64> {
    // Cheap pre-check before allocating the folded sets.
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    // After folding both could collapse to empty (all blanks) ⇒ skip.
    if a_set.is_empty() && b_set.is_empty() {
        return None;
    }
    // Exactly one side has tags ⇒ zero overlap is meaningful evidence.
    if a_set.is_empty() || b_set.is_empty() {
        return Some(0.0);
    }
    // Sets are sorted+deduped, so `contains` counts each shared item once.
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    // |A ∪ B| = |A| + |B| − |A ∩ B|.
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        Some(0.0)
    } else {
        // Set sizes are small bounded counts; convert losslessly via u32
        // (saturating at u32::MAX, which these counts never approach).
        let inter_f64 = f64::from(u32::try_from(inter).unwrap_or(u32::MAX));
        let union_f64 = f64::from(u32::try_from(union).unwrap_or(u32::MAX));
        Some(inter_f64 / union_f64)
    }
}

// Anonymous (`as _`) import of `CourseIdentifier`: it keeps the type
// in scope for trait/path resolution without binding a usable name,
// so it cannot collide with anything. `#[allow(unused_imports)]`
// silences the lint since the binding is intentionally name-less.
#[allow(unused_imports)]
use CourseIdentifier as _;

// ─── Tests ───────────────────────────────────────────────────────

/// Unit tests for the engine surface, deterministic short-circuits,
/// and each probabilistic component function.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::IdentifierScheme;

    /// Test helper: build a `CourseIdentifier` from a scheme + value.
    fn ident(scheme: IdentifierScheme, value: &str) -> crate::CourseIdentifier {
        crate::CourseIdentifier {
            scheme,
            value: value.into(),
        }
    }

    // Pins that two byte-identical courses score ≈1.0 and match.
    #[test]
    fn identical_courses_score_1() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("CS101 Introduction to Computer Science");
        let b = Course::new("CS101 Introduction to Computer Science");
        let r = engine.match_courses(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert!(r.is_match);
    }

    // Pins R-0: a shared DOI pins to 1.0 even when names are unrelated.
    #[test]
    fn doi_match_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("A");
        let mut b = Course::new("Completely different");
        a.identifiers
            .push(ident(IdentifierScheme::Doi, "10.1234/abc"));
        b.identifiers
            .push(ident(IdentifierScheme::Doi, "10.1234/abc"));
        let r = engine.match_courses(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    // Pins R-1: same provider + normalised course codes (`cs101` vs
    // `CS 101`) pin to 1.0 despite differing titles.
    #[test]
    fn same_provider_course_code_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("Intro CS");
        let mut b = Course::new("Introduction to Computer Science");
        a.provider_id = Some("prov-1".into());
        b.provider_id = Some("prov-1".into());
        a.course_code = Some("cs101".into());
        b.course_code = Some("CS 101".into());
        let r = engine.match_courses(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    // Pins that two unrelated titles score well below 0.5 and don't match.
    #[test]
    fn unrelated_courses_score_low() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("Linear Algebra");
        let b = Course::new("Tudor History 1485-1603");
        let r = engine.match_courses(&a, &b);
        assert!(r.score < 0.5, "expected low score, got {}", r.score);
        assert!(!r.is_match);
    }

    #[test]
    fn rename_typo_matches() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("Introduction to Computer Science");
        let b = Course::new("Intro to Computer Science");
        let r = engine.match_courses(&a, &b);
        // Name-only weighted-average should still be the Jaro-Winkler
        // similarity, which is high but won't always cross 0.85 on
        // name alone. Just assert directionality.
        assert!(r.score > 0.80, "expected > 0.80, got {}", r.score);
    }

    #[test]
    fn phonetic_bonus_lifts_homophone_pair() {
        // Tokens with the same Soundex code but a small Jaro-Winkler
        // gap: the bonus should push `name_score` up by exactly
        // PHONETIC_BONUS, clamped at PHONETIC_CEILING.
        let a = Course::new("Smyth");
        let b = Course::new("Smith");
        let with_bonus = name_score(&a, &b);
        // Without the bonus, baseline Jaro-Winkler is < 0.95.
        let base = strsim::jaro_winkler("smyth", "smith");
        assert!(base < 0.95);
        let expected = (base + 0.05_f64).min(0.95);
        assert!(
            (with_bonus - expected).abs() < 1e-9,
            "expected {expected}, got {with_bonus} (base {base})"
        );
    }

    #[test]
    fn phonetic_bonus_does_not_fire_on_non_homophones() {
        // Different Soundex codes → bonus must NOT fire.
        let a = Course::new("Jones");
        let b = Course::new("Smith");
        let with = name_score(&a, &b);
        let base = strsim::jaro_winkler("jones", "smith");
        assert!(
            (with - base).abs() < 1e-9,
            "expected base {base}, got {with}"
        );
    }

    #[test]
    fn phonetic_bonus_capped_at_ceiling() {
        // Two names that already score very high — bonus should not
        // push the result over the cap.
        let a = Course::new("CourseScience");
        let b = Course::new("CourceScience"); // tiny typo, Soundex same
        let with = name_score(&a, &b);
        assert!(with <= 0.95 + f64::EPSILON);
    }

    // Pins that one-to-many returns results in candidate order (no sort).
    #[test]
    fn match_one_to_many_preserves_input_order() {
        let engine = MatchingEngine::default_config();
        let query = Course::new("CS101");
        let cands = vec![
            Course::new("HIS200"),
            Course::new("CS101 Introduction"),
            Course::new("CS101"),
        ];
        let out = engine.match_one_to_many(&query, &cands);
        assert_eq!(out.len(), 3);
        // Exact match should be the third entry — preserves input order.
        assert!(out[2].score >= out[1].score);
        assert!(out[1].score >= out[0].score);
    }

    // Pins the empty-candidate edge case for one-to-many.
    #[test]
    fn match_one_to_many_empty_input_returns_empty() {
        let engine = MatchingEngine::default_config();
        let query = Course::new("Anything");
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
    }

    // Pins that `rank` sorts by descending score (exact match first).
    #[test]
    fn rank_orders_by_score() {
        let engine = MatchingEngine::default_config();
        let query = Course::new("CS101");
        let cands = vec![
            Course::new("HIS200"),
            Course::new("CS101 Introduction"),
            Course::new("CS101"),
        ];
        let ranked = engine.rank(&query, &cands);
        assert_eq!(ranked[0].0, 2); // exact match wins
    }

    // ─── Deterministic short-circuits ────────────────────────────

    #[test]
    fn same_as_url_overlap_short_circuits() {
        // R-2: a shared same_as URL pins the score to 1.0 even when
        // names are unrelated. Case + surrounding whitespace fold away.
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("Alpha");
        let mut b = Course::new("Omega");
        a.same_as = vec!["https://www.Wikidata.org/wiki/Q42".into()];
        b.same_as = vec!["  https://www.wikidata.org/wiki/Q42  ".into()];
        let r = engine.match_courses(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
        assert_eq!(r.confidence, Confidence::High);
    }

    #[test]
    fn non_deterministic_scheme_does_not_short_circuit() {
        // A shared CourseCode *identifier* is provider-scoped and must
        // NOT pin to 1.0 — CS101 is not globally unique.
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("History of Rome");
        let mut b = Course::new("Organic Chemistry");
        a.identifiers
            .push(ident(IdentifierScheme::CourseCode, "CS101"));
        b.identifiers
            .push(ident(IdentifierScheme::CourseCode, "CS101"));
        let r = engine.match_courses(&a, &b);
        // A provider-scoped identifier never pins the score to 1.0, and
        // these unrelated names fall well short of the match threshold.
        assert!(!r.breakdown.deterministic_match);
        assert!(!r.is_match, "should not be a match, got {}", r.score);
    }

    #[test]
    fn deterministic_scheme_must_match_value_to_short_circuit() {
        // Same scheme, different value → no short-circuit.
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("X");
        let mut b = Course::new("X");
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1/aaa"));
        b.identifiers.push(ident(IdentifierScheme::Doi, "10.1/bbb"));
        let r = engine.match_courses(&a, &b);
        assert!(!r.breakdown.deterministic_match);
    }

    #[test]
    fn deterministic_match_ignores_empty_identifier_values() {
        // Two empty DOI values must not be treated as "equal" and
        // short-circuit.
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.identifiers.push(ident(IdentifierScheme::Doi, "   "));
        b.identifiers.push(ident(IdentifierScheme::Doi, ""));
        assert!(!deterministic_match(&a, &b));
    }

    #[test]
    fn r1_needs_both_provider_and_course_code() {
        // Same course_code but no provider_id → R-1 does not fire.
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.course_code = Some("CS101".into());
        b.course_code = Some("cs 101".into());
        assert!(!deterministic_match(&a, &b));
        // Add a shared provider → now it fires.
        a.provider_id = Some("prov-1".into());
        b.provider_id = Some("prov-1".into());
        assert!(deterministic_match(&a, &b));
    }

    // ─── Component functions ─────────────────────────────────────

    #[test]
    fn course_code_skipped_across_different_providers() {
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.provider_id = Some("prov-1".into());
        b.provider_id = Some("prov-2".into());
        a.course_code = Some("CS101".into());
        b.course_code = Some("CS101".into());
        // Different providers → the component is skipped entirely.
        assert_eq!(course_code_score(&a, &b), None);
    }

    #[test]
    fn course_code_zero_when_same_provider_codes_differ() {
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.provider_id = Some("prov-1".into());
        b.provider_id = Some("prov-1".into());
        a.course_code = Some("CS101".into());
        b.course_code = Some("MATH220".into());
        assert_eq!(course_code_score(&a, &b), Some(0.0));
    }

    #[test]
    fn course_code_none_when_one_side_missing() {
        let mut a = Course::new("A");
        let b = Course::new("B");
        a.course_code = Some("CS101".into());
        assert_eq!(course_code_score(&a, &b), None);
    }

    #[test]
    fn provider_score_exact_id() {
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.provider_id = Some("ror-1".into());
        b.provider_id = Some("ror-1".into());
        assert_eq!(provider_score(&a, &b), Some(1.0));
        b.provider_id = Some("ror-2".into());
        assert_eq!(provider_score(&a, &b), Some(0.0));
    }

    #[test]
    fn provider_score_falls_back_to_name_when_no_id() {
        let mut a = Course::new("A");
        let mut b = Course::new("B");
        a.provider_name = Some("Stanford University".into());
        b.provider_name = Some("stanford university".into());
        // Case-folded identical names → Jaro-Winkler 1.0.
        assert_eq!(provider_score(&a, &b), Some(1.0));
    }

    #[test]
    fn provider_score_none_when_no_provider_info() {
        let a = Course::new("A");
        let b = Course::new("B");
        assert_eq!(provider_score(&a, &b), None);
    }

    #[test]
    fn educational_level_exact_one_off_and_unrelated() {
        use EducationalLevel::{Advanced, Beginner, Expert, Intermediate, Undergraduate};
        let mut a = Course::new("A");
        let mut b = Course::new("B");

        a.educational_level = Some(Beginner);
        b.educational_level = Some(Beginner);
        assert_eq!(educational_level_score(&a, &b), Some(1.0));

        b.educational_level = Some(Intermediate); // one rung up the skill ladder
        assert_eq!(educational_level_score(&a, &b), Some(0.5));

        b.educational_level = Some(Advanced); // two rungs apart
        assert_eq!(educational_level_score(&a, &b), Some(0.0));

        // Different ladders never count as "one off".
        a.educational_level = Some(Expert);
        b.educational_level = Some(Undergraduate);
        assert_eq!(educational_level_score(&a, &b), Some(0.0));
    }

    #[test]
    fn educational_level_none_when_either_side_unset() {
        let mut a = Course::new("A");
        let b = Course::new("B");
        a.educational_level = Some(EducationalLevel::Graduate);
        assert_eq!(educational_level_score(&a, &b), None);
    }

    // Pins that the off-ladder variants — Vocational,
    // ProfessionalDevelopment, and Custom(_) — never earn adjacency
    // credit. They sit on no ladder (spec §12 lists only the three
    // ladders), so a non-identical pairing involving them must score
    // 0.0, and an identical pairing must still score the exact-match
    // 1.0. This guards against one of them being silently wired into a
    // ladder later and minting spurious partial credit.
    #[test]
    fn educational_level_off_ladder_variants_score_zero_unless_identical() {
        use EducationalLevel::{Beginner, Custom, Expert, ProfessionalDevelopment, Vocational};
        let mut a = Course::new("A");
        let mut b = Course::new("B");

        // Identical off-ladder variant ⇒ exact-match 1.0.
        a.educational_level = Some(Vocational);
        b.educational_level = Some(Vocational);
        assert_eq!(educational_level_score(&a, &b), Some(1.0));
        a.educational_level = Some(ProfessionalDevelopment);
        b.educational_level = Some(ProfessionalDevelopment);
        assert_eq!(educational_level_score(&a, &b), Some(1.0));
        a.educational_level = Some(Custom("apprenticeship".into()));
        b.educational_level = Some(Custom("apprenticeship".into()));
        assert_eq!(educational_level_score(&a, &b), Some(1.0));

        // Off-ladder variant vs anything different ⇒ 0.0 (no adjacency).
        for (left, right) in [
            (Vocational, Beginner),
            (Vocational, ProfessionalDevelopment),
            (ProfessionalDevelopment, Expert),
            (Custom("apprenticeship".into()), Vocational),
            (Custom("apprenticeship".into()), Custom("internship".into())),
        ] {
            a.educational_level = Some(left.clone());
            b.educational_level = Some(right.clone());
            assert_eq!(
                educational_level_score(&a, &b),
                Some(0.0),
                "{left:?} vs {right:?} must score 0.0"
            );
        }
    }

    // Pins that `educational_level_one_off` returns false for every
    // off-ladder variant — they appear on none of the three ladders.
    #[test]
    fn educational_level_one_off_false_for_off_ladder_variants() {
        use EducationalLevel::{Beginner, Custom, ProfessionalDevelopment, Vocational};
        assert!(!educational_level_one_off(&Vocational, &Beginner));
        assert!(!educational_level_one_off(
            &ProfessionalDevelopment,
            &Beginner
        ));
        assert!(!educational_level_one_off(
            &Custom("apprenticeship".into()),
            &Beginner
        ));
        // Two off-ladder variants are never adjacent either.
        assert!(!educational_level_one_off(
            &Vocational,
            &ProfessionalDevelopment
        ));
    }

    // Pins that `learning_resource_type` carries NO scoring weight: it is
    // modelled on `Course` (spec §6) but deliberately excluded from the
    // algorithm. Two otherwise-identical inputs that differ ONLY in
    // `learning_resource_type` must score identically, and the type must
    // never surface in the breakdown (there is no such component).
    #[test]
    fn learning_resource_type_does_not_affect_score() {
        use crate::course::LearningResourceType;
        let engine = MatchingEngine::default_config();

        let mut a = Course::new("Introduction to Computer Science");
        a.keywords = vec!["programming".into()];
        a.learning_resource_type = Some(LearningResourceType::Lecture);

        let mut b = a.clone();
        b.learning_resource_type = Some(LearningResourceType::Workshop);

        let mut baseline = a.clone();
        baseline.learning_resource_type = None;

        let differing = engine.match_courses(&a, &b);
        let absent = engine.match_courses(&a, &baseline);
        // Differing types, type set on one side only, or type absent
        // everywhere all yield the same score — it is not an input.
        assert!((differing.score - absent.score).abs() < 1e-12);
    }

    // Pins that `in_language` is ignored by scoring (cross-language
    // matching is out of scope per spec §2.2 — callers use `same_as`).
    // Two otherwise-identical inputs differing only in `in_language`
    // must score identically.
    #[test]
    fn in_language_does_not_affect_score() {
        let engine = MatchingEngine::default_config();

        let mut a = Course::new("Introduction to Computer Science");
        a.keywords = vec!["programming".into()];
        a.in_language = vec!["en".into()];

        let mut b = a.clone();
        b.in_language = vec!["fr".into(), "de".into()];

        let mut baseline = a.clone();
        baseline.in_language = vec![];

        let differing = engine.match_courses(&a, &b);
        let absent = engine.match_courses(&a, &baseline);
        assert!((differing.score - absent.score).abs() < 1e-12);
    }

    #[test]
    fn jaccard_exact_fraction() {
        let a = vec!["alpha".to_string(), "beta".to_string()];
        let b = vec!["beta".to_string(), "gamma".to_string()];
        // intersection {beta}=1, union {alpha,beta,gamma}=3 → 1/3.
        let got = set_jaccard(&a, &b).expect("some");
        assert!((got - 1.0 / 3.0).abs() < 1e-9, "got {got}");
    }

    #[test]
    fn jaccard_skipped_when_both_empty() {
        assert_eq!(set_jaccard(&[], &[]), None);
    }

    #[test]
    fn jaccard_zero_when_only_one_side_has_items() {
        let a = vec!["alpha".to_string()];
        assert_eq!(set_jaccard(&a, &[]), Some(0.0));
        assert_eq!(set_jaccard(&[], &a), Some(0.0));
    }

    #[test]
    fn jaccard_is_case_and_whitespace_insensitive() {
        let a = vec![" Alpha ".to_string(), "BETA".to_string()];
        let b = vec!["alpha".to_string(), "beta".to_string()];
        assert_eq!(set_jaccard(&a, &b), Some(1.0));
    }

    // ─── Name scoring ────────────────────────────────────────────

    #[test]
    fn name_score_considers_alternate_names() {
        // Primary names disagree, but an alternate on `a` matches `b`.
        let mut a = Course::new("Intro to Programming");
        a.alternate_names = vec!["Data Structures".into()];
        let b = Course::new("Data Structures");
        let with_alt = name_score(&a, &b);
        let without_alt = name_score(&Course::new("Intro to Programming"), &b);
        assert!(
            with_alt > without_alt,
            "alternate name should raise the score: {with_alt} vs {without_alt}"
        );
        assert!(with_alt >= 0.99);
    }

    #[test]
    fn name_score_handles_diacritics() {
        // Identical diacritic strings must round-trip to a perfect score.
        let a = Course::new("Élève Café Über");
        let b = Course::new("Élève Café Über");
        assert!((name_score(&a, &b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn empty_names_do_not_panic() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("");
        let b = Course::new("");
        let r = engine.match_courses(&a, &b);
        assert!((0.0..=1.0).contains(&r.score));
    }

    // ─── Renormalisation & engine surface ────────────────────────

    #[test]
    fn renormalisation_ignores_absent_components() {
        // Only name + provider_id present on both sides; both identical.
        // The score must renormalise to 1.0, not be diluted by the
        // absent components' weights.
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("Quantum Mechanics");
        let mut b = Course::new("Quantum Mechanics");
        a.provider_id = Some("prov-9".into());
        b.provider_id = Some("prov-9".into());
        let r = engine.match_courses(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert!(r.breakdown.name_score.is_some());
        assert_eq!(r.breakdown.provider_score, Some(1.0));
        // Absent components stay None in the breakdown.
        assert!(r.breakdown.educational_level_score.is_none());
        assert!(r.breakdown.keywords_score.is_none());
    }

    #[test]
    fn find_matches_filters_below_threshold() {
        let engine = MatchingEngine::default_config();
        let query = Course::new("Organic Chemistry");
        let cands = vec![
            Course::new("Organic Chemistry"), // exact → passes
            Course::new("Medieval Poetry"),   // unrelated → filtered
        ];
        let matches = engine.find_matches(&query, &cands);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, 0);
        assert!(matches[0].1.is_match);
    }

    #[test]
    fn strict_config_rejects_a_merely_probable_match() {
        // A pair that clears the default 0.85 threshold but not 0.95.
        let lenient = MatchingEngine::new(MatchConfig::default());
        let strict = MatchingEngine::new(MatchConfig::strict());
        let a = Course::new("Introduction to Computer Science");
        let b = Course::new("Introduction to Computer Sciences");
        let rl = lenient.match_courses(&a, &b);
        let rs = strict.match_courses(&a, &b);
        // Same score, different is_match because the threshold moved.
        assert!((rl.score - rs.score).abs() < 1e-9);
        if rl.score >= 0.85 && rl.score < 0.95 {
            assert!(rl.is_match);
            assert!(!rs.is_match);
        }
    }
}
