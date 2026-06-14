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

/// Additive Soundex bonus applied to the title component when the two
/// titles share a phonetic code but the raw Jaro-Winkler score is still
/// below [`PHONETIC_CEILING`]. Small (`0.05`) by design: a phonetic
/// agreement is a weak corroborating signal, not a decisive one, so it
/// nudges near-misses upward without ever inventing a strong match.
const PHONETIC_BONUS: f64 = 0.05;

/// Upper bound the phonetic bonus may lift the title score to, and the
/// gate below which the bonus is considered at all. Capping at `0.95`
/// keeps the phonetic-only path strictly below a "certain" title match —
/// only literal string similarity can push the title component above
/// this ceiling.
const PHONETIC_CEILING: f64 = 0.95;

/// The case matcher: holds a [`MatchConfig`] and scores pairs.
///
/// Construct once with [`MatchingEngine::new`] (or
/// [`MatchingEngine::default_config`]) and reuse across many
/// [`MatchingEngine::match_cases`] calls — the engine is immutable and
/// holds no per-call state, so the same instance scores any number of
/// pairs.
pub struct MatchingEngine {
    /// The weights + threshold governing the probabilistic strategy.
    /// Borrowed read-only on every call; never mutated after construction.
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    ///
    /// `config` supplies the per-component weights and the probable-match
    /// threshold; pass a tuned [`MatchConfig`] to override the defaults.
    ///
    /// Returns the constructed engine.
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with [`MatchConfig::default`].
    ///
    /// Convenience constructor for the common case of "use the canonical
    /// weights and the `0.85` threshold". Returns the constructed engine.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    ///
    /// Lets callers inspect the active weights / threshold (e.g. to
    /// report which preset is in force). Returns a shared reference to the
    /// held [`MatchConfig`].
    #[must_use]
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two cases. Always returns a result (never errs).
    ///
    /// Runs the two-phase pipeline: first the deterministic
    /// short-circuit (`R-0`/`R-1`/`R-2`), which on a hit returns the
    /// pinned `1.0`/[`Confidence::High`] result with
    /// `breakdown.deterministic_match = true` and every component left
    /// `None`; otherwise the probabilistic strategy — per-component
    /// scores fed through [`weighted_average`], compared against the
    /// configured threshold.
    ///
    /// `a` and `b` are the two records to compare; argument order does
    /// not affect the score (the underlying components are symmetric).
    /// Returns the populated [`MatchResult`]. Infallible by contract —
    /// every input pair yields a score.
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
        // Phase 1: deterministic short-circuit. A globally-unique
        // identifier match, a same-agency case-number match, or a
        // `same_as` URL overlap is conclusive — skip scoring entirely
        // and pin the result to a certain match.
        if deterministic_match(a, b) {
            return MatchResult {
                score: 1.0,
                is_match: true,
                confidence: Confidence::High,
                breakdown: MatchBreakdown {
                    deterministic_match: true,
                    // All per-component scores stay `None`: when the
                    // deterministic rule fired, the fuzzy components were
                    // never computed and must not appear to contribute.
                    ..Default::default()
                },
            };
        }

        // Phase 2: probabilistic scoring. Each component is computed
        // independently. `title` is always present (title is required),
        // so it is wrapped in `Some`; every other component returns
        // `None` when one or both records lack the data, so it drops out
        // of the renormalised average rather than scoring 0.
        let title_score = Some(title_score(a, b));
        let subjects_score = set_jaccard(&a.subjects, &b.subjects);
        let case_number_score = case_number_score(a, b);
        let case_type_score = case_type_score(a, b);
        let status_score = status_score(a, b);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);

        // Weighted average over only the present components: absent
        // components neither add to the numerator nor the denominator.
        let score = weighted_average(&[
            (title_score, self.config.title_weight),
            (subjects_score, self.config.subjects_weight),
            (case_number_score, self.config.case_number_weight),
            (case_type_score, self.config.case_type_weight),
            (status_score, self.config.status_weight),
            (keywords_score, self.config.keywords_weight),
        ]);

        // `is_match` is purely a threshold comparison; the same `score`
        // can be a match under `lenient` and a non-match under `strict`.
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
    ///
    /// Scores `query` against every entry of `candidates`, preserving the
    /// candidate slice's order so result `i` corresponds to
    /// `candidates[i]`. Returns one [`MatchResult`] per candidate (an
    /// empty `Vec` for an empty slice). Use [`MatchingEngine::rank`] when
    /// you want them sorted instead.
    #[must_use]
    pub fn match_one_to_many(&self, query: &Case, candidates: &[Case]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_cases(query, c))
            .collect()
    }

    /// One-to-many: `(index, result)` sorted by descending score.
    ///
    /// Like [`MatchingEngine::match_one_to_many`] but the returned pairs
    /// are sorted best-first; each pair's `usize` is the candidate's
    /// original index in `candidates`, so the caller can recover which
    /// record produced each result after the reorder. Returns all
    /// candidates (this does NOT filter by threshold — see
    /// [`MatchingEngine::find_matches`]).
    #[must_use]
    pub fn rank(&self, query: &Case, candidates: &[Case]) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_cases(query, c)))
            .collect();
        // Descending by score (`b` vs `a`). Scores are finite `f64`s, but
        // `partial_cmp` is used defensively: any incomparable pair (e.g. a
        // hypothetical NaN) is treated as `Equal` rather than panicking,
        // honouring the crate's no-panic guarantee.
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Rank then drop everything below [`MatchConfig`]'s threshold.
    ///
    /// Convenience over [`MatchingEngine::rank`]: keeps only the
    /// candidates whose `is_match` flag is set (score ≥ the configured
    /// threshold), still sorted best-first with original indices.
    /// Returns an empty `Vec` when nothing clears the threshold.
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
///
/// A case number is only meaningful within its handling organisation, so
/// the `R-1` short-circuit and the `case_number` component both need a
/// stable per-agency key. `agency_id` is the opaque, authoritative key;
/// `agency_name` is a softer fallback used only when no id is present.
///
/// `c` is the record to key. Returns the borrowed key string, or `None`
/// when neither field is set (or both are empty) — in which case the
/// agency-scoped rules cannot apply.
fn agency_key(c: &Case) -> Option<&str> {
    c.agency_id
        .as_deref()
        .or(c.agency_name.as_deref())
        // Treat an empty string as "no key": two records with blank agency
        // ids must not be considered the same agency.
        .filter(|s| !s.is_empty())
}

/// Decide whether the deterministic short-circuit fires for `a` vs `b`.
///
/// Returns `true` if any of the three rules below hold, in which case the
/// caller pins the score to `1.0`. The rules are evaluated in priority
/// order but any single hit is conclusive:
///
/// - `R-0` — a shared value on the same globally-unique identifier scheme.
/// - `R-1` — the same agency and the same normalised case number.
/// - `R-2` — an overlapping `same_as` identity URL.
///
/// `a` and `b` are the two records; the function is symmetric in them.
fn deterministic_match(a: &Case, b: &Case) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value.
    // Agency-scoped / custom schemes are deliberately excluded (they are
    // not globally unique) via `is_deterministic`.
    for ai in &a.identifiers {
        if !ai.scheme.is_deterministic() {
            continue;
        }
        // Fold the value so case / whitespace / Unicode form differences
        // do not defeat the comparison; skip values that fold to empty.
        let av = normalize::fold(&ai.value);
        if av.is_empty() {
            continue;
        }
        for bi in &b.identifiers {
            // Scheme must match too: a `Docket` value equal to a `Uri`
            // value is coincidence, not identity.
            if ai.scheme == bi.scheme && av == normalize::fold(&bi.value) {
                return true;
            }
        }
    }

    // R-1 — same agency + same normalised case_number. Requires all four
    // of {agency key, case number} on both sides; the non-empty guard
    // stops two records with blank case numbers matching spuriously.
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

    // R-2 — any same_as URL overlaps (folded + trailing-slash-trimmed).
    // These are cross-system identity URLs (court record page, registry
    // entry); a shared one is as conclusive as a shared identifier.
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

/// Score the title component for `a` vs `b` in `[0.0, 1.0]`.
///
/// Titles are the primary identity signal, so this is the only component
/// that is always present. The algorithm is Jaro-Winkler (case-insensitive
/// after folding, with the prefix bonus characteristic of name-like
/// strings), taking the *best* score across each record's primary title
/// and all of its alternate titles — alternates exist precisely to catch
/// re-phrasings, so any of them may carry the match.
///
/// A Soundex agreement on the primary titles adds [`PHONETIC_BONUS`],
/// clamped to [`PHONETIC_CEILING`], but only when the literal score has
/// not already reached the ceiling — this rescues phonetic near-misses
/// (e.g. `Smith` / `Smyth`) without overstating them.
///
/// `a` and `b` are the records; returns the best title similarity found.
fn title_score(a: &Case, b: &Case) -> f64 {
    let an = normalize::fold(&a.title);
    let bn = normalize::fold(&b.title);
    let mut best = jaro_winkler(&an, &bn);
    // Try `a`'s alternates against `b`'s primary, keeping the maximum.
    for alt in &a.alternate_titles {
        best = best.max(jaro_winkler(&normalize::fold(alt), &bn));
    }
    // Symmetrically, try `b`'s alternates against `a`'s primary.
    for alt in &b.alternate_titles {
        best = best.max(jaro_winkler(&an, &normalize::fold(alt)));
    }
    // Phonetic bonus: only below the ceiling, and only on the primary
    // titles (alternates are not re-encoded). `min` clamps so the bonus
    // can never lift a near-miss above the ceiling.
    if best < PHONETIC_CEILING && phonetic::same(&an, &bn) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

/// Score the agency-scoped case-number component for `a` vs `b`.
///
/// A case number only identifies a case *within* its agency, so this is a
/// binary, agency-gated comparison rather than a fuzzy one. Both records
/// must carry a non-empty case number AND share an agency key (see
/// [`agency_key`]); otherwise the component is skipped so it cannot drag a
/// genuine cross-agency match down with noise.
///
/// `a` and `b` are the records. Returns:
/// - `Some(1.0)` — same agency, equal normalised case numbers;
/// - `Some(0.0)` — same agency, differing case numbers (a real
///   distinguishing signal);
/// - `None` — a case number is missing, or the agencies differ / are
///   unknown (the component does not apply).
fn case_number_score(a: &Case, b: &Case) -> Option<f64> {
    let (ac, bc) = match (a.case_number.as_deref(), b.case_number.as_deref()) {
        // Both case numbers must be present and non-empty to compare.
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };
    // Across-agency case numbers are noise. Only contribute when both
    // records share an agency.
    match (agency_key(a), agency_key(b)) {
        (Some(ak), Some(bk)) if ak == bk => {
            // Normalise (strip punctuation, uppercase) before comparing so
            // `CV-2024-001234` and `cv 2024 001234` are treated as equal.
            if normalize::case_number(ac) == normalize::case_number(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        // Different or unknown agency → the case number says nothing.
        _ => None,
    }
}

/// Score the case-type component for `a` vs `b`.
///
/// Case type is a categorical signal, so the comparison is exact equality
/// rather than fuzzy. `a` and `b` are the records. Returns `Some(1.0)`
/// when both types are present and equal, `Some(0.0)` when both are
/// present but differ, and `None` when either record omits its type.
fn case_type_score(a: &Case, b: &Case) -> Option<f64> {
    match (&a.case_type, &b.case_type) {
        (Some(x), Some(y)) => Some(if x == y { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Score the status component for `a` vs `b`.
///
/// Lifecycle status is categorical, so — like case type — it is compared
/// by exact equality. `a` and `b` are the records. Returns `Some(1.0)`
/// for equal present statuses, `Some(0.0)` for differing present statuses
/// (a weak distinguishing signal: the same matter at different lifecycle
/// stages), and `None` when either status is absent.
fn status_score(a: &Case, b: &Case) -> Option<f64> {
    match (&a.status, &b.status) {
        (Some(x), Some(y)) => Some(if x == y { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Jaccard similarity of two string sets (used for `subjects` and
/// `keywords`).
///
/// Jaccard = |A ∩ B| / |A ∪ B|, computed over the folded + deduplicated
/// sets, so it rewards overlap while staying insensitive to ordering,
/// duplicates, case, and Unicode form. Used for the two list-valued
/// components.
///
/// `a` and `b` are the raw string slices. Returns:
/// - `None` — both sides are empty (before or after folding), so the
///   component does not apply and is renormalised away;
/// - `Some(0.0)` — exactly one side is empty (no possible overlap);
/// - `Some(ratio)` — the Jaccard ratio in `[0.0, 1.0]` otherwise.
fn set_jaccard(a: &[String], b: &[String]) -> Option<f64> {
    // Fast path: nothing on either side → component absent.
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    // Folding can empty a side (e.g. only blank entries) — re-check.
    if a_set.is_empty() && b_set.is_empty() {
        return None;
    }
    // One side populated, the other empty: zero overlap is meaningful.
    if a_set.is_empty() || b_set.is_empty() {
        return Some(0.0);
    }
    // |A ∩ B| then |A ∪ B| via inclusion-exclusion to avoid a second pass.
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        Some(0.0)
    } else {
        // Both sets are non-empty here, so `union > 0`; convert losslessly
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
    use crate::case::{CaseIdentifier, CaseStatus, CaseType, IdentifierScheme};

    /// Test helper: build a `CaseIdentifier` from a scheme + value without
    /// the struct-literal boilerplate.
    fn ident(scheme: IdentifierScheme, value: &str) -> CaseIdentifier {
        CaseIdentifier {
            scheme,
            value: value.into(),
        }
    }

    // Pins: two identical titles drive the probabilistic score to ~1.0
    // and clear the default threshold (no deterministic ids involved).
    #[test]
    fn identical_cases_score_high() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let b = Case::new("Housing benefit appeal — J. Smith");
        let r = engine.match_cases(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
    }

    // Pins R-0: a shared `Docket` value pins the score to 1.0 and sets
    // `deterministic_match`, even though the titles are unrelated; also
    // checks docket values are case-folded before comparison.
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

    // Pins R-0 for the `ExternalCaseId` scheme: another globally-unique
    // scheme short-circuits to 1.0 regardless of title similarity.
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

    // Pins the agency-gating of case numbers: with no agency there is no
    // short-circuit; across different agencies the component is skipped
    // (`None`); only a shared agency makes the case number conclusive.
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

    // Pins the `agency_name` fallback: when `agency_id` is absent, a
    // shared `agency_name` still gates R-1, and the case-number
    // normalisation collapses the hyphen/space formatting difference.
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

    // Pins R-2: an overlapping `same_as` URL short-circuits to 1.0, and
    // the URL normaliser tolerates surrounding whitespace.
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

    // Pins the subjects component: a shared subject (folded, so case
    // differs) scores 1.0 and, combined with a strong title, pushes the
    // pair over the threshold without any deterministic id.
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

    // Pins the Jaccard math directly: {1,2} vs {1} = 1/2 = 0.5, and two
    // empty sets yield `None` (component absent).
    #[test]
    fn subjects_jaccard_partial_and_skip() {
        let a = vec!["person:1".to_string(), "person:2".to_string()];
        let b = vec!["person:1".to_string()];
        let got = set_jaccard(&a, &b).expect("some");
        assert!((got - 0.5).abs() < 1e-9, "got {got}");
        assert_eq!(set_jaccard(&[], &[]), None);
    }

    // Pins case-type scoring: equal → 1.0, differing → 0.0, one absent
    // → `None` (skipped).
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

    // Pins status scoring: equal → 1.0, differing → 0.0, one absent
    // → `None` (skipped).
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

    // Pins the negative case: dissimilar titles with no corroborating
    // components fall below the threshold and into the `Low` band.
    #[test]
    fn unrelated_cases_score_low() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let b = Case::new("Commercial driving licence renewal");
        let r = engine.match_cases(&a, &b);
        assert!(!r.is_match, "got {}", r.score);
        assert_eq!(r.confidence, crate::scoring::Confidence::Low);
    }

    // Pins the one-to-many surface: `rank` sorts best-first (the exact
    // duplicate at index 1 ranks first), `find_matches` returns only
    // above-threshold results, and an empty candidate list is handled.
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
