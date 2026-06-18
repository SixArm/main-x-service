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

/// Additive bonus applied to `name_score` when two names share a Soundex
/// code. Small (`0.05`) on purpose: phonetic agreement is weak evidence,
/// so it nudges a borderline near-match up rather than dominating.
const PHONETIC_BONUS: f64 = 0.05;
/// Upper bound the phonetic bonus may lift a score to. Kept just under
/// `1.0` so a phonetic-only agreement can never reach the High band
/// (>= 0.95) or pass for a deterministic-grade `1.0` match.
const PHONETIC_CEILING: f64 = 0.95;

/// The care-pathway matcher: holds a [`MatchConfig`] and scores pairs.
///
/// Stateless apart from its configuration, so a single engine can be
/// shared across threads and reused for many comparisons.
pub struct MatchingEngine {
    /// Weights + threshold governing the probabilistic score and the
    /// `is_match` cutoff. Immutable for the engine's lifetime.
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with `MatchConfig::default()`.
    ///
    /// Convenience constructor for the common case where the caller wants
    /// the canonical weights and the 0.85 threshold.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    ///
    /// Lets callers inspect the active weights / threshold (e.g. to
    /// explain a score) without owning or cloning the config.
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
        // Phase 1 — deterministic short-circuit. A shared globally-unique
        // identifier (or R-1/R-2 rule) is conclusive, so skip scoring
        // entirely and pin to 1.0 / High. The breakdown's component
        // scores stay `None` because no fuzzy work was done.
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

        // Phase 2 — probabilistic scoring. Compute each component; a
        // component yields `None` when one or both records lack the data
        // so it is dropped from (rather than penalising) the average.
        // `name` is always present (records require a name), hence `Some`.
        let name_score = Some(name_score(a, b));
        // Condition codes are the defining attribute of a pathway; compare
        // them as a set via Jaccard over `"system:code"` tokens.
        let condition_score = set_jaccard(
            &condition_tokens(&a.condition_codes),
            &condition_tokens(&b.condition_codes),
        );
        let pathway_code_score = pathway_code_score(a, b);
        let care_setting_score = care_setting_score(a, b);
        let interventions_score = set_jaccard(&a.interventions, &b.interventions);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);

        // Renormalised weighted average over the present components only.
        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (condition_score, self.config.condition_weight),
            (pathway_code_score, self.config.pathway_code_weight),
            (care_setting_score, self.config.care_setting_weight),
            (interventions_score, self.config.interventions_weight),
            (keywords_score, self.config.keywords_weight),
        ]);

        // `is_match` is purely threshold-driven; the band in `confidence`
        // is computed independently from the same score.
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
    ///
    /// Scores `query` against every candidate, preserving the candidates'
    /// order so the caller can zip results back to their inputs by index.
    /// Returns an empty `Vec` for an empty candidate slice.
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
    ///
    /// Like [`Self::match_one_to_many`] but carries each candidate's
    /// original index and orders the best matches first — the typical
    /// shape for a "top-N candidates" UI.
    #[must_use]
    pub fn rank(
        &self,
        query: &CarePathway,
        candidates: &[CarePathway],
    ) -> Vec<(usize, MatchResult)> {
        // Pair each result with its source index before sorting, so the
        // returned index still points at the caller's candidate slice.
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_care_pathways(query, c)))
            .collect();
        // Descending by score. `partial_cmp` is `None` only for NaN; scores
        // are finite here, so falling back to `Equal` keeps the sort total
        // and panic-free (library code must never panic).
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Rank then drop everything below `MatchConfig::threshold`.
    ///
    /// Convenience over [`Self::rank`] that keeps only the entries whose
    /// `is_match` flag is set, i.e. the actual probable/deterministic
    /// matches, still in descending-score order.
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

/// True when any deterministic short-circuit rule fires, meaning the two
/// records are conclusively the same pathway and probabilistic scoring
/// can be skipped. Checks three independent rules (any one suffices):
///
/// - **R-0** — a shared value on a globally-unique identifier scheme.
/// - **R-1** — same provider plus same normalised pathway code.
/// - **R-2** — an overlapping `same_as` identity URL.
fn deterministic_match(a: &CarePathway, b: &CarePathway) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value.
    // Compare only within the same scheme (a DOI value must not match a
    // UUID value) and only for schemes flagged globally unique.
    for ai in &a.identifiers {
        // Provider-scoped / Custom schemes are not globally unique → skip.
        if !ai.scheme.is_deterministic() {
            continue;
        }
        let av = normalize::fold(&ai.value);
        // An empty (or whitespace-only) identifier value carries no
        // identity and must never match another empty value.
        if av.is_empty() {
            continue;
        }
        for bi in &b.identifiers {
            // Same scheme AND equal folded value ⇒ conclusive match.
            if ai.scheme == bi.scheme && av == normalize::fold(&bi.value) {
                return true;
            }
        }
    }

    // R-1 — same provider + same normalised pathway_code. A pathway code
    // is only unique *within* its issuing provider, so all four parts
    // must be present and the providers must agree before comparing codes.
    if let (Some(ap), Some(bp), Some(ac), Some(bc)) = (
        a.provider_id.as_deref(),
        b.provider_id.as_deref(),
        a.pathway_code.as_deref(),
        b.pathway_code.as_deref(),
    ) && !ap.is_empty() // empty provider id is not a real scope
        && ap == bp // identical provider scope
        && normalize::pathway_code(ac) == normalize::pathway_code(bc)
    {
        return true;
    }

    // R-2 — any same_as URL overlaps (case-folded). Two records pointing
    // at the same external identity page are the same pathway. Folding
    // tolerates surrounding whitespace and case differences.
    for au in &a.same_as {
        let an = normalize::fold(au);
        // Skip blank URLs so two blanks don't spuriously "overlap".
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

/// Name similarity in `[0.0, 1.0]` via Jaro-Winkler, taking the best
/// score across each record's primary name and its alternate names, then
/// applying a small Soundex bonus.
///
/// Trying alternates on both sides means an abbreviation or former title
/// on either record can rescue the match. The phonetic bonus only applies
/// below [`PHONETIC_CEILING`] so it can lift a near-miss but never invent
/// a High-band score from sound-alikes alone.
fn name_score(a: &CarePathway, b: &CarePathway) -> f64 {
    let an = normalize::fold(&a.name);
    let bn = normalize::fold(&b.name);
    // Start from the primary-vs-primary similarity.
    let mut best = jaro_winkler(&an, &bn);
    // `a`'s alternates vs `b`'s primary.
    for alt in &a.alternate_names {
        best = best.max(jaro_winkler(&normalize::fold(alt), &bn));
    }
    // `b`'s alternates vs `a`'s primary.
    for alt in &b.alternate_names {
        best = best.max(jaro_winkler(&an, &normalize::fold(alt)));
    }
    // Phonetic nudge: only when not already in/above the High band, and
    // capped at the ceiling so the bonus can't push past it.
    if best < PHONETIC_CEILING && phonetic::same(&an, &bn) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

/// Render condition codes as comparable `"system:code"` tokens.
///
/// Qualifying each code with its system prevents a code collision across
/// systems (an ICD code that happens to equal a SNOMED code must not
/// count as a match). Both halves are folded so case/whitespace
/// differences don't split otherwise-equal tokens; the resulting tokens
/// feed [`set_jaccard`].
fn condition_tokens(codes: &[ConditionCode]) -> Vec<String> {
    codes
        .iter()
        .map(|c| {
            // Map known systems to stable lowercase labels; `Custom`
            // labels are folded so casing can't fork the token.
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

/// Provider-scoped pathway-code component, binary `1.0` / `0.0`, or
/// `None` when the component does not apply.
///
/// Returns `None` unless both records carry a non-empty pathway code AND
/// share a non-empty provider id — comparing codes across different
/// providers is noise, since the same code string means different things
/// to different organisations. When in scope, codes are compared after
/// [`normalize::pathway_code`] folding (alphanumerics only, uppercased).
fn pathway_code_score(a: &CarePathway, b: &CarePathway) -> Option<f64> {
    // Both codes must be present and non-empty, else the component is N/A.
    let (ac, bc) = match (a.pathway_code.as_deref(), b.pathway_code.as_deref()) {
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };
    // Across-provider pathway codes are noise. Only contribute when both
    // records share a provider.
    match (a.provider_id.as_deref(), b.provider_id.as_deref()) {
        (Some(ap), Some(bp)) if !ap.is_empty() && ap == bp => {
            // Equal normalised code ⇒ full credit, otherwise zero (a
            // pathway code is an exact key, so there is no partial match).
            if normalize::pathway_code(ac) == normalize::pathway_code(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        // Missing or differing provider scope ⇒ component does not apply.
        _ => None,
    }
}

/// Care-setting component: `1.0` for an exact match, `0.0` for a
/// mismatch, or `None` when either record omits the setting.
///
/// Care setting is a small closed-ish enum, so equality is the right
/// comparison; there is no notion of "nearby" settings.
fn care_setting_score(a: &CarePathway, b: &CarePathway) -> Option<f64> {
    match (&a.care_setting, &b.care_setting) {
        // Both present ⇒ exact-equality test (variants, incl. `Custom`).
        (Some(x), Some(y)) => Some(if x == y { 1.0 } else { 0.0 }),
        // Either side absent ⇒ skip the component.
        _ => None,
    }
}

/// Jaccard similarity over two string sets: `|A ∩ B| / |A ∪ B|`.
///
/// Used for the set-valued components (condition codes as tokens,
/// interventions, keywords). Inputs are folded and deduplicated first via
/// [`normalize::fold_set`]. Returns:
///
/// - `None` — both sides are empty (nothing to compare; the component is
///   dropped from the weighted average rather than scored `0.0`).
/// - `Some(0.0)` — exactly one side is empty (real disagreement: one has
///   data, the other has none).
/// - `Some(ratio)` — otherwise the standard Jaccard ratio in `[0.0, 1.0]`.
fn set_jaccard(a: &[String], b: &[String]) -> Option<f64> {
    // Fast path: nothing on either side ⇒ component absent.
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    // Re-check after folding: inputs may have been only blanks/dupes.
    if a_set.is_empty() && b_set.is_empty() {
        return None;
    }
    // One side empty, the other not ⇒ maximal disagreement.
    if a_set.is_empty() || b_set.is_empty() {
        return Some(0.0);
    }
    // Intersection size by membership test against the (deduped) `b_set`.
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    // Inclusion–exclusion: |A ∪ B| = |A| + |B| − |A ∩ B|.
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        // Unreachable given the emptiness guards above, but kept as a
        // defensive divide-by-zero guard.
        Some(0.0)
    } else {
        // `inter`/`union` are small set cardinalities; convert losslessly
        // via `u32` so the ratio is exact (only absurd counts saturate).
        Some(
            f64::from(u32::try_from(inter).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(union).unwrap_or(u32::MAX)),
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::care_pathway::{CareSetting, IdentifierScheme, PathwayIdentifier};

    /// Terse constructor for a [`PathwayIdentifier`] in tests.
    fn ident(scheme: IdentifierScheme, value: &str) -> PathwayIdentifier {
        PathwayIdentifier {
            scheme,
            value: value.into(),
        }
    }

    /// Terse constructor for a [`ConditionCode`] in tests.
    fn cond(system: CodeSystem, code: &str) -> ConditionCode {
        ConditionCode {
            system,
            code: code.into(),
        }
    }

    // Identical names with no other data must land in the High band and
    // count as a match — the simplest positive case.
    #[test]
    fn identical_pathways_score_high() {
        let engine = MatchingEngine::default_config();
        let a = CarePathway::new("Acute Stroke Care Pathway");
        let b = CarePathway::new("Acute Stroke Care Pathway");
        let r = engine.match_care_pathways(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
    }

    // R-0: a shared DOI pins the score to exactly 1.0 and sets the
    // deterministic flag, overriding completely different names.
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

    // R-0 again, with a case difference ("NICE-NG128" vs "nice-ng128") to
    // pin that identifier comparison is case-folded before equality.
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

    // Pins the provider-scoping invariant for pathway codes across three
    // states: no provider (no short-circuit, R-1 needs a provider),
    // different providers (component skipped → `None`), and same provider
    // (R-1 fires). Guards against treating a code as globally unique.
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

    // R-2: overlapping `same_as` URLs short-circuit. The surrounding
    // whitespace on `b`'s URL pins that folding trims before comparison.
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

    // A shared condition code (case-insensitively) scores 1.0 on the
    // condition component and tips a near-name into an overall match.
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

    // Jaccard arithmetic on condition tokens: one shared code out of two
    // distinct ones gives 0.5 (|∩|=1, |∪|=2); two empty sets give `None`.
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

    // Care-setting component across its three outcomes: equal → 1.0,
    // differing → 0.0, one side absent → `None`.
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

    // Negative control: clearly different pathways are non-matches and
    // classify into the Low confidence band.
    #[test]
    fn unrelated_pathways_score_low() {
        let engine = MatchingEngine::default_config();
        let a = CarePathway::new("Acute Stroke Care Pathway");
        let b = CarePathway::new("Diabetic Foot Ulcer Management");
        let r = engine.match_care_pathways(&a, &b);
        assert!(!r.is_match, "got {}", r.score);
        assert_eq!(r.confidence, crate::scoring::Confidence::Low);
    }

    // Phonetic bonus wiring (§9): two non-identical but Soundex-equal
    // names get the +0.05 nudge, yet the bonus is capped at the ceiling so
    // it can never reach the High band (>= 0.95) from sound-alikes alone.
    #[test]
    fn phonetic_bonus_lifts_but_caps_below_high_band() {
        // "Smyth" / "Smith" share Soundex S530 but are not identical, so the
        // base Jaro-Winkler is below the ceiling and the bonus applies.
        let base = jaro_winkler(&normalize::fold("Smyth"), &normalize::fold("Smith"));
        assert!(base < PHONETIC_CEILING, "base {base} must be below ceiling");
        let a = CarePathway::new("Smyth");
        let b = CarePathway::new("Smith");
        let lifted = name_score(&a, &b);
        // The score is lifted by the bonus (up to the ceiling)...
        assert!(lifted > base, "lifted {lifted} should exceed base {base}");
        assert!(
            (lifted - (base + PHONETIC_BONUS).min(PHONETIC_CEILING)).abs() < 1e-9,
            "lifted {lifted} should be base + bonus, capped at ceiling"
        );
        // ...but a phonetic-only agreement can never reach the High band.
        assert!(lifted < 0.95, "phonetic bonus must stay below High band");
    }

    // The phonetic bonus is suppressed once the base score already clears
    // the ceiling: identical names stay at exactly 1.0 (no +0.05 overshoot).
    #[test]
    fn phonetic_bonus_suppressed_at_or_above_ceiling() {
        let a = CarePathway::new("Stroke");
        let b = CarePathway::new("Stroke");
        let s = name_score(&a, &b);
        assert!((s - 1.0).abs() < 1e-9, "identical names stay at 1.0, got {s}");
    }

    // `set_jaccard` `Some(0.0)` branch (§13): exactly one side populated is
    // real disagreement (not skipped). Distinct from both-empty (`None`).
    #[test]
    fn set_jaccard_one_side_populated_is_zero() {
        let populated = vec!["thrombolysis".to_string()];
        let empty: Vec<String> = vec![];
        assert_eq!(set_jaccard(&populated, &empty), Some(0.0));
        assert_eq!(set_jaccard(&empty, &populated), Some(0.0));
        // Contrast: both empty is `None` (component dropped, not scored 0.0).
        assert_eq!(set_jaccard(&empty, &empty), None);
        // Folding to empty (blanks only on one side) is still the 0.0 branch.
        let blanks = vec!["   ".to_string()];
        assert_eq!(set_jaccard(&populated, &blanks), Some(0.0));
    }

    // `alternate_names` rescue (§9): primary names diverge, but A's
    // alternate equals B's primary, so the best Jaro-Winkler is taken from
    // the alternate and the names component scores ~1.0.
    #[test]
    fn alternate_name_rescues_diverging_primary_names() {
        let mut a = CarePathway::new("CVA Pathway");
        a.alternate_names = vec!["Acute Stroke Care Pathway".into()];
        let b = CarePathway::new("Acute Stroke Care Pathway");
        // Primary-vs-primary alone is weak; the alternate rescues it.
        let primary_only = jaro_winkler(
            &normalize::fold("CVA Pathway"),
            &normalize::fold("Acute Stroke Care Pathway"),
        );
        let rescued = name_score(&a, &b);
        assert!(
            rescued > primary_only,
            "alternate should rescue: rescued {rescued} vs primary {primary_only}"
        );
        assert!(rescued >= 0.99, "alternate equals B's primary, got {rescued}");
        // Symmetric: the same rescue works when the alternate is on B.
        let a2 = CarePathway::new("Acute Stroke Care Pathway");
        let mut b2 = CarePathway::new("CVA Pathway");
        b2.alternate_names = vec!["Acute Stroke Care Pathway".into()];
        assert!(name_score(&a2, &b2) >= 0.99);
    }

    // One-to-many surface: `rank` orders the exact match first (index 1),
    // `find_matches` returns only `is_match` entries, and the empty-input
    // path yields an empty result.
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
