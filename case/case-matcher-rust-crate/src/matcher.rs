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

use crate::case::{Case, IdentifierScheme};
use crate::config::MatchConfig;
use crate::evidence::{self, Contradiction, NoFrequencies, ValueFrequencies};
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
    /// **The caller must bound `subjects`/`keywords` array sizes.**
    /// This crate has no length cap of its own; both fields are scored
    /// by set Jaccard over every element, an unbounded O(n·m)
    /// operation. See the crate-root docs ("The caller must bound array
    /// sizes") for the reference cap (`case-service`'s `MAX_ARRAY_LEN` /
    /// `MAX_ITEM_LEN`) a standalone integrator should copy.
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
        self.match_cases_with_frequencies(a, b, &NoFrequencies)
    }

    /// Score two cases, weighing an exact agreement on a
    /// [`evidence::FREQUENCY_COMPONENTS`] value by that value's own share
    /// from `frequencies` (`agents/share/match-evidence.md` §7).
    ///
    /// Only `evidence` can differ from [`MatchingEngine::match_cases`]:
    /// `score`, `is_match`, `confidence` and the breakdown are identical.
    /// Passing [`NoFrequencies`] gives exactly the plain call's result.
    #[must_use]
    pub fn match_cases_with_frequencies(
        &self,
        a: &Case,
        b: &Case,
        frequencies: &dyn ValueFrequencies,
    ) -> MatchResult {
        // Components are computed on both paths: the probabilistic score
        // needs them, and so does the evidence (match-evidence.md §3 — a
        // short-circuit still weighs the components it can compute). Each
        // returns `None` when one or both records lack the data. `title`
        // is always present (title is required).
        let components = MatchBreakdown {
            title_score: Some(title_score(a, b)),
            subjects_score: set_jaccard(&a.subjects, &b.subjects),
            case_number_score: case_number_score(a, b),
            case_type_score: case_type_score(a, b),
            status_score: status_score(a, b),
            keywords_score: set_jaccard(&a.keywords, &b.keywords),
            deterministic_match: false,
        };
        let contradictions = contradictions(a, b);
        // A share is looked up only when both primary titles have the same
        // key, i.e. the records hold the same normalised value.
        let mut shares: Vec<(&str, f64)> = Vec::new();
        if let (Some(ka), Some(kb)) = (
            evidence::frequency_key("title", &a.title),
            evidence::frequency_key("title", &b.title),
        ) && ka == kb
            && let Some(share) = frequencies.share("title", &ka)
        {
            shares.push(("title", share));
        }

        // Phase 1: deterministic short-circuit. A globally-unique
        // identifier match, a same-agency case-number match, or a
        // `same_as` URL overlap is conclusive — pin the result to a
        // certain match.
        if deterministic_match(a, b) {
            let evidence = evidence::assess(
                &components,
                true,
                contradictions,
                &shares,
                true,
                self.config.proof_bits,
            );
            return MatchResult {
                score: 1.0,
                is_match: true,
                confidence: Confidence::High,
                breakdown: MatchBreakdown {
                    deterministic_match: true,
                    // All per-component scores stay `None` in the
                    // breakdown: when the deterministic rule fired, the
                    // fuzzy components do not contribute to the score and
                    // must not appear to.
                    ..Default::default()
                },
                evidence,
            };
        }

        // Phase 2: probabilistic scoring. Weighted average over only the
        // present components: absent components neither add to the
        // numerator nor the denominator.
        let score = weighted_average(&[
            (components.title_score, self.config.title_weight),
            (components.subjects_score, self.config.subjects_weight),
            (components.case_number_score, self.config.case_number_weight),
            (components.case_type_score, self.config.case_type_weight),
            (components.status_score, self.config.status_weight),
            (components.keywords_score, self.config.keywords_weight),
        ]);

        // `is_match` is purely a threshold comparison; the same `score`
        // can be a match under `lenient` and a non-match under `strict`.
        let is_match = score >= self.config.threshold;
        let evidence = evidence::assess(
            &components,
            false,
            contradictions,
            &shares,
            is_match,
            self.config.proof_bits,
        );
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: components,
            evidence,
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

/// SEC-M2: decide whether a normalised identifier value is *trivial* and
/// therefore not identity evidence.
///
/// A value is trivial when it carries no alphanumeric character that is
/// anything other than `'0'` — i.e. it is empty / punctuation-only, the
/// sentinel `"0"`, or an all-zeros UUID
/// (`"00000000-0000-0000-0000-000000000000"`). Such placeholders must not
/// let two *different* cases short-circuit to a certain match.
///
/// `s` is the already-folded value. Returns `true` when the value is
/// trivial (skip it), `false` when it is a real identifier.
fn is_trivial_identifier(s: &str) -> bool {
    !s.chars().any(|c| c.is_alphanumeric() && c != '0')
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
        // SEC-M2: also skip *trivial* sentinels — a placeholder id like
        // `"0"` or an all-zeros UUID is not identity evidence, so two
        // different cases carrying it must not deterministically match.
        let av = normalize::fold(&ai.value);
        if is_trivial_identifier(&av) {
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
        // SEC-M2: skip trivial / non-identity URLs — empty, or a bare
        // root `"/"` (`normalize::url` deliberately keeps a lone slash) —
        // so two different cases sharing only `"/"` do not short-circuit.
        if an.is_empty() || an == "/" {
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

/// The hard conflicts between `a` and `b` (match-evidence.md §3).
///
/// For each deterministic scheme, both records hold at least one
/// non-trivial folded value and the two value sets share none →
/// `identifier:<scheme>`. The same agency with two different non-empty
/// normalised case numbers → `identifier:case_number`. A record holding
/// several values under one scheme is not contradicted by a partner
/// holding any one of them; blank and sentinel values (SEC-M2) are
/// ignored. Returns the contradictions in a fixed scheme order, so the
/// result is symmetric in `a` and `b`.
fn contradictions(a: &Case, b: &Case) -> Vec<Contradiction> {
    let values = |c: &Case, scheme: &IdentifierScheme| -> Vec<String> {
        c.identifiers
            .iter()
            .filter(|i| &i.scheme == scheme)
            .map(|i| normalize::fold(&i.value))
            .filter(|v| !is_trivial_identifier(v))
            .collect()
    };
    let mut out = Vec::new();
    for (scheme, name) in [
        (IdentifierScheme::Docket, "docket"),
        (IdentifierScheme::ExternalCaseId, "externalcaseid"),
        (IdentifierScheme::Uri, "uri"),
        (IdentifierScheme::Uuid, "uuid"),
    ] {
        let av = values(a, &scheme);
        let bv = values(b, &scheme);
        if !av.is_empty() && !bv.is_empty() && !av.iter().any(|v| bv.contains(v)) {
            out.push(Contradiction {
                field: format!("identifier:{name}"),
            });
        }
    }
    if let (Some(ak), Some(bk), Some(ac), Some(bc)) = (
        agency_key(a),
        agency_key(b),
        a.case_number.as_deref(),
        b.case_number.as_deref(),
    ) {
        let (an, bn) = (normalize::case_number(ac), normalize::case_number(bc));
        if ak == bk && !an.is_empty() && !bn.is_empty() && an != bn {
            out.push(Contradiction {
                field: "identifier:case_number".into(),
            });
        }
    }
    out
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

    // SEC-M2: a *trivial* shared identifier (the sentinel `"0"` or an
    // all-zeros UUID) is not identity evidence — two DIFFERENT cases that
    // carry only such a placeholder must NOT deterministically match.
    #[test]
    fn trivial_zero_identifier_does_not_short_circuit() {
        let engine = MatchingEngine::default_config();
        let mut zero_a = Case::new("Alpha");
        let mut zero_b = Case::new("Omega");
        zero_a
            .identifiers
            .push(ident(IdentifierScheme::Docket, "0"));
        zero_b
            .identifiers
            .push(ident(IdentifierScheme::Docket, "0"));
        assert!(!deterministic_match(&zero_a, &zero_b));
        let r = engine.match_cases(&zero_a, &zero_b);
        assert!(!r.breakdown.deterministic_match);
        assert!(!r.is_match, "got {}", r.score);

        // An all-zeros UUID is likewise a placeholder, not identity.
        let mut uuid_a = Case::new("Alpha");
        let mut uuid_b = Case::new("Omega");
        let zeros = "00000000-0000-0000-0000-000000000000";
        uuid_a
            .identifiers
            .push(ident(IdentifierScheme::Uuid, zeros));
        uuid_b
            .identifiers
            .push(ident(IdentifierScheme::Uuid, zeros));
        assert!(!deterministic_match(&uuid_a, &uuid_b));

        // Positive control: a real shared id still short-circuits.
        let mut real_a = Case::new("Alpha");
        let mut real_b = Case::new("Omega");
        real_a
            .identifiers
            .push(ident(IdentifierScheme::Docket, "CV-2024-001234"));
        real_b
            .identifiers
            .push(ident(IdentifierScheme::Docket, "CV-2024-001234"));
        assert!(deterministic_match(&real_a, &real_b));
    }

    // SEC-M2: a bare root `same_as` URL (`"/"`, which `normalize::url`
    // deliberately keeps non-empty) is not identity evidence — two
    // DIFFERENT cases sharing only `"/"` must NOT short-circuit, while a
    // real shared URL still does.
    #[test]
    fn trivial_root_same_as_does_not_short_circuit() {
        let engine = MatchingEngine::default_config();
        let mut root_a = Case::new("Alpha");
        let mut root_b = Case::new("Omega");
        root_a.same_as = vec!["/".into()];
        root_b.same_as = vec!["/".into()];
        assert!(!deterministic_match(&root_a, &root_b));
        let r = engine.match_cases(&root_a, &root_b);
        assert!(!r.breakdown.deterministic_match);
        assert!(!r.is_match, "got {}", r.score);

        // Positive control: a real shared identity URL still short-circuits.
        let mut url_a = Case::new("Alpha");
        let mut url_b = Case::new("Omega");
        url_a.same_as = vec!["https://courts.example.gov/case/CV-2024-001234".into()];
        url_b.same_as = vec!["https://courts.example.gov/case/CV-2024-001234".into()];
        assert!(deterministic_match(&url_a, &url_b));
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

    // Pins the Soundex +0.05 title bonus (PHONETIC_BONUS / PHONETIC_CEILING,
    // spec §9): a Smith/Smyth-style primary-title pair whose literal
    // Jaro-Winkler is below the ceiling is lifted by phonetic agreement.
    // The bonused score must (a) exceed the raw Jaro-Winkler and (b) never
    // exceed the ceiling.
    #[test]
    fn title_soundex_bonus_lifts_phonetic_near_miss() {
        let a = Case::new("Smith");
        let b = Case::new("Smyth");
        // Sanity: the two encode to the same Soundex code, and the raw
        // Jaro-Winkler is below the phonetic ceiling (so the bonus applies).
        assert!(phonetic::same("smith", "smyth"));
        let raw = jaro_winkler("smith", "smyth");
        assert!(raw < PHONETIC_CEILING, "raw {raw} should be below ceiling");
        let scored = title_score(&a, &b);
        assert!(scored > raw, "bonus should lift {raw} but got {scored}");
        assert!(
            (scored - (raw + PHONETIC_BONUS)).abs() < 1e-9,
            "got {scored}"
        );
        assert!(
            scored <= PHONETIC_CEILING,
            "must not exceed ceiling: {scored}"
        );
    }

    // Pins that the phonetic bonus never lifts a near-miss above the
    // ceiling: even when (raw + bonus) would exceed 0.95, the result is
    // clamped to PHONETIC_CEILING and the bonus is symmetric in argument
    // order.
    #[test]
    fn title_soundex_bonus_is_clamped_and_symmetric() {
        // "Jackson" / "Jaksen": same Soundex (J250), raw JW close to but
        // below the ceiling — the bonus would push past 0.95 if unclamped.
        let a = Case::new("Jackson");
        let b = Case::new("Jaksen");
        if phonetic::same("jackson", "jaksen") {
            let raw = jaro_winkler("jackson", "jaksen");
            if raw < PHONETIC_CEILING {
                let s = title_score(&a, &b);
                assert!(s <= PHONETIC_CEILING + 1e-12, "clamped: {s}");
                // Symmetry: order of arguments must not change the score.
                assert!((s - title_score(&b, &a)).abs() < 1e-9);
            }
        }
    }

    // Pins the alternate_titles contribution to title_score (spec §9): an
    // alternate on either side that matches the other record's primary
    // title carries the component via the best-over-alternates rule, and
    // the result is symmetric in argument order.
    #[test]
    fn alternate_titles_carry_title_score() {
        let primary = Case::new("Housing benefit appeal — J. Smith");
        // `other` has an unrelated primary but a matching alternate title.
        let mut other = Case::new("Matter 2024-XYZ");
        other
            .alternate_titles
            .push("Housing benefit appeal — J. Smith".into());
        // Best-over-alternates should find the strong match (≈1.0), far
        // above the weak primary-vs-primary score.
        let with_alt = title_score(&primary, &other);
        let primary_only = jaro_winkler(
            &normalize::fold(&primary.title),
            &normalize::fold("Matter 2024-XYZ"),
        );
        assert!(
            with_alt > primary_only,
            "alternate should lift {primary_only} but got {with_alt}"
        );
        assert!(
            with_alt >= 0.99,
            "alternate match should be near-perfect: {with_alt}"
        );
        // Symmetric: the alternate may sit on either side.
        assert!((with_alt - title_score(&other, &primary)).abs() < 1e-9);
    }

    // Pins the keywords component (spec §13, weight 0.15): keywords use the
    // same Jaccard path as subjects. A full overlap scores 1.0, a partial
    // overlap scores the Jaccard ratio, one-sided scores 0.0, both-empty is
    // skipped (`None`) — and the engine surfaces it in the breakdown.
    #[test]
    fn keywords_jaccard_and_skip() {
        // Full overlap (case-folded) → 1.0.
        let a = vec!["Eviction".to_string(), "Arrears".to_string()];
        let b = vec!["eviction".to_string(), "ARREARS".to_string()];
        assert_eq!(set_jaccard(&a, &b), Some(1.0));
        // Partial overlap {1,2} vs {1} → 0.5.
        let got = set_jaccard(&a, &["eviction".to_string()]).expect("some");
        assert!((got - 0.5).abs() < 1e-9, "got {got}");
        // One side populated, other empty → 0.0.
        assert_eq!(set_jaccard(&a, &[]), Some(0.0));
        // Both empty → component absent.
        assert_eq!(set_jaccard(&[], &[]), None);

        // End-to-end: the engine populates keywords_score when present and
        // leaves it None when both sides are empty.
        let engine = MatchingEngine::default_config();
        let mut ka = Case::new("Tenancy dispute");
        let mut kb = Case::new("Tenancy dispute");
        ka.keywords = vec!["eviction".into(), "arrears".into()];
        kb.keywords = vec!["eviction".into()];
        let r = engine.match_cases(&ka, &kb);
        assert_eq!(r.breakdown.keywords_score, Some(0.5));
        // No keywords on either side → skipped.
        let plain = engine.match_cases(&Case::new("X"), &Case::new("X"));
        assert!(plain.breakdown.keywords_score.is_none());
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

    // ─── Evidence (agents/share/match-evidence.md) ───────────────

    use crate::evidence::{IDENTIFIER_BITS, MatchStrength};

    /// A richly populated case with no deterministic identifier and no
    /// case number (a shared same-agency case number would fire R-1).
    fn rich_case() -> Case {
        let mut c = Case::new("Housing benefit appeal — J. Smith");
        c.subjects = vec!["John Smith".into(), "Leeds City Council".into()];
        c.keywords = vec!["housing".into(), "appeal".into()];
        c.case_type = Some(CaseType::Benefit);
        c.status = Some(CaseStatus::Open);
        c
    }

    // Identical title-only records are a match on similarity but prove
    // little: a template title is shared by unrelated cases.
    #[test]
    fn identical_sparse_records_are_consistent_not_proven() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let r = engine.match_cases(&a, &a.clone());
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        assert_eq!(r.evidence.fields_compared, 1);
        assert!(r.evidence.weight_bits > 0.0 && r.evidence.weight_bits < 30.0);
    }

    // A shared globally-unique identifier clears the bar alone, even with
    // unrelated titles; the returned score/breakdown are unchanged.
    #[test]
    fn shared_identifier_is_proven() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Alpha");
        let mut b = Case::new("Omega");
        a.identifiers
            .push(ident(IdentifierScheme::Docket, "CV-2024-001234"));
        b.identifiers
            .push(ident(IdentifierScheme::Docket, "cv-2024-001234"));
        let r = engine.match_cases(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-12);
        assert!(r.breakdown.deterministic_match);
        assert!(r.breakdown.title_score.is_none());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        assert!(r.evidence.weight_bits >= IDENTIFIER_BITS - 10.0);
        assert!(r.evidence.contradictions.is_empty());
    }

    // Same-agency case number (R-1) is the identifier: weighed once, not
    // again as the case-number component.
    #[test]
    fn same_agency_case_number_is_proven_and_weighed_once() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Housing benefit appeal — J. Smith");
        a.agency_id = Some("leeds".into());
        a.case_number = Some("HB-2024-77".into());
        let r = engine.match_cases(&a, &a.clone());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        // title + identifier only.
        assert_eq!(r.evidence.fields_compared, 2);
    }

    // Conflicting docket numbers on otherwise identical records are
    // contradicted, at any score.
    #[test]
    fn conflicting_identifiers_are_contradicted() {
        let engine = MatchingEngine::default_config();
        let mut a = rich_case();
        let mut b = rich_case();
        a.identifiers.push(ident(IdentifierScheme::Docket, "CV-1"));
        b.identifiers.push(ident(IdentifierScheme::Docket, "CV-2"));
        let r = engine.match_cases(&a, &b);
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(r.evidence.contradictions[0].field, "identifier:docket");

        // A partner holding either of two values is not contradicted.
        a.identifiers.push(ident(IdentifierScheme::Docket, "CV-2"));
        let r = engine.match_cases(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    // Same agency, different case numbers: a contradiction.
    #[test]
    fn same_agency_different_case_numbers_are_contradicted() {
        let engine = MatchingEngine::default_config();
        let mut a = rich_case();
        let mut b = rich_case();
        a.agency_id = Some("leeds".into());
        b.agency_id = Some("leeds".into());
        a.case_number = Some("HB-1".into());
        b.case_number = Some("HB-2".into());
        let r = engine.match_cases(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(r.evidence.contradictions[0].field, "identifier:case_number");
    }

    // Blank and sentinel identifiers cannot contradict (SEC-M2).
    #[test]
    fn trivial_identifiers_do_not_contradict() {
        let engine = MatchingEngine::default_config();
        let mut a = rich_case();
        let mut b = rich_case();
        a.identifiers.push(ident(IdentifierScheme::Uuid, "0"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "CV-9"));
        let r = engine.match_cases(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
    }

    // Pins that a case can NOT be proven from its fields alone under the
    // default table: title 7.57 + ½(subjects 6.41 + keywords 4.00) +
    // type 2.66 + ½ status 1.68 ≈ 16.28 bits after the correlated-group
    // discount (22.3 before it), short of 30. A case's descriptive fields are
    // shared by related matters (one claimant's several appeals), so only
    // an identifier or a same-agency case number proves identity.
    #[test]
    fn richest_identical_record_without_identifier_is_not_proven() {
        let engine = MatchingEngine::default_config();
        let a = rich_case();
        let r = engine.match_cases(&a, &a.clone());
        assert!(r.is_match);
        assert_eq!(r.evidence.fields_compared, 5);
        assert!(
            (r.evidence.weight_bits - 16.28).abs() < 0.01,
            "got {}",
            r.evidence.weight_bits
        );
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
    }

    // A clear disagreement gives negative evidence.
    #[test]
    fn clear_disagreement_is_negative() {
        let engine = MatchingEngine::default_config();
        let a = rich_case();
        let mut b = Case::new("Commercial driving licence renewal");
        b.subjects = vec!["Acme Haulage Ltd".into()];
        b.keywords = vec!["licensing".into()];
        b.case_type = Some(CaseType::Legal);
        b.status = Some(CaseStatus::Pending);
        let r = engine.match_cases(&a, &b);
        assert!(
            r.evidence.weight_bits < 0.0,
            "got {}",
            r.evidence.weight_bits
        );
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
    }

    // ─── Correlated groups and value frequencies ─────────────────

    use crate::evidence::{FrequencyTable, NoFrequencies, frequency_key};

    // One claimant's two separate cases: templated title, same type, same
    // subjects, different (but unknown) numbers. Related matters, not one
    // case, so they must not be Proven, with or without frequencies.
    #[test]
    fn related_matters_of_one_claimant_are_not_proven() {
        let engine = MatchingEngine::default_config();
        let mut a = Case::new("Housing Benefit Claim");
        a.subjects = vec!["John Smith".into()];
        a.case_type = Some(CaseType::Benefit);
        let mut b = a.clone();
        a.status = Some(CaseStatus::Closed);
        b.status = Some(CaseStatus::Open);
        let plain = engine.match_cases(&a, &b);
        assert_ne!(plain.evidence.strength, MatchStrength::Proven);
        assert!(
            plain.evidence.weight_bits < 30.0,
            "got {}",
            plain.evidence.weight_bits
        );

        let mut table = FrequencyTable::default();
        let key = frequency_key("title", &a.title).expect("key");
        table.insert("title", &key, 0.2);
        let freq = engine.match_cases_with_frequencies(&a, &b, &table);
        assert_ne!(freq.evidence.strength, MatchStrength::Proven);
        assert!(freq.evidence.weight_bits < plain.evidence.weight_bits);
    }

    // A templated title held by a fifth of all cases proves little:
    // log2(0.95 / 0.2) ≈ 2.25 bits, against 7.57 from the default u.
    #[test]
    fn common_templated_title_proves_little() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing Benefit Claim");
        let b = Case::new("  housing benefit claim ");
        let table =
            FrequencyTable::from_counts(&[("title", "housing benefit claim", 2000)], 10_000);
        let r = engine.match_cases_with_frequencies(&a, &b, &table);
        let title = &r.evidence.components[0];
        assert_eq!(title.component, "title");
        assert!(title.frequency_adjusted);
        assert!(
            (title.bits - (0.95_f64 / 0.2).log2()).abs() < 1e-9,
            "got {}",
            title.bits
        );
        assert!(title.bits < 2.5);
        // score / is_match / confidence are untouched by frequencies.
        let plain = engine.match_cases(&a, &b);
        assert!((r.score - plain.score).abs() < 1e-15);
        assert_eq!(r.is_match, plain.is_match);
        assert_eq!(r.confidence, plain.confidence);
    }

    // A share is not used when the titles only nearly agree.
    #[test]
    fn near_agreeing_titles_are_not_frequency_adjusted() {
        let engine = MatchingEngine::default_config();
        let a = Case::new("Housing benefit appeal — J. Smith");
        let b = Case::new("Housing benefit appeal — John Smith");
        let mut table = FrequencyTable::default();
        for t in [&a.title, &b.title] {
            table.insert("title", &frequency_key("title", t).expect("key"), 0.5);
        }
        let r = engine.match_cases_with_frequencies(&a, &b, &table);
        assert!(!r.evidence.components[0].frequency_adjusted);
        assert_eq!(
            serde_json::to_value(&r.evidence).expect("json"),
            serde_json::to_value(&engine.match_cases(&a, &b).evidence).expect("json"),
        );
    }

    // `NoFrequencies` gives exactly the plain call.
    #[test]
    fn no_frequencies_equals_plain_call() {
        let engine = MatchingEngine::default_config();
        let mut a = rich_case();
        let b = rich_case();
        a.identifiers.push(ident(IdentifierScheme::Docket, "CV-1"));
        for (x, y) in [(&a, &b), (&b, &b)] {
            let plain = serde_json::to_value(engine.match_cases(x, y)).expect("json");
            let none =
                serde_json::to_value(engine.match_cases_with_frequencies(x, y, &NoFrequencies))
                    .expect("json");
            assert_eq!(plain, none);
        }
    }

    // The evidence listing names every term and sums to weight_bits,
    // including the identifier and contradiction terms.
    #[test]
    fn components_list_every_term() {
        let engine = MatchingEngine::default_config();
        let mut a = rich_case();
        let mut b = rich_case();
        a.identifiers.push(ident(IdentifierScheme::Docket, "CV-1"));
        b.identifiers.push(ident(IdentifierScheme::Docket, "CV-1"));
        a.identifiers.push(ident(IdentifierScheme::Uuid, "u-1"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "u-2"));
        let e = engine.match_cases(&a, &b).evidence;
        let names: Vec<&str> = e.components.iter().map(|c| c.component.as_str()).collect();
        assert!(names.contains(&"identifier"));
        assert!(names.contains(&"contradiction:identifier:uuid"));
        let sum: f64 = e.components.iter().map(|c| c.bits).sum();
        assert!((sum - e.weight_bits).abs() < 1e-12);
        assert!(e.components.iter().any(|c| c.correlated));
    }

    // The frequency key is the folded primary title; other components and
    // blank titles have none.
    #[test]
    fn frequency_key_is_the_compared_form() {
        assert_eq!(
            frequency_key("title", "  Housing Benefit CLAIM "),
            Some("housing benefit claim".into())
        );
        assert_eq!(frequency_key("title", "   "), None);
        assert_eq!(frequency_key("subjects", "John"), None);
    }
}
