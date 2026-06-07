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
use crate::scoring::{weighted_average, Confidence, MatchBreakdown, MatchResult};

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
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with `MatchConfig::default()`. Convenience for the common path.
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two courses. Always returns a result (never errs).
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
            (educational_level_score, self.config.educational_level_weight),
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
    pub fn match_one_to_many(&self, query: &Course, candidates: &[Course]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_courses(query, c))
            .collect()
    }

    /// One-to-many: score `query` against each `candidate`, return
    /// `(index, result)` sorted by score descending.
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
    pub fn find_matches(&self, query: &Course, candidates: &[Course]) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

fn deterministic_match(a: &Course, b: &Course) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value.
    for ai in &a.identifiers {
        if !ai.scheme.is_deterministic() {
            continue;
        }
        let av = normalize::fold(&ai.value);
        if av.is_empty() {
            continue;
        }
        for bi in &b.identifiers {
            if ai.scheme == bi.scheme && av == normalize::fold(&bi.value) {
                return true;
            }
        }
    }

    // R-1 — same provider + same normalised course_code.
    if let (Some(ap), Some(bp), Some(ac), Some(bc)) = (
        a.provider_id.as_deref(),
        b.provider_id.as_deref(),
        a.course_code.as_deref(),
        b.course_code.as_deref(),
    ) {
        if !ap.is_empty() && ap == bp && normalize::course_code(ac) == normalize::course_code(bc) {
            return true;
        }
    }

    // R-2 — any same_as URL overlaps (case-folded host+path).
    for au in &a.same_as {
        let an = normalize::fold(au);
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

fn name_score(a: &Course, b: &Course) -> f64 {
    let an = normalize::fold(&a.name);
    let bn = normalize::fold(&b.name);
    let mut best = jaro_winkler(&an, &bn);
    // Try alternate names from both sides too.
    for alt in &a.alternate_names {
        let alt_n = normalize::fold(alt);
        best = best.max(jaro_winkler(&alt_n, &bn));
    }
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

fn course_code_score(a: &Course, b: &Course) -> Option<f64> {
    let (ac, bc) = (a.course_code.as_deref(), b.course_code.as_deref());
    let (ap, bp) = (a.provider_id.as_deref(), b.provider_id.as_deref());

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

fn provider_score(a: &Course, b: &Course) -> Option<f64> {
    if let (Some(ap), Some(bp)) = (a.provider_id.as_deref(), b.provider_id.as_deref()) {
        if !ap.is_empty() {
            return Some(if ap == bp { 1.0 } else { 0.0 });
        }
    }
    match (a.provider_name.as_deref(), b.provider_name.as_deref()) {
        (Some(an), Some(bn)) if !an.is_empty() && !bn.is_empty() => {
            Some(jaro_winkler(&normalize::fold(an), &normalize::fold(bn)))
        }
        _ => None,
    }
}

fn educational_level_score(a: &Course, b: &Course) -> Option<f64> {
    let (al, bl) = match (&a.educational_level, &b.educational_level) {
        (Some(al), Some(bl)) => (al, bl),
        _ => return None,
    };
    Some(if al == bl {
        1.0
    } else if educational_level_one_off(al, bl) {
        0.5
    } else {
        0.0
    })
}

fn educational_level_one_off(a: &EducationalLevel, b: &EducationalLevel) -> bool {
    use EducationalLevel::*;
    // Skill ladder.
    let skill = [Beginner, Intermediate, Advanced, Expert];
    if let (Some(ai), Some(bi)) = (skill.iter().position(|x| x == a), skill.iter().position(|x| x == b)) {
        return (ai as i32 - bi as i32).abs() == 1;
    }
    // School ladder.
    let school = [PrimaryEducation, SecondaryEducation, HigherEducation];
    if let (Some(ai), Some(bi)) = (school.iter().position(|x| x == a), school.iter().position(|x| x == b)) {
        return (ai as i32 - bi as i32).abs() == 1;
    }
    // Degree ladder.
    let degree = [Undergraduate, Graduate, Postgraduate];
    if let (Some(ai), Some(bi)) = (degree.iter().position(|x| x == a), degree.iter().position(|x| x == b)) {
        return (ai as i32 - bi as i32).abs() == 1;
    }
    false
}

fn set_jaccard(a: &[String], b: &[String]) -> Option<f64> {
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    if a_set.is_empty() && b_set.is_empty() {
        return None;
    }
    if a_set.is_empty() || b_set.is_empty() {
        return Some(0.0);
    }
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        Some(0.0)
    } else {
        Some(inter as f64 / union as f64)
    }
}

// Re-export so the `course_matcher::CourseIdentifier` path resolves
// from inside other modules without re-naming.
#[allow(unused_imports)]
use CourseIdentifier as _;

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::IdentifierScheme;

    fn ident(scheme: IdentifierScheme, value: &str) -> crate::CourseIdentifier {
        crate::CourseIdentifier { scheme, value: value.into() }
    }

    #[test]
    fn identical_courses_score_1() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("CS101 Introduction to Computer Science");
        let b = Course::new("CS101 Introduction to Computer Science");
        let r = engine.match_courses(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert!(r.is_match);
    }

    #[test]
    fn doi_match_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Course::new("A");
        let mut b = Course::new("Completely different");
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1234/abc"));
        b.identifiers.push(ident(IdentifierScheme::Doi, "10.1234/abc"));
        let r = engine.match_courses(&a, &b);
        assert_eq!(r.score, 1.0);
        assert!(r.breakdown.deterministic_match);
    }

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
        assert_eq!(r.score, 1.0);
    }

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
        assert!((with - base).abs() < 1e-9, "expected base {base}, got {with}");
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

    #[test]
    fn match_one_to_many_empty_input_returns_empty() {
        let engine = MatchingEngine::default_config();
        let query = Course::new("Anything");
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
    }

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
}
