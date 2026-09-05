//! `MatchingEngine` — the public entry point.
//!
//! Two phases (the former kind gate is gone — the four kinds were unified
//! into one recursive plan tree, so any two plans may match
//! regardless of their now-optional, descriptive `kind`):
//!
//! 1. **Deterministic short-circuit.** If both records share a value on a
//!    deterministic identifier scheme (the tool/registry ids plus URI /
//!    UUID) OR share `owner_org_id` + normalised `code` OR overlap on a
//!    `same_as` URL, return score `1.0`.
//! 2. **Probabilistic scoring.** Per-component scores, then a weighted
//!    average over the *present* components.

use strsim::jaro_winkler;

use crate::config::MatchConfig;
use crate::normalize;
use crate::phonetic;
use crate::plan::{Plan, PlanRelationship, RelationKind};
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

/// Additive Soundex bonus applied to the name component when the two
/// names share a phonetic code but the raw Jaro-Winkler score is still
/// below [`PHONETIC_CEILING`]. Small (`0.05`) by design: a phonetic
/// agreement is a weak corroborating signal, not a decisive one.
const PHONETIC_BONUS: f64 = 0.05;

/// Upper bound the phonetic bonus may lift the name score to, and the
/// gate below which the bonus is considered at all. Capping at `0.95`
/// keeps the phonetic-only path strictly below a "certain" name match.
const PHONETIC_CEILING: f64 = 0.95;

/// The plan matcher: holds a [`MatchConfig`] and scores pairs.
///
/// Construct once with [`MatchingEngine::new`] (or
/// [`MatchingEngine::default_config`]) and reuse across many
/// [`MatchingEngine::match_plans`] calls — the engine is immutable
/// and holds no per-call state.
pub struct MatchingEngine {
    /// The weights + threshold governing the probabilistic strategy.
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    ///
    /// `config` supplies the per-component weights, timeframe σ, and the
    /// probable-match threshold. Returns the constructed engine.
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with [`MatchConfig::default`].
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration.
    #[must_use]
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two plans. Always returns a result (never errs).
    ///
    /// Runs the two-phase pipeline: the deterministic short-circuit
    /// (`R-0`/`R-1`/`R-2` → pinned `1.0`/[`Confidence::High`],
    /// `deterministic_match`), then the probabilistic strategy —
    /// per-component scores fed through [`weighted_average`], compared
    /// against the configured threshold. There is no kind gate.
    ///
    /// `a` and `b` are the two records to compare; argument order does
    /// not affect the score. Returns the populated [`MatchResult`].
    /// Infallible by contract.
    ///
    /// **The caller must bound `goals`/`keywords`/`relationships`/`tags`
    /// array sizes.** This crate has no length cap of its own; each is
    /// scored by Jaccard over every element, an unbounded O(n·m)
    /// operation. See the crate-root docs ("The caller must bound array
    /// sizes") for the reference cap
    /// (`project-portfolio-management-service`'s `MAX_ARRAY_LEN` /
    /// `MAX_ITEM_LEN`) a standalone integrator should copy.
    ///
    /// # Examples
    ///
    /// ```
    /// use project_portfolio_management_matcher::{Plan, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = Plan::new("Apollo platform migration");
    /// let b = Plan::new("Apollo platform migration");
    /// let r = engine.match_plans(&a, &b);
    /// assert!((0.0..=1.0).contains(&r.score));
    /// ```
    #[must_use]
    pub fn match_plans(&self, a: &Plan, b: &Plan) -> MatchResult {
        // The four kinds were unified into one recursive plan tree,
        // so there is no longer a kind gate: any two plans may match
        // regardless of their (now optional, descriptive) `kind`.
        // Phase 1: deterministic short-circuit. A globally-unique
        // identifier match, a same-owner code match, or a `same_as` URL
        // overlap is conclusive — pin the result to a certain match.
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

        // Phase 2: probabilistic scoring. `name` is always present (name
        // is required); every other component returns `None` when one or
        // both records lack the data, so it drops out of the renormalised
        // average rather than scoring 0.
        let name_score = Some(name_score(a, b));
        let goals_score = set_jaccard(&goal_titles(a), &goal_titles(b));
        let code_score = code_score(a, b);
        let owner_org_score = owner_org_score(a, b);
        let parent_score = parent_score(a, b);
        let timeframe_score = timeframe_score(a, b, self.config.timeframe_sigma_days);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);
        let relationships_score = set_jaccard_strict(
            &relationship_tokens(&a.relationships),
            &relationship_tokens(&b.relationships),
        );
        let tags_score = set_jaccard_strict(&a.tags, &b.tags);

        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (goals_score, self.config.goals_weight),
            (code_score, self.config.code_weight),
            (owner_org_score, self.config.owner_org_weight),
            (parent_score, self.config.parent_weight),
            (timeframe_score, self.config.timeframe_weight),
            (keywords_score, self.config.keywords_weight),
            (relationships_score, self.config.relationships_weight),
            (tags_score, self.config.tags_weight),
        ]);

        let is_match = score >= self.config.threshold;
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: MatchBreakdown {
                name_score,
                goals_score,
                code_score,
                owner_org_score,
                parent_score,
                timeframe_score,
                keywords_score,
                relationships_score,
                tags_score,
                deterministic_match: false,
                kind_gate_blocked: false,
            },
        }
    }

    /// One-to-many: results in input order.
    ///
    /// Scores `query` against every entry of `candidates`, preserving
    /// order so result `i` corresponds to `candidates[i]`.
    #[must_use]
    pub fn match_one_to_many(&self, query: &Plan, candidates: &[Plan]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_plans(query, c))
            .collect()
    }

    /// One-to-many: `(index, result)` sorted by descending score. Each
    /// pair's `usize` is the candidate's original index.
    #[must_use]
    pub fn rank(&self, query: &Plan, candidates: &[Plan]) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_plans(query, c)))
            .collect();
        // Descending by score; `partial_cmp` defensively treats any
        // incomparable pair as Equal rather than panicking.
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Rank then drop everything below [`MatchConfig`]'s threshold.
    #[must_use]
    pub fn find_matches(&self, query: &Plan, candidates: &[Plan]) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

/// The owner key both records must share for the owner-scoped rules:
/// the opaque `owner_org_id`. `owner_org_name` is deliberately NOT a
/// fallback — the owner scope keys solely on the id, never on a fuzzy
/// organisation-name comparison.
///
/// `w` is the record to key. Returns the borrowed key, or `None` when
/// `owner_org_id` is unset or empty.
fn owner_key(w: &Plan) -> Option<&str> {
    w.owner_org_id.as_deref().filter(|s| !s.is_empty())
}

/// Decide whether the deterministic short-circuit fires for `a` vs `b`
/// (already known to be the same kind). Any single rule is conclusive:
///
/// - `R-0` — a shared value on the same globally-unique identifier scheme.
/// - `R-1` — the same `owner_org_id` and the same normalised `code`.
/// - `R-2` — an overlapping `same_as` identity URL.
fn deterministic_match(a: &Plan, b: &Plan) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value. Owner-
    // scoped / custom schemes are excluded via `is_deterministic`.
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

    // R-1 — same owner + same normalised code. Requires all four of
    // {owner key, code} on both sides; the non-empty guard stops two
    // records with blank codes matching spuriously.
    if let (Some(ak), Some(bk), Some(ac), Some(bc)) = (
        owner_key(a),
        owner_key(b),
        a.code.as_deref(),
        b.code.as_deref(),
    ) && ak == bk
        && !normalize::code(ac).is_empty()
        && normalize::code(ac) == normalize::code(bc)
    {
        return true;
    }

    // R-2 — any same_as URL overlaps (folded + trailing-slash-trimmed).
    for au in &a.same_as {
        let an = normalize::url(au);
        // SEC-M2: skip trivial / non-identity URLs — empty, or a bare
        // root `"/"` (`normalize::url` deliberately keeps a lone slash
        // non-empty) — so two different plans sharing only `"/"` do
        // not short-circuit to a certain match.
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

// ─── Probabilistic components ────────────────────────────────────

/// Score the name component for `a` vs `b` in `[0.0, 1.0]`.
///
/// Best Jaro-Winkler (case-insensitive after folding) across each
/// record's primary name and all of its alternate names, plus a Soundex
/// +[`PHONETIC_BONUS`] (clamped to [`PHONETIC_CEILING`]) on the primary
/// names when they agree phonetically but the literal score is below the
/// ceiling. Name is required, so this component is always present.
fn name_score(a: &Plan, b: &Plan) -> f64 {
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

/// The folded set input for the goals component: each goal's title.
/// Empty titles are dropped by the downstream `fold_set`.
fn goal_titles(w: &Plan) -> Vec<String> {
    w.goals.iter().map(|g| g.title.clone()).collect()
}

/// Score the owner-scoped code component for `a` vs `b`.
///
/// A code only identifies a plan *within* its owning organisation,
/// so this is a binary, owner-gated comparison. Both records must carry a
/// non-empty code AND share an `owner_org_id`; otherwise the component is
/// skipped (`None`) so a local code cannot match across owners. Returns
/// `Some(1.0)` for equal normalised codes under the same owner,
/// `Some(0.0)` for differing codes under the same owner, else `None`.
fn code_score(a: &Plan, b: &Plan) -> Option<f64> {
    let (ac, bc) = match (a.code.as_deref(), b.code.as_deref()) {
        (Some(ac), Some(bc)) if !ac.is_empty() && !bc.is_empty() => (ac, bc),
        _ => return None,
    };
    match (owner_key(a), owner_key(b)) {
        (Some(ak), Some(bk)) if ak == bk => {
            if normalize::code(ac) == normalize::code(bc) {
                Some(1.0)
            } else {
                Some(0.0)
            }
        }
        _ => None,
    }
}

/// Score the owner-org component for `a` vs `b`.
///
/// Exact match on the opaque `owner_org_id` (case-folded): `Some(1.0)`
/// when both are present and equal, `Some(0.0)` when both present but
/// differ, `None` when either is unset. Keys solely on the id, never on
/// `owner_org_name`.
fn owner_org_score(a: &Plan, b: &Plan) -> Option<f64> {
    match (owner_key(a), owner_key(b)) {
        (Some(x), Some(y)) => Some(if normalize::fold(x) == normalize::fold(y) {
            1.0
        } else {
            0.0
        }),
        _ => None,
    }
}

/// Score the parent plan component for `a` vs `b`.
///
/// Applies to any plan (there is no kind restriction). Both records
/// must carry a non-empty `parent_ref`: `Some(1.0)` when they name the
/// same parent, `Some(0.0)` when they differ, `None` otherwise. Two
/// children of the same parent are mildly more likely to be the same
/// initiative — a supporting signal, never decisive.
fn parent_score(a: &Plan, b: &Plan) -> Option<f64> {
    match (
        a.parent_ref.as_deref().filter(|s| !s.is_empty()),
        b.parent_ref.as_deref().filter(|s| !s.is_empty()),
    ) {
        (Some(x), Some(y)) => Some(if normalize::fold(x) == normalize::fold(y) {
            1.0
        } else {
            0.0
        }),
        _ => None,
    }
}

/// Score the timeframe component for `a` vs `b` by date proximity.
///
/// Compares the records' `start_date` ↔ `start_date` and `target_date` ↔
/// `target_date` where both sides carry a parseable date, applying a
/// Gaussian decay `exp(-(Δdays / σ)² / 2)` to each available pair and
/// averaging. A zero day gap scores 1.0; the score degrades smoothly with
/// distance (near-misses are not snapped to 0). Returns `None` when no
/// date pair is comparable on both sides. `sigma` is the decay width in
/// days (guarded to a positive value).
fn timeframe_score(a: &Plan, b: &Plan, sigma: f64) -> Option<f64> {
    // Guard against a non-positive σ: fall back to a 1-day width.
    let sigma_eff = if sigma > 0.0 { sigma } else { 1.0 };
    let pairs = [
        (a.start_date.as_deref(), b.start_date.as_deref()),
        (a.target_date.as_deref(), b.target_date.as_deref()),
    ];
    let mut sum = 0.0_f64;
    let mut count = 0_u32;
    for (left, right) in pairs {
        if let (Some(left), Some(right)) = (left, right)
            && let (Some(left_day), Some(right_day)) = (
                normalize::iso_date_to_days(left),
                normalize::iso_date_to_days(right),
            )
        {
            // Day gap fits losslessly in `i32` for any realistic date pair;
            // saturate the absurd case, and `f64::from(i32)` is exact (no
            // precision-loss cast).
            let delta = i32::try_from((left_day - right_day).abs()).unwrap_or(i32::MAX);
            let ratio = f64::from(delta) / sigma_eff;
            sum += (-(ratio * ratio) / 2.0).exp();
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(sum / f64::from(count))
    }
}

/// Render a record's relationships to a folded token set for the
/// typed-set Jaccard: each link becomes `"<relation>\u{1f}<plan_id>"`
/// so a `DependsOn` link only agrees with a `DependsOn` link to the
/// **same** id. The relation kind is rendered verbatim (no inversion or
/// transitive closure). Empty ids are dropped by the downstream folding.
fn relationship_tokens(rels: &[PlanRelationship]) -> Vec<String> {
    rels.iter()
        .filter(|r| !r.plan_id.trim().is_empty())
        .map(|r| format!("{}\u{1f}{}", relation_label(&r.relation), r.plan_id))
        .collect()
}

/// A stable label for a [`RelationKind`], used as the typed key in the
/// relationships token set.
fn relation_label(r: &RelationKind) -> String {
    match r {
        RelationKind::ParentOf => "ParentOf".to_string(),
        RelationKind::ChildOf => "ChildOf".to_string(),
        RelationKind::DependsOn => "DependsOn".to_string(),
        RelationKind::BlockedBy => "BlockedBy".to_string(),
        RelationKind::Supersedes => "Supersedes".to_string(),
        RelationKind::SupersededBy => "SupersededBy".to_string(),
        RelationKind::SimilarTo => "SimilarTo".to_string(),
        RelationKind::RelatedTo => "RelatedTo".to_string(),
        RelationKind::Custom(s) => format!("Custom:{s}"),
    }
}

/// Jaccard similarity over two string sets, **lenient** on emptiness:
/// `None` only when *both* sides are empty, `Some(0.0)` when exactly one
/// side is populated. Used for `goals` and `keywords`.
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
    Some(jaccard(&a_set, &b_set))
}

/// Jaccard similarity over two string sets, **strict** on emptiness:
/// `None` when *either* side is empty (the component does not
/// participate). Used for `relationships` and `tags` — a supporting
/// signal that only speaks when both records carry the data.
fn set_jaccard_strict(a: &[String], b: &[String]) -> Option<f64> {
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    if a_set.is_empty() || b_set.is_empty() {
        return None;
    }
    Some(jaccard(&a_set, &b_set))
}

/// `|A ∩ B| / |A ∪ B|` over two non-empty, deduplicated sets.
fn jaccard(a_set: &[String], b_set: &[String]) -> f64 {
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        0.0
    } else {
        f64::from(u32::try_from(inter).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(union).unwrap_or(u32::MAX))
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{
        Goal, IdentifierScheme, PlanIdentifier, PlanKind, PlanPhase, PlanRelationship, PlanStatus,
    };

    fn ident(scheme: IdentifierScheme, value: &str) -> PlanIdentifier {
        PlanIdentifier {
            scheme,
            value: value.into(),
        }
    }

    fn project(name: &str) -> Plan {
        Plan::new(name)
    }

    // Pins the unified model: there is no kind gate. Two plans with
    // different (optional) kinds but an identical name score as a normal
    // probabilistic match; `kind_gate_blocked` is always false.
    #[test]
    fn different_kinds_still_match() {
        let engine = MatchingEngine::default_config();
        let mut a = Plan::new("Apollo");
        a.kind = Some(PlanKind::Portfolio);
        let mut b = Plan::new("Apollo");
        b.kind = Some(PlanKind::Project);
        let r = engine.match_plans(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
        assert!(!r.breakdown.kind_gate_blocked);
        assert!(r.breakdown.name_score.is_some());
    }

    // Pins that a shared deterministic id pins to 1.0 regardless of kind:
    // the gate no longer suppresses a cross-kind deterministic match.
    #[test]
    fn shared_id_matches_across_kinds() {
        let engine = MatchingEngine::default_config();
        let mut a = Plan::new("A");
        a.kind = Some(PlanKind::Product);
        let mut b = Plan::new("B");
        b.kind = Some(PlanKind::Program);
        a.identifiers.push(ident(IdentifierScheme::Uuid, "shared"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "shared"));
        let r = engine.match_plans(&a, &b);
        assert!(!r.breakdown.kind_gate_blocked);
        assert!(r.breakdown.deterministic_match);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    // Pins: identical same-kind names drive the probabilistic score ~1.0.
    #[test]
    fn identical_plans_score_high() {
        let engine = MatchingEngine::default_config();
        let a = project("Apollo platform migration");
        let b = project("Apollo platform migration");
        let r = engine.match_plans(&a, &b);
        assert!(r.score >= 0.99, "got {}", r.score);
        assert!(r.is_match);
    }

    // Pins R-0: a shared deterministic id pins to 1.0 / deterministic.
    #[test]
    fn deterministic_id_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = project("A");
        let mut b = project("Totally Different");
        a.identifiers
            .push(ident(IdentifierScheme::JiraProjectKey, "APOLLO"));
        b.identifiers
            .push(ident(IdentifierScheme::JiraProjectKey, "apollo"));
        let r = engine.match_plans(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    // Pins the owner gate on code: no owner → no short-circuit; different
    // owners → component skipped; same owner → conclusive.
    #[test]
    fn owner_scoped_code_does_not_cross_owners() {
        let mut a = project("A");
        let mut b = project("B");
        a.code = Some("PROJ-2026".into());
        b.code = Some("PROJ-2026".into());
        assert!(!deterministic_match(&a, &b));
        a.owner_org_id = Some("organization:1".into());
        b.owner_org_id = Some("organization:2".into());
        assert_eq!(code_score(&a, &b), None);
        b.owner_org_id = Some("organization:1".into());
        assert!(deterministic_match(&a, &b));
    }

    // Pins R-2: an overlapping same_as URL short-circuits (whitespace ok).
    #[test]
    fn same_as_overlap_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = project("Alpha");
        let mut b = project("Omega");
        a.same_as = vec!["https://pm.example.com/p/APOLLO".into()];
        b.same_as = vec!["  https://pm.example.com/p/APOLLO/  ".into()];
        let r = engine.match_plans(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    // SEC-M2: a bare root `same_as` URL (`"/"`, which `normalize::url`
    // deliberately keeps non-empty) is not identity evidence — two
    // DIFFERENT plans sharing only `"/"` must NOT short-circuit,
    // while a real shared URL still does.
    #[test]
    fn trivial_root_same_as_does_not_short_circuit() {
        let engine = MatchingEngine::default_config();
        let mut root_a = project("Alpha");
        let mut root_b = project("Omega");
        root_a.same_as = vec!["/".into()];
        root_b.same_as = vec!["/".into()];
        assert!(!deterministic_match(&root_a, &root_b));
        let r = engine.match_plans(&root_a, &root_b);
        assert!(!r.breakdown.deterministic_match);
        assert!(!r.is_match, "got {}", r.score);

        // Positive control: a real shared identity URL still short-circuits.
        let mut url_a = project("Alpha");
        let mut url_b = project("Omega");
        url_a.same_as = vec!["https://pm.example.com/p/APOLLO".into()];
        url_b.same_as = vec!["https://pm.example.com/p/APOLLO".into()];
        assert!(deterministic_match(&url_a, &url_b));
    }

    // Pins the goals component: shared goal titles (folded) score 1.0.
    #[test]
    fn goals_corroborate() {
        let engine = MatchingEngine::default_config();
        let mut a = project("Apollo platform migration");
        let mut b = project("Apollo platform migrate");
        a.goals = vec![Goal {
            title: "Cut latency".into(),
            ..Default::default()
        }];
        b.goals = vec![Goal {
            title: "CUT LATENCY".into(),
            ..Default::default()
        }];
        let r = engine.match_plans(&a, &b);
        assert_eq!(r.breakdown.goals_score, Some(1.0));
    }

    // Pins the owner-org exact component (1.0 / 0.0 / None).
    #[test]
    fn owner_org_exact_and_mismatch() {
        let mut a = project("A");
        let mut b = project("B");
        a.owner_org_id = Some("organization:9a2f".into());
        b.owner_org_id = Some("organization:9a2f".into());
        assert_eq!(owner_org_score(&a, &b), Some(1.0));
        b.owner_org_id = Some("organization:bbbb".into());
        assert_eq!(owner_org_score(&a, &b), Some(0.0));
        b.owner_org_id = None;
        assert_eq!(owner_org_score(&a, &b), None);
    }

    // Pins the parent component: any two plans with a parent_ref
    // compare it exactly (matching parents → 1.0, differing → 0.0);
    // absent on either side → None. There is no kind-based skip.
    #[test]
    fn parent_component_compares_refs() {
        let mut a = project("A");
        let mut b = project("B");
        a.parent_ref = Some("parent-1".into());
        b.parent_ref = Some("parent-1".into());
        assert_eq!(parent_score(&a, &b), Some(1.0));
        b.parent_ref = Some("parent-2".into());
        assert_eq!(parent_score(&a, &b), Some(0.0));
        // Absent on one side → not comparable.
        b.parent_ref = None;
        assert_eq!(parent_score(&a, &b), None);
    }

    // Pins the timeframe Gaussian: equal dates → 1.0, a σ-sized gap →
    // ≈0.607, no comparable pair → None.
    #[test]
    fn timeframe_gaussian_decay() {
        let mut a = project("A");
        let mut b = project("B");
        a.start_date = Some("2026-01-01".into());
        b.start_date = Some("2026-01-01".into());
        assert!((timeframe_score(&a, &b, 90.0).unwrap() - 1.0).abs() < 1e-9);
        // A 90-day gap at σ=90 → exp(-0.5) ≈ 0.6065.
        b.start_date = Some("2026-04-01".into()); // +90 days
        let s = timeframe_score(&a, &b, 90.0).unwrap();
        assert!((s - (-0.5_f64).exp()).abs() < 1e-6, "got {s}");
        // No comparable date pair → None.
        let plain_a = project("A");
        let plain_b = project("B");
        assert_eq!(timeframe_score(&plain_a, &plain_b, 90.0), None);
    }

    // Pins relationships typed-set Jaccard: same (relation, id) agrees;
    // a different relation kind to the same id does not; strict-empty.
    #[test]
    fn relationships_typed_set_jaccard() {
        let a = relationship_tokens(&[PlanRelationship {
            relation: RelationKind::DependsOn,
            plan_id: "p2".into(),
        }]);
        let b = relationship_tokens(&[PlanRelationship {
            relation: RelationKind::DependsOn,
            plan_id: "p2".into(),
        }]);
        assert_eq!(set_jaccard_strict(&a, &b), Some(1.0));
        // Different relation to the same id → no overlap.
        let c = relationship_tokens(&[PlanRelationship {
            relation: RelationKind::BlockedBy,
            plan_id: "p2".into(),
        }]);
        assert_eq!(set_jaccard_strict(&a, &c), Some(0.0));
        // Strict emptiness: either side empty → None.
        assert_eq!(set_jaccard_strict(&a, &[]), None);
        assert_eq!(set_jaccard_strict(&[], &[]), None);
    }

    // Pins tags strict Jaccard: partial overlap is the ratio; either side
    // empty is None (NOT 0.0 — strict semantics).
    #[test]
    fn tags_strict_jaccard() {
        let a = vec!["q3".to_string(), "fast-track".to_string()];
        let b = vec!["Q3".to_string()];
        assert!((set_jaccard_strict(&a, &b).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(set_jaccard_strict(&a, &[]), None);
    }

    // Pins keywords lenient Jaccard: one side populated → 0.0; both empty
    // → None.
    #[test]
    fn keywords_lenient_jaccard() {
        let a = vec!["migration".to_string()];
        assert_eq!(set_jaccard(&a, &[]), Some(0.0));
        assert_eq!(set_jaccard(&[], &[]), None);
    }

    // Pins that status is never scored (informational-only): flipping the
    // status leaves the score unchanged.
    #[test]
    fn status_is_not_scored() {
        let engine = MatchingEngine::default_config();
        let mut a = project("Apollo platform migration");
        let mut b = project("Apollo platform migration");
        a.status = Some(PlanStatus::Active);
        b.status = Some(PlanStatus::Completed);
        let differing = engine.match_plans(&a, &b).score;
        b.status = Some(PlanStatus::Active);
        let same = engine.match_plans(&a, &b).score;
        assert!((differing - same).abs() < 1e-9);
    }

    // Pins that the project phase is never scored, for the same reason
    // status is not: two systems describing one initiative routinely
    // disagree about which phase it is in, and the phase is the field
    // most likely to differ. Scoring it would make the matcher worse
    // exactly where records are hardest to reconcile.
    #[test]
    fn phase_is_not_scored() {
        let engine = MatchingEngine::default_config();
        let mut a = project("Apollo platform migration");
        let mut b = project("Apollo platform migration");
        a.phase = Some(PlanPhase::Initiating);
        b.phase = Some(PlanPhase::Closing);
        let differing = engine.match_plans(&a, &b).score;
        b.phase = Some(PlanPhase::Initiating);
        let same = engine.match_plans(&a, &b).score;
        assert!((differing - same).abs() < 1e-9);
    }

    // Pins the negative case: dissimilar same-kind names with no
    // corroboration fall below threshold and into the Low band.
    #[test]
    fn unrelated_plans_score_low() {
        let engine = MatchingEngine::default_config();
        let a = project("Apollo platform migration");
        let b = project("Cafeteria refit programme");
        let r = engine.match_plans(&a, &b);
        assert!(!r.is_match, "got {}", r.score);
        assert_eq!(r.confidence, Confidence::Low);
    }

    // Pins the one-to-many surface: rank sorts best-first, find_matches
    // filters by threshold, empty candidates handled.
    #[test]
    fn rank_and_find_matches() {
        let engine = MatchingEngine::default_config();
        let query = project("Apollo platform migration");
        let cands = vec![
            project("Cafeteria refit programme"),
            project("Apollo platform migration"),
        ];
        let ranked = engine.rank(&query, &cands);
        assert_eq!(ranked[0].0, 1);
        let matches = engine.find_matches(&query, &cands);
        assert!(matches.iter().all(|(_, r)| r.is_match));
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
    }
}
