//! `MatchingEngine` — the public entry point.
//!
//! Two phases:
//!
//! 1. **Deterministic short-circuit.** If both records share a value on
//!    a deterministic identifier scheme (DOI, Wikidata, `GuidelineId`,
//!    URI, UUID) OR share `provider_id` + normalised `pathway_code` OR
//!    overlap on a `same_as` URL, return score `1.0`.
//! 2. **Probabilistic scoring.** Per-component scores, then a weighted
//!    average over the *present* components.

use strsim::jaro_winkler;

use crate::care_pathway::{CarePathway, CodeSystem, ConditionCode};
use crate::config::MatchConfig;
use crate::normalize;
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

const PHONETIC_BONUS: f64 = 0.05;
const PHONETIC_CEILING: f64 = 0.95;

/// The care-pathway matcher: holds a [`MatchConfig`] and scores pairs.
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

    /// Score two care pathways. Always returns a result (never errs).
    ///
    /// # Examples
    ///
    /// ```
    /// use care_pathway_matcher::{CarePathway, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = CarePathway::new("Acute Stroke Care Pathway");
    /// let b = CarePathway::new("Acute Stroke Pathway");
    /// let r = engine.match_care_pathways(&a, &b);
    /// assert!((0.0..=1.0).contains(&r.score));
    /// ```
    #[must_use]
    pub fn match_care_pathways(&self, a: &CarePathway, b: &CarePathway) -> MatchResult {
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

        let name_score = Some(name_score(a, b));
        let condition_score = set_jaccard(
            &condition_tokens(&a.condition_codes),
            &condition_tokens(&b.condition_codes),
        );
        let pathway_code_score = pathway_code_score(a, b);
        let care_setting_score = care_setting_score(a, b);
        let interventions_score = set_jaccard(&a.interventions, &b.interventions);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);

        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (condition_score, self.config.condition_weight),
            (pathway_code_score, self.config.pathway_code_weight),
            (care_setting_score, self.config.care_setting_weight),
            (interventions_score, self.config.interventions_weight),
            (keywords_score, self.config.keywords_weight),
        ]);

        let is_match = score >= self.config.threshold;
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: MatchBreakdown {
                name_score,
                condition_score,
                pathway_code_score,
                care_setting_score,
                interventions_score,
                keywords_score,
                deterministic_match: false,
            },
        }
    }

    /// One-to-many: results in input order.
    #[must_use]
    pub fn match_one_to_many(
        &self,
        query: &CarePathway,
        candidates: &[CarePathway],
    ) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_care_pathways(query, c))
            .collect()
    }

    /// One-to-many: `(index, result)` sorted by descending score.
    #[must_use]
    pub fn rank(
        &self,
        query: &CarePathway,
        candidates: &[CarePathway],
    ) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_care_pathways(query, c)))
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
    pub fn find_matches(
        &self,
        query: &CarePathway,
        candidates: &[CarePathway],
    ) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

fn deterministic_match(a: &CarePathway, b: &CarePathway) -> bool {
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

    // R-1 — same provider + same normalised pathway_code.
    if let (Some(ap), Some(bp), Some(ac), Some(bc)) = (
        a.provider_id.as_deref(),
        b.provider_id.as_deref(),
        a.pathway_code.as_deref(),
        b.pathway_code.as_deref(),
    ) && !ap.is_empty()
        && ap == bp
        && normalize::pathway_code(ac) == normalize::pathway_code(bc)
    {
        return true;
    }

    // R-2 — any same_as URL overlaps (case-folded).
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

fn name_score(a: &CarePathway, b: &CarePathway) -> f64 {
    let an = normalize::fold(&a.name);
    let bn = normalize::fold(&b.name);
    let mut best = jaro_winkler(&an, &bn);
    for alt in &a.alternate_names {
        best = best.max(jaro_winkler(&normalize::fold(alt), &bn));
    }
    for alt in &b.alternate_names {
        best = best.max(jaro_winkler(&an, &normalize::fold(alt)));
    }
    if best < PHONETIC_CEILING && phonetic::same(&an, &bn) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

/// Render condition codes as comparable `"system:code"` tokens.
fn condition_tokens(codes: &[ConditionCode]) -> Vec<String> {
    codes
        .iter()
        .map(|c| {
            let system = match &c.system {
                CodeSystem::Icd10 => "icd10".to_string(),
                CodeSystem::Icd11 => "icd11".to_string(),
                CodeSystem::Snomed => "snomed".to_string(),
                CodeSystem::Custom(s) => normalize::fold(s),
            };
            format!("{system}:{}", normalize::fold(&c.code))
        })
        .collect()
}

fn pathway_code_score(a: &CarePathway, b: &CarePathway) -> Option<f64> {
    let (ac, bc) = match (a.pathway_code.as_deref(), b.pathway_code.as_deref()) {
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };
    // Across-provider pathway codes are noise. Only contribute when both
    // records share a provider.
    match (a.provider_id.as_deref(), b.provider_id.as_deref()) {
        (Some(ap), Some(bp)) if !ap.is_empty() && ap == bp => {
            if normalize::pathway_code(ac) == normalize::pathway_code(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        _ => None,
    }
}

fn care_setting_score(a: &CarePathway, b: &CarePathway) -> Option<f64> {
    match (&a.care_setting, &b.care_setting) {
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
    use crate::care_pathway::{CareSetting, IdentifierScheme, PathwayIdentifier};

    fn ident(scheme: IdentifierScheme, value: &str) -> PathwayIdentifier {
        PathwayIdentifier {
            scheme,
            value: value.into(),
        }
    }

    fn cond(system: CodeSystem, code: &str) -> ConditionCode {
        ConditionCode {
            system,
            code: code.into(),
        }
    }

    #[test]
    fn identical_pathways_score_high() {
        let engine = MatchingEngine::default_config();
        let a = CarePathway::new("Acute Stroke Care Pathway");
        let b = CarePathway::new("Acute Stroke Care Pathway");
        let r = engine.match_care_pathways(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
    }

    #[test]
    fn doi_match_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = CarePathway::new("A");
        let mut b = CarePathway::new("Totally Different");
        a.identifiers
            .push(ident(IdentifierScheme::Doi, "10.1/stroke"));
        b.identifiers
            .push(ident(IdentifierScheme::Doi, "10.1/stroke"));
        let r = engine.match_care_pathways(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    #[test]
    fn guideline_id_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = CarePathway::new("Stroke");
        let mut b = CarePathway::new("Cerebrovascular accident pathway");
        a.identifiers
            .push(ident(IdentifierScheme::GuidelineId, "NICE-NG128"));
        b.identifiers
            .push(ident(IdentifierScheme::GuidelineId, "nice-ng128"));
        let r = engine.match_care_pathways(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn provider_scoped_pathway_code_does_not_short_circuit_across_providers() {
        let mut a = CarePathway::new("A");
        let mut b = CarePathway::new("B");
        a.pathway_code = Some("STROKE-01".into());
        b.pathway_code = Some("STROKE-01".into());
        // No provider → no short-circuit.
        assert!(!deterministic_match(&a, &b));
        // Different provider → component skipped.
        a.provider_id = Some("trust-1".into());
        b.provider_id = Some("trust-2".into());
        assert_eq!(pathway_code_score(&a, &b), None);
        // Same provider → short-circuits.
        b.provider_id = Some("trust-1".into());
        assert!(deterministic_match(&a, &b));
    }

    #[test]
    fn same_as_overlap_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = CarePathway::new("Alpha");
        let mut b = CarePathway::new("Omega");
        a.same_as = vec!["https://www.nice.org.uk/guidance/ng128".into()];
        b.same_as = vec!["  https://www.nice.org.uk/guidance/ng128  ".into()];
        let r = engine.match_care_pathways(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn condition_codes_corroborate() {
        let engine = MatchingEngine::default_config();
        let mut a = CarePathway::new("Acute Stroke Care Pathway");
        let mut b = CarePathway::new("Acute Stroke Pathway");
        a.condition_codes = vec![cond(CodeSystem::Icd10, "I63")];
        b.condition_codes = vec![cond(CodeSystem::Icd10, "i63")];
        let r = engine.match_care_pathways(&a, &b);
        assert_eq!(r.breakdown.condition_score, Some(1.0));
        assert!(r.is_match, "got {}", r.score);
    }

    #[test]
    fn condition_jaccard_partial_and_skip() {
        let a = vec![
            cond(CodeSystem::Icd10, "I63"),
            cond(CodeSystem::Snomed, "230690007"),
        ];
        let b = vec![cond(CodeSystem::Icd10, "I63")];
        let got = set_jaccard(&condition_tokens(&a), &condition_tokens(&b)).expect("some");
        assert!((got - 0.5).abs() < 1e-9, "got {got}");
        assert_eq!(
            set_jaccard(&condition_tokens(&[]), &condition_tokens(&[])),
            None
        );
    }

    #[test]
    fn care_setting_exact_and_mismatch() {
        let mut a = CarePathway::new("A");
        let mut b = CarePathway::new("B");
        a.care_setting = Some(CareSetting::Inpatient);
        b.care_setting = Some(CareSetting::Inpatient);
        assert_eq!(care_setting_score(&a, &b), Some(1.0));
        b.care_setting = Some(CareSetting::PrimaryCare);
        assert_eq!(care_setting_score(&a, &b), Some(0.0));
        b.care_setting = None;
        assert_eq!(care_setting_score(&a, &b), None);
    }

    #[test]
    fn unrelated_pathways_score_low() {
        let engine = MatchingEngine::default_config();
        let a = CarePathway::new("Acute Stroke Care Pathway");
        let b = CarePathway::new("Diabetic Foot Ulcer Management");
        let r = engine.match_care_pathways(&a, &b);
        assert!(!r.is_match, "got {}", r.score);
        assert_eq!(r.confidence, crate::scoring::Confidence::Low);
    }

    #[test]
    fn rank_and_find_matches() {
        let engine = MatchingEngine::default_config();
        let query = CarePathway::new("Acute Stroke Care Pathway");
        let cands = vec![
            CarePathway::new("Diabetic Foot Ulcer Management"),
            CarePathway::new("Acute Stroke Care Pathway"),
        ];
        let ranked = engine.rank(&query, &cands);
        assert_eq!(ranked[0].0, 1);
        let matches = engine.find_matches(&query, &cands);
        assert!(matches.iter().all(|(_, r)| r.is_match));
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
    }
}
