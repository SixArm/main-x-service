//! `MatchingEngine` — the public entry point.
//!
//! Two phases:
//!
//! 1. **Deterministic short-circuit.** If both records share a value on
//!    a deterministic identifier scheme (`Docket`, `ExternalCaseId`,
//!    URI, UUID) OR share `agency_id` (or `agency_name` fallback) +
//!    normalised `case_number` OR overlap on a `same_as` URL, return
//!    score `1.0`.
//! 2. **Probabilistic scoring.** Per-component scores, then a weighted
//!    average over the *present* components.

use strsim::jaro_winkler;

use crate::case::Case;
use crate::config::MatchConfig;
use crate::normalize;
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

const PHONETIC_BONUS: f64 = 0.05;
const PHONETIC_CEILING: f64 = 0.95;

/// The case matcher: holds a [`MatchConfig`] and scores pairs.
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with `MatchConfig::default()`.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    #[must_use]
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two cases. Always returns a result (never errs).
    ///
    /// # Examples
    ///
    /// ```
    /// use case_matcher::{Case, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = Case::new("Housing benefit appeal — J. Smith");
    /// let b = Case::new("Housing benefit appeal — John Smith");
    /// let r = engine.match_cases(&a, &b);
    /// assert!((0.0..=1.0).contains(&r.score));
    /// ```
    #[must_use]
    pub fn match_cases(&self, a: &Case, b: &Case) -> MatchResult {
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

        let title_score = Some(title_score(a, b));
        let subjects_score = set_jaccard(&a.subjects, &b.subjects);
        let case_number_score = case_number_score(a, b);
        let case_type_score = case_type_score(a, b);
        let status_score = status_score(a, b);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);

        let score = weighted_average(&[
            (title_score, self.config.title_weight),
            (subjects_score, self.config.subjects_weight),
            (case_number_score, self.config.case_number_weight),
            (case_type_score, self.config.case_type_weight),
            (status_score, self.config.status_weight),
            (keywords_score, self.config.keywords_weight),
        ]);

        let is_match = score >= self.config.threshold;
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: MatchBreakdown {
                title_score,
                subjects_score,
                case_number_score,
                case_type_score,
                status_score,
                keywords_score,
                deterministic_match: false,
            },
        }
    }

    /// One-to-many: results in input order.
    #[must_use]
    pub fn match_one_to_many(&self, query: &Case, candidates: &[Case]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_cases(query, c))
            .collect()
    }

    /// One-to-many: `(index, result)` sorted by descending score.
    #[must_use]
    pub fn rank(&self, query: &Case, candidates: &[Case]) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_cases(query, c)))
            .collect();
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Rank then drop everything below `MatchConfig::threshold`.
    #[must_use]
    pub fn find_matches(&self, query: &Case, candidates: &[Case]) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

/// The agency key both records must share for the agency-scoped rules:
/// prefer `agency_id`, fall back to `agency_name`.
fn agency_key(c: &Case) -> Option<&str> {
    c.agency_id
        .as_deref()
        .or(c.agency_name.as_deref())
        .filter(|s| !s.is_empty())
}

fn deterministic_match(a: &Case, b: &Case) -> bool {
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

    // R-1 — same agency + same normalised case_number.
    if let (Some(ak), Some(bk), Some(ac), Some(bc)) = (
        agency_key(a),
        agency_key(b),
        a.case_number.as_deref(),
        b.case_number.as_deref(),
    ) && ak == bk
        && !normalize::case_number(ac).is_empty()
        && normalize::case_number(ac) == normalize::case_number(bc)
    {
        return true;
    }

    // R-2 — any same_as URL overlaps (case-folded).
    for au in &a.same_as {
        let an = normalize::url(au);
        if an.is_empty() {
            continue;
        }
        for bu in &b.same_as {
            if an == normalize::url(bu) {
                return true;
            }
        }
    }

    false
}

// ─── Probabilistic components ────────────────────────────────────

fn title_score(a: &Case, b: &Case) -> f64 {
    let an = normalize::fold(&a.title);
    let bn = normalize::fold(&b.title);
    let mut best = jaro_winkler(&an, &bn);
    for alt in &a.alternate_titles {
        best = best.max(jaro_winkler(&normalize::fold(alt), &bn));
    }
    for alt in &b.alternate_titles {
        best = best.max(jaro_winkler(&an, &normalize::fold(alt)));
    }
    if best < PHONETIC_CEILING && phonetic::same(&an, &bn) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

fn case_number_score(a: &Case, b: &Case) -> Option<f64> {
    let (ac, bc) = match (a.case_number.as_deref(), b.case_number.as_deref()) {
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };
    // Across-agency case numbers are noise. Only contribute when both
    // records share an agency.
    match (agency_key(a), agency_key(b)) {
        (Some(ak), Some(bk)) if ak == bk => {
            if normalize::case_number(ac) == normalize::case_number(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        _ => None,
    }
}

fn case_type_score(a: &Case, b: &Case) -> Option<f64> {
    match (&a.case_type, &b.case_type) {
        (Some(x), Some(y)) => Some(if x == y { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn status_score(a: &Case, b: &Case) -> Option<f64> {
    match (&a.status, &b.status) {
        (Some(x), Some(y)) => Some(if x == y { 1.0 } else { 0.0 }),
        _ => None,
    }
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
        #[allow(clippy::cast_precision_loss)]
        Some(inter as f64 / union as f64)
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::{CaseIdentifier, CaseStatus, CaseType, IdentifierScheme};

    fn ident(scheme: IdentifierScheme, value: &str) -> CaseIdentifier {
        CaseIdentifier {
            scheme,
            value: value.into(),
        }
    }

    #[test]
    fn identical_cases_score_high() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let b = Case::new("Housing benefit appeal — J. Smith");
        let r = engine.match_cases(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
    }

    #[test]
    fn docket_match_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("A");
        let mut b = Case::new("Totally Different");
        a.identifiers
            .push(ident(IdentifierScheme::Docket, "CV-2024-001234"));
        b.identifiers
            .push(ident(IdentifierScheme::Docket, "cv-2024-001234"));
        let r = engine.match_cases(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    #[test]
    fn external_case_id_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Benefit review");
        let mut b = Case::new("Entitlement reassessment");
        a.identifiers
            .push(ident(IdentifierScheme::ExternalCaseId, "EXT-9001"));
        b.identifiers
            .push(ident(IdentifierScheme::ExternalCaseId, "ext-9001"));
        let r = engine.match_cases(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn agency_scoped_case_number_does_not_short_circuit_across_agencies() {
        let mut a = Case::new("A");
        let mut b = Case::new("B");
        a.case_number = Some("CV-2024-001234".into());
        b.case_number = Some("CV-2024-001234".into());
        // No agency → no short-circuit.
        assert!(!deterministic_match(&a, &b));
        // Different agency → component skipped.
        a.agency_id = Some("agency-1".into());
        b.agency_id = Some("agency-2".into());
        assert_eq!(case_number_score(&a, &b), None);
        // Same agency → short-circuits.
        b.agency_id = Some("agency-1".into());
        assert!(deterministic_match(&a, &b));
    }

    #[test]
    fn agency_name_fallback_gates_case_number() {
        let mut a = Case::new("A");
        let mut b = Case::new("B");
        a.case_number = Some("CV-2024-001234".into());
        b.case_number = Some("cv 2024 001234".into());
        a.agency_name = Some("County Court".into());
        b.agency_name = Some("County Court".into());
        assert!(deterministic_match(&a, &b));
    }

    #[test]
    fn same_as_overlap_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Alpha");
        let mut b = Case::new("Omega");
        a.same_as = vec!["https://courts.example.gov/case/CV-2024-001234".into()];
        b.same_as = vec!["  https://courts.example.gov/case/CV-2024-001234  ".into()];
        let r = engine.match_cases(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn subjects_corroborate() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Housing benefit appeal — J. Smith");
        let mut b = Case::new("Housing benefit appeal — John Smith");
        a.subjects = vec!["person:pid-42".into()];
        b.subjects = vec!["PERSON:PID-42".into()];
        let r = engine.match_cases(&a, &b);
        assert_eq!(r.breakdown.subjects_score, Some(1.0));
        assert!(r.is_match, "got {}", r.score);
    }

    #[test]
    fn subjects_jaccard_partial_and_skip() {
        let a = vec!["person:1".to_string(), "person:2".to_string()];
        let b = vec!["person:1".to_string()];
        let got = set_jaccard(&a, &b).expect("some");
        assert!((got - 0.5).abs() < 1e-9, "got {got}");
        assert_eq!(set_jaccard(&[], &[]), None);
    }

    #[test]
    fn case_type_exact_and_mismatch() {
        let mut a = Case::new("A");
        let mut b = Case::new("B");
        a.case_type = Some(CaseType::Housing);
        b.case_type = Some(CaseType::Housing);
        assert_eq!(case_type_score(&a, &b), Some(1.0));
        b.case_type = Some(CaseType::Benefit);
        assert_eq!(case_type_score(&a, &b), Some(0.0));
        b.case_type = None;
        assert_eq!(case_type_score(&a, &b), None);
    }

    #[test]
    fn status_exact_and_mismatch() {
        let mut a = Case::new("A");
        let mut b = Case::new("B");
        a.status = Some(CaseStatus::Open);
        b.status = Some(CaseStatus::Open);
        assert_eq!(status_score(&a, &b), Some(1.0));
        b.status = Some(CaseStatus::Closed);
        assert_eq!(status_score(&a, &b), Some(0.0));
        b.status = None;
        assert_eq!(status_score(&a, &b), None);
    }

    #[test]
    fn unrelated_cases_score_low() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let b = Case::new("Commercial driving licence renewal");
        let r = engine.match_cases(&a, &b);
        assert!(!r.is_match, "got {}", r.score);
        assert_eq!(r.confidence, crate::scoring::Confidence::Low);
    }

    #[test]
    fn rank_and_find_matches() {
        let engine = MatchingEngine::default_config();
        let query = Case::new("Housing benefit appeal — J. Smith");
        let cands = vec![
            Case::new("Commercial driving licence renewal"),
            Case::new("Housing benefit appeal — J. Smith"),
        ];
        let ranked = engine.rank(&query, &cands);
        assert_eq!(ranked[0].0, 1);
        let matches = engine.find_matches(&query, &cands);
        assert!(matches.iter().all(|(_, r)| r.is_match));
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
    }
}
