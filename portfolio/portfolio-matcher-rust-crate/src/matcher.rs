//! `MatchingEngine` — the public entry point.
//!
//! Three phases:
//!
//! 1. **Kind gate (R-GATE).** If the two records have different `kind`,
//!    they are distinct record types in distinct collections and never
//!    match: return score `0.0` with `breakdown.kind_gate_blocked = true`
//!    and every component left `None`. This runs before everything else.
//! 2. **Deterministic short-circuit.** If both (same-kind) records share
//!    a value on a deterministic identifier scheme (the tool/registry ids
//!    plus URI / UUID) OR share `owner_org_id` + normalised `code` OR
//!    overlap on a `same_as` URL, return score `1.0`.
//! 3. **Probabilistic scoring.** Per-component scores, then a weighted
//!    average over the *present* components.

use strsim::jaro_winkler;

use crate::config::MatchConfig;
use crate::normalize;
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};
use crate::work_item::{RelationKind, WorkItem, WorkItemRelationship};

/// Additive Soundex bonus applied to the name component when the two
/// names share a phonetic code but the raw Jaro-Winkler score is still
/// below [`PHONETIC_CEILING`]. Small (`0.05`) by design: a phonetic
/// agreement is a weak corroborating signal, not a decisive one.
const PHONETIC_BONUS: f64 = 0.05;

/// Upper bound the phonetic bonus may lift the name score to, and the
/// gate below which the bonus is considered at all. Capping at `0.95`
/// keeps the phonetic-only path strictly below a "certain" name match.
const PHONETIC_CEILING: f64 = 0.95;

/// The work-item matcher: holds a [`MatchConfig`] and scores pairs.
///
/// Construct once with [`MatchingEngine::new`] (or
/// [`MatchingEngine::default_config`]) and reuse across many
/// [`MatchingEngine::match_work_items`] calls — the engine is immutable
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

    /// Score two work items. Always returns a result (never errs).
    ///
    /// Runs the three-phase pipeline: the **kind gate** (different kind →
    /// `0.0`, `kind_gate_blocked`), then the deterministic short-circuit
    /// (`R-0`/`R-1`/`R-2` → pinned `1.0`/[`Confidence::High`],
    /// `deterministic_match`), then the probabilistic strategy —
    /// per-component scores fed through [`weighted_average`], compared
    /// against the configured threshold.
    ///
    /// `a` and `b` are the two records to compare; argument order does
    /// not affect the score. Returns the populated [`MatchResult`].
    /// Infallible by contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use portfolio_matcher::{WorkItem, WorkItemKind, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    /// let b = WorkItem::new(WorkItemKind::Project, "Apollo platform migration");
    /// let r = engine.match_work_items(&a, &b);
    /// assert!((0.0..=1.0).contains(&r.score));
    /// ```
    #[must_use]
    pub fn match_work_items(&self, a: &WorkItem, b: &WorkItem) -> MatchResult {
        // Phase 0: the kind gate. Different kinds are distinct record
        // types and can never be the same identity — pin to a 0.0 no-match
        // and record the reason, skipping every other rule (including the
        // deterministic short-circuit).
        if a.kind != b.kind {
            return MatchResult {
                breakdown: MatchBreakdown {
                    kind_gate_blocked: true,
                    ..Default::default()
                },
                ..Default::default()
            };
        }

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
        let portfolio_score = portfolio_score(a, b);
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
            (portfolio_score, self.config.portfolio_weight),
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
                portfolio_score,
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
    pub fn match_one_to_many(&self, query: &WorkItem, candidates: &[WorkItem]) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_work_items(query, c))
            .collect()
    }

    /// One-to-many: `(index, result)` sorted by descending score. Each
    /// pair's `usize` is the candidate's original index.
    #[must_use]
    pub fn rank(&self, query: &WorkItem, candidates: &[WorkItem]) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_work_items(query, c)))
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
    pub fn find_matches(
        &self,
        query: &WorkItem,
        candidates: &[WorkItem],
    ) -> Vec<(usize, MatchResult)> {
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
fn owner_key(w: &WorkItem) -> Option<&str> {
    w.owner_org_id.as_deref().filter(|s| !s.is_empty())
}

/// Decide whether the deterministic short-circuit fires for `a` vs `b`
/// (already known to be the same kind). Any single rule is conclusive:
///
/// - `R-0` — a shared value on the same globally-unique identifier scheme.
/// - `R-1` — the same `owner_org_id` and the same normalised `code`.
/// - `R-2` — an overlapping `same_as` identity URL.
fn deterministic_match(a: &WorkItem, b: &WorkItem) -> bool {
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

/// Score the name component for `a` vs `b` in `[0.0, 1.0]`.
///
/// Best Jaro-Winkler (case-insensitive after folding) across each
/// record's primary name and all of its alternate names, plus a Soundex
/// +[`PHONETIC_BONUS`] (clamped to [`PHONETIC_CEILING`]) on the primary
/// names when they agree phonetically but the literal score is below the
/// ceiling. Name is required, so this component is always present.
fn name_score(a: &WorkItem, b: &WorkItem) -> f64 {
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
fn goal_titles(w: &WorkItem) -> Vec<String> {
    w.goals.iter().map(|g| g.title.clone()).collect()
}

/// Score the owner-scoped code component for `a` vs `b`.
///
/// A code only identifies a work item *within* its owning organisation,
/// so this is a binary, owner-gated comparison. Both records must carry a
/// non-empty code AND share an `owner_org_id`; otherwise the component is
/// skipped (`None`) so a local code cannot match across owners. Returns
/// `Some(1.0)` for equal normalised codes under the same owner,
/// `Some(0.0)` for differing codes under the same owner, else `None`.
fn code_score(a: &WorkItem, b: &WorkItem) -> Option<f64> {
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
fn owner_org_score(a: &WorkItem, b: &WorkItem) -> Option<f64> {
    match (owner_key(a), owner_key(b)) {
        (Some(x), Some(y)) => Some(if normalize::fold(x) == normalize::fold(y) {
            1.0
        } else {
            0.0
        }),
        _ => None,
    }
}

/// Score the parent-portfolio component for `a` vs `b`.
///
/// Applies only to **child kinds** (`Project` / `Product` / `Program`);
/// for the `Portfolio` kind the component is always skipped. Both records
/// must carry a non-empty `portfolio_ref`: `Some(1.0)` when they name the
/// same parent, `Some(0.0)` when they differ, `None` otherwise. Two
/// children of the same portfolio are mildly more likely to be the same
/// initiative — a supporting signal, never decisive.
fn portfolio_score(a: &WorkItem, b: &WorkItem) -> Option<f64> {
    // Kind is gated equal before this runs, so checking `a` suffices.
    if !a.kind.is_child() {
        return None;
    }
    match (
        a.portfolio_ref.as_deref().filter(|s| !s.is_empty()),
        b.portfolio_ref.as_deref().filter(|s| !s.is_empty()),
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
fn timeframe_score(a: &WorkItem, b: &WorkItem, sigma: f64) -> Option<f64> {
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
/// typed-set Jaccard: each link becomes `"<relation>\u{1f}<work_item_id>"`
/// so a `DependsOn` link only agrees with a `DependsOn` link to the
/// **same** id. The relation kind is rendered verbatim (no inversion or
/// transitive closure). Empty ids are dropped by the downstream folding.
fn relationship_tokens(rels: &[WorkItemRelationship]) -> Vec<String> {
    rels.iter()
        .filter(|r| !r.work_item_id.trim().is_empty())
        .map(|r| format!("{}\u{1f}{}", relation_label(&r.relation), r.work_item_id))
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
    use crate::work_item::{
        Goal, IdentifierScheme, WorkItemIdentifier, WorkItemKind, WorkItemRelationship,
        WorkItemStatus,
    };

    fn ident(scheme: IdentifierScheme, value: &str) -> WorkItemIdentifier {
        WorkItemIdentifier {
            scheme,
            value: value.into(),
        }
    }

    fn project(name: &str) -> WorkItem {
        WorkItem::new(WorkItemKind::Project, name)
    }

    // Pins the kind gate: two records of different kind never match, even
    // with an otherwise-identical name, and the reason is recorded.
    #[test]
    fn kind_gate_blocks_cross_kind() {
        let engine = MatchingEngine::default_config();
        let a = WorkItem::new(WorkItemKind::Project, "Apollo");
        let b = WorkItem::new(WorkItemKind::Product, "Apollo");
        let r = engine.match_work_items(&a, &b);
        assert!(r.score.abs() < 1e-9);
        assert!(!r.is_match);
        assert!(r.breakdown.kind_gate_blocked);
        assert!(r.breakdown.name_score.is_none());
    }

    // Pins that the kind gate beats even a shared deterministic id: a
    // shared UUID across kinds is NOT a match (distinct record types).
    #[test]
    fn kind_gate_beats_deterministic_id() {
        let engine = MatchingEngine::default_config();
        let mut a = WorkItem::new(WorkItemKind::Project, "A");
        let mut b = WorkItem::new(WorkItemKind::Program, "B");
        a.identifiers.push(ident(IdentifierScheme::Uuid, "shared"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "shared"));
        let r = engine.match_work_items(&a, &b);
        assert!(r.breakdown.kind_gate_blocked);
        assert!(!r.breakdown.deterministic_match);
        assert!(r.score.abs() < 1e-9);
    }

    // Pins: identical same-kind names drive the probabilistic score ~1.0.
    #[test]
    fn identical_work_items_score_high() {
        let engine = MatchingEngine::default_config();
        let a = project("Apollo platform migration");
        let b = project("Apollo platform migration");
        let r = engine.match_work_items(&a, &b);
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
        let r = engine.match_work_items(&a, &b);
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
        let r = engine.match_work_items(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
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
        let r = engine.match_work_items(&a, &b);
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

    // Pins the parent-portfolio component: child kinds compare the parent
    // ref; the Portfolio kind always skips it.
    #[test]
    fn portfolio_component_child_only() {
        let mut a = project("A");
        let mut b = project("B");
        a.portfolio_ref = Some("portfolio-1".into());
        b.portfolio_ref = Some("portfolio-1".into());
        assert_eq!(portfolio_score(&a, &b), Some(1.0));
        b.portfolio_ref = Some("portfolio-2".into());
        assert_eq!(portfolio_score(&a, &b), Some(0.0));
        // Portfolio kind never participates.
        let mut pa = WorkItem::new(WorkItemKind::Portfolio, "A");
        let mut pb = WorkItem::new(WorkItemKind::Portfolio, "B");
        pa.portfolio_ref = Some("x".into());
        pb.portfolio_ref = Some("x".into());
        assert_eq!(portfolio_score(&pa, &pb), None);
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
        let a = relationship_tokens(&[WorkItemRelationship {
            relation: RelationKind::DependsOn,
            work_item_id: "p2".into(),
        }]);
        let b = relationship_tokens(&[WorkItemRelationship {
            relation: RelationKind::DependsOn,
            work_item_id: "p2".into(),
        }]);
        assert_eq!(set_jaccard_strict(&a, &b), Some(1.0));
        // Different relation to the same id → no overlap.
        let c = relationship_tokens(&[WorkItemRelationship {
            relation: RelationKind::BlockedBy,
            work_item_id: "p2".into(),
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
        a.status = Some(WorkItemStatus::Active);
        b.status = Some(WorkItemStatus::Completed);
        let differing = engine.match_work_items(&a, &b).score;
        b.status = Some(WorkItemStatus::Active);
        let same = engine.match_work_items(&a, &b).score;
        assert!((differing - same).abs() < 1e-9);
    }

    // Pins the negative case: dissimilar same-kind names with no
    // corroboration fall below threshold and into the Low band.
    #[test]
    fn unrelated_work_items_score_low() {
        let engine = MatchingEngine::default_config();
        let a = project("Apollo platform migration");
        let b = project("Cafeteria refit programme");
        let r = engine.match_work_items(&a, &b);
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
