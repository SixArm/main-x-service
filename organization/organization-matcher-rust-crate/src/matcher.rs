//! `MatchingEngine` — the public entry point.
//!
//! Two phases:
//!
//! 1. **Deterministic short-circuit.** If both records share a value on
//!    a deterministic identifier scheme (LEI, DUNS, ISO 6523, GLN,
//!    Wikidata, ROR, ISNI, VAT) OR share `jurisdiction` + tax id OR
//!    overlap on a `same_as` URL, return score `1.0`.
//! 2. **Probabilistic scoring.** Per-component scores, then a weighted
//!    average over the *present* components.

use std::collections::HashSet;

use strsim::jaro_winkler;

use crate::config::MatchConfig;
use crate::normalize;
use crate::organization::{IdentifierScheme, Organization, RelationKind, RelationshipRef};
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

/// Additive Soundex reward applied to the name component when the two
/// primary names share a phonetic code but their literal Jaro-Winkler
/// similarity is still below the High band. Deliberately small (+0.05):
/// a phonetic agreement is corroborating evidence, never a substitute
/// for an actual spelling match.
const PHONETIC_BONUS: f64 = 0.05;
/// Upper bound the Soundex bonus may lift the name score to. Kept below
/// a perfect `1.0` (0.95 = the High threshold) so a purely phonetic
/// agreement can never masquerade as an exact-name match.
const PHONETIC_CEILING: f64 = 0.95;

/// Additive reward applied to the address component when the two
/// records' postal codes are present and fold-equal, mirroring
/// [`PHONETIC_BONUS`]'s role for names. A postal code is a strong,
/// fine-grained locality signal (especially alphanumeric schemes like
/// UK/Canada), so an exact match anchors the address score up even when
/// `street_address`/`locality`/`region` disagree or are only fuzzily
/// similar — corroborating evidence, not a substitute for the other
/// fields actually agreeing.
const POSTAL_CODE_ANCHOR_BONUS: f64 = 0.10;
/// Upper bound the postal-code anchor may lift the address score to.
/// Kept below a perfect `1.0` (mirroring [`PHONETIC_CEILING`]'s role for
/// names) so a shared postal code alone can never masquerade as a full
/// address match — an organization's street address is not implied by
/// its postal code.
const POSTAL_CODE_ANCHOR_CEILING: f64 = 0.95;

/// The organization matcher: holds a [`MatchConfig`] and scores pairs.
///
/// Construct one with [`MatchingEngine::new`] (or
/// [`MatchingEngine::default_config`]) and reuse it across many pairs —
/// the engine is immutable and carries no per-call state, so it is
/// cheap to clone the borrow and safe to share.
pub struct MatchingEngine {
    /// Per-component weights and the probable-match threshold applied to
    /// every probabilistic comparison this engine performs.
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
    ///
    /// `config` supplies the per-component weights and the
    /// probable-match threshold; it is stored verbatim and used for
    /// every subsequent call.
    #[must_use]
    pub fn new(config: MatchConfig) -> Self {
        Self { config }
    }

    /// Build with [`MatchConfig::default`]. Convenience for the common
    /// path where the caller wants the canonical weights/threshold.
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(MatchConfig::default())
    }

    /// Borrow the engine's configuration, e.g. to read the active
    /// `threshold` back when interpreting `is_match`.
    #[must_use]
    pub fn config(&self) -> &MatchConfig {
        &self.config
    }

    /// Score two organizations. Always returns a result (never errs).
    ///
    /// # Examples
    ///
    /// ```
    /// use organization_matcher::{Organization, MatchingEngine};
    ///
    /// let engine = MatchingEngine::default_config();
    /// let a = Organization::new("Acme Corporation");
    /// let b = Organization::new("Acme Corp");
    /// let result = engine.match_organizations(&a, &b);
    /// assert!((0.0..=1.0).contains(&result.score));
    /// ```
    #[must_use]
    pub fn match_organizations(&self, a: &Organization, b: &Organization) -> MatchResult {
        // Phase 1 — deterministic short-circuit. A shared globally-unique
        // identifier (or same-jurisdiction tax id, or `same_as` overlap)
        // is conclusive on its own, so we return a perfect score without
        // computing — or being diluted by — any fuzzy components.
        if deterministic_match(a, b) {
            return MatchResult {
                score: 1.0,
                is_match: true,
                confidence: Confidence::High,
                breakdown: MatchBreakdown {
                    // Flag the short-circuit so callers can tell a rule
                    // hit apart from a fuzzy 1.0; the per-component
                    // scores stay `None` because none were evaluated.
                    deterministic_match: true,
                    ..Default::default()
                },
            };
        }

        // Phase 2 — probabilistic scoring. Name is always present (it is
        // a required field), hence the unconditional `Some`; every other
        // component returns `None` when the data is absent on either side
        // so it can be skipped during renormalisation.
        let name_score = Some(name_score(a, b));
        let address_score = address_score(a, b);
        let url_score = url_score(a, b);
        let jurisdiction_score = jurisdiction_score(a, b);
        let founding_date_score = founding_date_score(a, b);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);
        let relationships_score = relationships_score(&a.relationships, &b.relationships);
        let tags_score = tags_score(&a.tags, &b.tags);

        // Renormalised weighted average: absent components drop out of
        // both numerator and denominator, so missing data neither helps
        // nor hurts the overall score.
        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (address_score, self.config.address_weight),
            (url_score, self.config.url_weight),
            (jurisdiction_score, self.config.jurisdiction_weight),
            (founding_date_score, self.config.founding_date_weight),
            (keywords_score, self.config.keywords_weight),
            (relationships_score, self.config.relationships_weight),
            (tags_score, self.config.tags_weight),
        ]);

        // A match is declared when the renormalised score clears the
        // configured probable-match threshold (default 0.85).
        let is_match = score >= self.config.threshold;
        MatchResult {
            score,
            is_match,
            confidence: Confidence::classify(score),
            breakdown: MatchBreakdown {
                name_score,
                address_score,
                url_score,
                jurisdiction_score,
                founding_date_score,
                keywords_score,
                relationships_score,
                tags_score,
                deterministic_match: false,
            },
        }
    }

    /// One-to-many: score `query` against each candidate, returning the
    /// results in candidate input order (the result at position `i`
    /// corresponds to `candidates[i]`). An empty `candidates` slice
    /// yields an empty vector.
    #[must_use]
    pub fn match_one_to_many(
        &self,
        query: &Organization,
        candidates: &[Organization],
    ) -> Vec<MatchResult> {
        candidates
            .iter()
            .map(|c| self.match_organizations(query, c))
            .collect()
    }

    /// One-to-many ranked: `(original_index, result)` pairs sorted by
    /// descending score, so the best candidate is first. The index is
    /// the candidate's position in the input slice, preserved through
    /// the sort so callers can recover which candidate each result came
    /// from.
    #[must_use]
    pub fn rank(
        &self,
        query: &Organization,
        candidates: &[Organization],
    ) -> Vec<(usize, MatchResult)> {
        let mut ranked: Vec<(usize, MatchResult)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, self.match_organizations(query, c)))
            .collect();
        // Descending by score. `partial_cmp` can only be `None` for NaN,
        // which scores never are; treating that as `Equal` keeps the
        // sort total without a panic, satisfying the no-panic rule.
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    /// Rank, then drop every candidate whose score falls below the
    /// configured [`MatchConfig::threshold`] (i.e. keep only the entries
    /// flagged `is_match`). Returns the surviving `(index, result)`
    /// pairs in descending-score order.
    #[must_use]
    pub fn find_matches(
        &self,
        query: &Organization,
        candidates: &[Organization],
    ) -> Vec<(usize, MatchResult)> {
        self.rank(query, candidates)
            .into_iter()
            .filter(|(_, r)| r.is_match)
            .collect()
    }
}

// ─── Deterministic rules ─────────────────────────────────────────

/// True when any deterministic rule fires, making the pair a certain
/// match regardless of fuzzy similarity. Three rules, checked in order;
/// the first hit short-circuits:
///
/// - **R-0** — both records publish the *same value* under the *same*
///   globally-unique scheme (LEI/DUNS/ISO 6523/GLN/Wikidata/ROR/ISNI/VAT).
/// - **R-1** — both records sit in the *same jurisdiction* and share a
///   `TaxId` value; a tax id is only unique within its issuing country,
///   hence the jurisdiction guard.
/// - **R-2** — their `same_as` URL sets overlap (a shared canonical
///   identity URL such as a Wikidata or ROR page).
///
/// All comparisons are case-folded so trivial casing/whitespace
/// differences do not defeat the rule. Empty values are ignored so two
/// records that both merely *lack* an identifier never match on it.
fn deterministic_match(a: &Organization, b: &Organization) -> bool {
    // R-0 — any pair of deterministic identifiers shares a value.
    for ai in &a.identifiers {
        // Only globally-unique schemes qualify; classification codes
        // (NAICS/SIC/…) and `Custom` are skipped here.
        if !ai.scheme.is_deterministic() {
            continue;
        }
        let av = normalize::fold(&ai.value);
        // An empty (or whitespace-only) value carries no identity.
        if av.is_empty() {
            continue;
        }
        for bi in &b.identifiers {
            // Same scheme AND same folded value → conclusive.
            if ai.scheme == bi.scheme && av == normalize::fold(&bi.value) {
                return true;
            }
        }
    }

    // R-1 — same jurisdiction + same tax id. The outer guard requires a
    // non-empty, matching jurisdiction on both sides before any tax id
    // is even considered, because the same number can be reused across
    // countries.
    if let (Some(aj), Some(bj)) = (a.jurisdiction.as_deref(), b.jurisdiction.as_deref())
        && !aj.is_empty()
        && normalize::fold(aj) == normalize::fold(bj)
    {
        for ai in &a.identifiers {
            if ai.scheme != IdentifierScheme::TaxId {
                continue;
            }
            let av = normalize::fold(&ai.value);
            if av.is_empty() {
                continue;
            }
            for bi in &b.identifiers {
                if bi.scheme == IdentifierScheme::TaxId && av == normalize::fold(&bi.value) {
                    return true;
                }
            }
        }
    }

    // R-2 — any same_as URL overlaps (case-folded). A shared canonical
    // identity URL is treated as conclusive on its own.
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

/// Collect every name that should contribute to the name score —
/// `name`, then `legal_name`, then each `alternate_names` entry — each
/// reduced to its legal-suffix-stripped comparison form. Empty keys are
/// dropped. The primary `name` is kept first so callers (e.g. the
/// Soundex bonus) can address the canonical name via `first()`.
fn name_keys(o: &Organization) -> Vec<String> {
    let mut keys = vec![normalize::legal_name(&o.name)];
    if let Some(ln) = &o.legal_name {
        keys.push(normalize::legal_name(ln));
    }
    for alt in &o.alternate_names {
        keys.push(normalize::legal_name(alt));
    }
    // Drop any key that normalised to empty so they cannot spuriously
    // match each other (two empties score 1.0 under Jaro-Winkler).
    keys.retain(|k| !k.is_empty());
    keys
}

/// Name component score in `[0.0, 1.0]`: the best Jaro-Winkler
/// similarity across the full cross-product of both records' name keys
/// (so an alias on one side can match the primary name on the other),
/// with a capped Soundex bonus on the primary names. Always returns a
/// value because `name` is required.
fn name_score(a: &Organization, b: &Organization) -> f64 {
    let a_keys = name_keys(a);
    let b_keys = name_keys(b);
    // Take the strongest pairing: any name on `a` against any on `b`.
    let mut best = 0.0_f64;
    for ak in &a_keys {
        for bk in &b_keys {
            best = best.max(jaro_winkler(ak, bk));
        }
    }
    // Soundex bonus on the primary normalised names, capped. Applied
    // only when the literal similarity has not already reached the
    // ceiling, so it lifts near-misses (typos, transliteration) without
    // ever forging an exact-name match.
    let ap = a_keys.first().map_or("", String::as_str);
    let bp = b_keys.first().map_or("", String::as_str);
    if best < PHONETIC_CEILING && phonetic::same(ap, bp) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

/// Postal-address component score, or `None` when either record has no
/// address (or no field is populated on both sides). Computes a
/// weighted, field-by-field Jaro-Winkler average, renormalised over the
/// fields actually shared so a partially-filled address is not
/// penalised for its blanks. A postal code that fold-matches exactly on
/// both sides then anchors the result up via [`POSTAL_CODE_ANCHOR_BONUS`]
/// (capped at [`POSTAL_CODE_ANCHOR_CEILING`]) — see that constant's docs.
fn address_score(a: &Organization, b: &Organization) -> Option<f64> {
    // Both records must carry an address at all.
    let (Some(aa), Some(ba)) = (&a.address, &b.address) else {
        return None;
    };
    // Per-field weights: street and postal code carry the most signal
    // (most discriminating), country the least (coarse / often shared).
    // Sum = 1.00 but the divisor below is the present-field subset.
    let pairs: [(Option<&String>, Option<&String>, f64); 5] = [
        (aa.street_address.as_ref(), ba.street_address.as_ref(), 0.30),
        (aa.locality.as_ref(), ba.locality.as_ref(), 0.25),
        (aa.region.as_ref(), ba.region.as_ref(), 0.15),
        (aa.postal_code.as_ref(), ba.postal_code.as_ref(), 0.20),
        (aa.country.as_ref(), ba.country.as_ref(), 0.10),
    ];
    let mut sum = 0.0_f64;
    let mut wsum = 0.0_f64;
    for (x, y, w) in pairs {
        // Only fields present on both sides contribute.
        if let (Some(x), Some(y)) = (x, y) {
            let (xf, yf) = (normalize::fold(x), normalize::fold(y));
            // Skip a field that is blank on both sides after folding.
            if xf.is_empty() && yf.is_empty() {
                continue;
            }
            sum += jaro_winkler(&xf, &yf) * w;
            wsum += w;
        }
    }
    // Renormalise over the contributing weights; `None` if none shared.
    let mut score = if wsum > 0.0 { sum / wsum } else { return None };

    // Postal-code exact-anchor boost: an exact fold-match on postal code
    // alone is strong evidence of the same location, so it nudges a
    // borderline (or partially-disagreeing) address score up — but never
    // past the ceiling, and never when either side's postal code is
    // absent or blank.
    if let (Some(ap), Some(bp)) = (aa.postal_code.as_deref(), ba.postal_code.as_deref()) {
        let (apf, bpf) = (normalize::fold(ap), normalize::fold(bp));
        if !apf.is_empty() && apf == bpf && score < POSTAL_CODE_ANCHOR_CEILING {
            score = (score + POSTAL_CODE_ANCHOR_BONUS).min(POSTAL_CODE_ANCHOR_CEILING);
        }
    }
    Some(score)
}

/// URL component score, comparing the two registered domains (scheme /
/// `www.` / path stripped). Equal domains score a perfect `1.0`;
/// otherwise the domains' Jaro-Winkler similarity (catches typos and
/// suffix variants). `None` when either side has no usable URL/domain.
fn url_score(a: &Organization, b: &Organization) -> Option<f64> {
    let (Some(au), Some(bu)) = (a.url.as_deref(), b.url.as_deref()) else {
        return None;
    };
    let (ad, bd) = (normalize::domain(au), normalize::domain(bu));
    // A URL that normalises to an empty host carries no signal.
    if ad.is_empty() || bd.is_empty() {
        return None;
    }
    Some(if ad == bd {
        1.0
    } else {
        jaro_winkler(&ad, &bd)
    })
}

/// Jurisdiction component score: a binary `1.0` (case-folded equal) or
/// `0.0` (differing). `None` when either side omits a jurisdiction —
/// jurisdiction is a coarse, categorical signal with no useful
/// in-between, so no fuzzy similarity is computed.
fn jurisdiction_score(a: &Organization, b: &Organization) -> Option<f64> {
    match (a.jurisdiction.as_deref(), b.jurisdiction.as_deref()) {
        (Some(aj), Some(bj)) if !aj.is_empty() && !bj.is_empty() => {
            Some(if normalize::fold(aj) == normalize::fold(bj) {
                1.0
            } else {
                0.0
            })
        }
        _ => None,
    }
}

/// Parse the four-character year prefix of an ISO-8601 date string into
/// an `i32`. Only the leading `YYYY` is read, so `"1998-09-04"` and
/// `"1998"` both yield `1998`. `None` if those characters are not a
/// valid integer.
fn founding_year(s: &str) -> Option<i32> {
    // Compare on year only; founding dates are frequently recorded to
    // year precision, so day/month would add noise, not signal.
    let head: String = s.trim().chars().take(4).collect();
    head.parse::<i32>().ok()
}

/// Founding-date component score by year proximity: exact year `1.0`,
/// one-year-off `0.5` (tolerates fiscal-vs-calendar / off-by-one
/// recording error), otherwise `0.0`. `None` when either date is
/// missing or unparseable.
fn founding_date_score(a: &Organization, b: &Organization) -> Option<f64> {
    let (Some(af), Some(bf)) = (a.founding_date.as_deref(), b.founding_date.as_deref()) else {
        return None;
    };
    let (Some(ay), Some(by)) = (founding_year(af), founding_year(bf)) else {
        return None;
    };
    Some(match (ay - by).abs() {
        0 => 1.0,
        1 => 0.5,
        _ => 0.0,
    })
}

/// Jaccard similarity (`|intersection| / |union|`) of two case-folded,
/// de-duplicated string sets — used for the keywords component.
///
/// Returns `None` only when *neither* side has any keyword (so the
/// component is skipped entirely). When one side is empty but the other
/// is not, returns `Some(0.0)` — a present-vs-absent disagreement,
/// which is signal worth counting against the match.
fn set_jaccard(a: &[String], b: &[String]) -> Option<f64> {
    // Fast path: both raw inputs empty → nothing to compare.
    if a.is_empty() && b.is_empty() {
        return None;
    }
    let a_set = normalize::fold_set(a);
    let b_set = normalize::fold_set(b);
    // Both empty after folding (e.g. only blank strings) → skip.
    if a_set.is_empty() && b_set.is_empty() {
        return None;
    }
    // Exactly one side empty → maximal disagreement.
    if a_set.is_empty() || b_set.is_empty() {
        return Some(0.0);
    }
    let inter: usize = a_set.iter().filter(|x| b_set.contains(x)).count();
    // Inclusion-exclusion: |A ∪ B| = |A| + |B| − |A ∩ B|.
    let union: usize = a_set.len() + b_set.len() - inter;
    if union == 0 {
        Some(0.0)
    } else {
        // Keyword counts are tiny; convert losslessly via `u32` so the
        // ratio is exact (only absurd counts would saturate).
        Some(
            f64::from(u32::try_from(inter).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(union).unwrap_or(u32::MAX)),
        )
    }
}

/// Relationships component score: typed-set **Jaccard** over
/// `(relation, organization_id)` pairs — `|A ∩ B| / |A ∪ B|` — so a
/// `SubOrganizationOf` reference only agrees with a `SubOrganizationOf`
/// reference to the **same** organization. `None` when either side has
/// no relationships recorded at all (the field is irrelevant for this
/// pair, not evidence of non-match). See spec §14a.
fn relationships_score(a: &[RelationshipRef], b: &[RelationshipRef]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let set_a: HashSet<(RelationKind, &str)> = a
        .iter()
        .map(|r| (r.relation, r.organization_id.as_str()))
        .collect();
    let set_b: HashSet<(RelationKind, &str)> = b
        .iter()
        .map(|r| (r.relation, r.organization_id.as_str()))
        .collect();
    Some(jaccard(&set_a, &set_b))
}

/// Tags component score: plain set **Jaccard** over the
/// case-insensitively normalised (folded) tag sets — `|A ∩ B| / |A ∪
/// B|`. Normalisation happens here, at scoring time — [`Organization::
/// tags`](crate::Organization::tags) stores tags verbatim. Unlike the
/// `keywords` component (§14, [`set_jaccard`]), which returns
/// `Some(0.0)` for a present-vs-absent disagreement, `tags_score`
/// returns `None` when either side has no tags recorded at all —
/// consistent with the equally-sparse, equally-supporting
/// `relationships_score` above. See spec §14b.
fn tags_score(a: &[String], b: &[String]) -> Option<f64> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let a_set: HashSet<String> = normalize::fold_set(a).into_iter().collect();
    let b_set: HashSet<String> = normalize::fold_set(b).into_iter().collect();
    // A list that folds down to nothing (e.g. only blank strings) still
    // carries no signal, mirroring the raw-emptiness guard above.
    if a_set.is_empty() || b_set.is_empty() {
        return None;
    }
    Some(jaccard(&a_set, &b_set))
}

/// Jaccard index `|A ∩ B| / |A ∪ B|` over two hash sets. Callers are
/// required to have already checked both inputs are non-empty (see
/// [`relationships_score`] / [`tags_score`]), so the union here is
/// never zero-sized.
///
/// Set sizes in practice are small (an organization's relationship or
/// tag list), so the `usize` counts are routed through `u32` (exact,
/// lint-free `u32 -> f64`) rather than a direct `usize -> f64` cast;
/// `unwrap_or(u32::MAX)` is a non-panicking saturating fallback for a
/// pathologically large set rather than a realistic code path.
fn jaccard<T: Eq + std::hash::Hash>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    let intersection = u32::try_from(a.intersection(b).count()).unwrap_or(u32::MAX);
    let union = u32::try_from(a.union(b).count()).unwrap_or(u32::MAX);
    f64::from(intersection) / f64::from(union)
}

// ─── Tests ───────────────────────────────────────────────────────

/// Unit tests for the matching engine and its per-component scorers.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{IdentifierScheme, OrgIdentifier, PostalAddress};

    /// Test helper: build an `OrgIdentifier` from a scheme + string value.
    fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme,
            value: value.into(),
        }
    }

    /// Two byte-identical names must score essentially perfectly and match.
    #[test]
    fn identical_orgs_score_high() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme Corporation");
        let b = Organization::new("Acme Corporation");
        let r = engine.match_organizations(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert!(r.is_match);
    }

    /// Pins that legal-suffix stripping makes `"Acme, Inc."` and
    /// `"ACME Corporation"` equivalent for name scoring.
    #[test]
    fn legal_suffix_variants_match() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme, Inc.");
        let b = Organization::new("ACME Corporation");
        let r = engine.match_organizations(&a, &b);
        // Both normalise to "acme" → name score 1.0.
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
    }

    /// Pins R-0: a shared LEI forces score 1.0 and sets the
    /// `deterministic_match` flag even when the names are unrelated.
    #[test]
    fn lei_match_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Organization::new("A");
        let mut b = Organization::new("Totally Different");
        a.identifiers
            .push(ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12"));
        b.identifiers
            .push(ident(IdentifierScheme::Lei, "5493001KJTIIGC8Y1R12"));
        let r = engine.match_organizations(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    /// Pins that classification codes (NAICS) describe a sector, not an
    /// entity, so a shared code must NOT short-circuit or force a match.
    #[test]
    fn classification_code_does_not_short_circuit() {
        let engine = MatchingEngine::default_config();
        let mut a = Organization::new("Alpha Foods");
        let mut b = Organization::new("Beta Mining");
        a.identifiers.push(ident(IdentifierScheme::Naics, "541511"));
        b.identifiers.push(ident(IdentifierScheme::Naics, "541511"));
        let r = engine.match_organizations(&a, &b);
        assert!(!r.breakdown.deterministic_match);
        assert!(!r.is_match);
    }

    /// Pins R-1's jurisdiction guard: a shared tax id only short-circuits
    /// when both records share a (case-folded) jurisdiction — never with
    /// no jurisdiction, never across differing ones.
    #[test]
    fn tax_id_short_circuits_only_within_jurisdiction() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.identifiers
            .push(ident(IdentifierScheme::TaxId, "12-3456789"));
        b.identifiers
            .push(ident(IdentifierScheme::TaxId, "12-3456789"));
        // No jurisdiction → no short-circuit.
        assert!(!deterministic_match(&a, &b));
        // Same jurisdiction → fires.
        a.jurisdiction = Some("US".into());
        b.jurisdiction = Some("us".into());
        assert!(deterministic_match(&a, &b));
        // Different jurisdiction → does not fire.
        b.jurisdiction = Some("GB".into());
        assert!(!deterministic_match(&a, &b));
    }

    /// Pins R-2: overlapping `same_as` URLs short-circuit, and that the
    /// comparison is whitespace-tolerant (one side is padded with spaces).
    #[test]
    fn same_as_overlap_short_circuits() {
        let engine = MatchingEngine::default_config();
        let mut a = Organization::new("Alpha");
        let mut b = Organization::new("Omega");
        a.same_as = vec!["https://www.wikidata.org/wiki/Q312".into()];
        b.same_as = vec!["  https://www.wikidata.org/wiki/Q312  ".into()];
        let r = engine.match_organizations(&a, &b);
        assert!((r.score - 1.0).abs() < 1e-9);
        assert!(r.breakdown.deterministic_match);
    }

    /// Pins that two clearly unrelated names score below 0.5 and don't match.
    #[test]
    fn unrelated_orgs_score_low() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme Corporation");
        let b = Organization::new("Globex Industries");
        let r = engine.match_organizations(&a, &b);
        assert!(r.score < 0.5, "expected low, got {}", r.score);
        assert!(!r.is_match);
    }

    /// Pins domain normalisation: differing scheme / `www.` / path all
    /// strip away, so the two URLs share a domain and score 1.0.
    #[test]
    fn url_domain_equality_scores_one() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.url = Some("https://www.Acme.com/about".into());
        b.url = Some("http://acme.com/contact".into());
        assert_eq!(url_score(&a, &b), Some(1.0));
    }

    /// Pins that a one-sided URL yields `None` (component skipped), not 0.0.
    #[test]
    fn url_none_when_one_side_missing() {
        let mut a = Organization::new("A");
        let b = Organization::new("B");
        a.url = Some("https://acme.com".into());
        assert_eq!(url_score(&a, &b), None);
    }

    /// Pins the binary jurisdiction score: case-folded equal → 1.0,
    /// different country → 0.0.
    #[test]
    fn jurisdiction_exact_and_mismatch() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.jurisdiction = Some("US".into());
        b.jurisdiction = Some("us".into());
        assert_eq!(jurisdiction_score(&a, &b), Some(1.0));
        b.jurisdiction = Some("GB".into());
        assert_eq!(jurisdiction_score(&a, &b), Some(0.0));
    }

    /// Pins the year-proximity bands: same year 1.0 (despite differing
    /// precision), one year off 0.5, far apart 0.0.
    #[test]
    fn founding_date_year_proximity() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.founding_date = Some("1998-09-04".into());
        b.founding_date = Some("1998".into());
        assert_eq!(founding_date_score(&a, &b), Some(1.0));
        b.founding_date = Some("1999".into());
        assert_eq!(founding_date_score(&a, &b), Some(0.5));
        b.founding_date = Some("2010".into());
        assert_eq!(founding_date_score(&a, &b), Some(0.0));
    }

    /// Pins field-by-field address scoring with renormalisation: only
    /// `locality` + `postal_code` are present, both agree (case-folded),
    /// so the score is ~1.0 despite the other three fields being absent.
    #[test]
    fn address_field_by_field() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        let addr = |city: &str, pc: &str| PostalAddress {
            locality: Some(city.into()),
            postal_code: Some(pc.into()),
            ..Default::default()
        };
        a.address = Some(addr("Mountain View", "94043"));
        b.address = Some(addr("mountain view", "94043"));
        let s = address_score(&a, &b).expect("some");
        assert!(s >= 0.99, "expected ~1.0, got {s}");
    }

    /// Pins that a one-sided address yields `None`, so it is skipped.
    #[test]
    fn address_none_when_one_side_missing() {
        let mut a = Organization::new("A");
        let b = Organization::new("B");
        a.address = Some(PostalAddress {
            locality: Some("X".into()),
            ..Default::default()
        });
        assert_eq!(address_score(&a, &b), None);
    }

    /// Pins the postal-code exact-anchor boost: a fold-equal postal code
    /// nudges up an address score that would otherwise be dragged down
    /// by a fuzzily-disagreeing street — by exactly
    /// `POSTAL_CODE_ANCHOR_BONUS`, capped at `POSTAL_CODE_ANCHOR_CEILING`.
    #[test]
    fn postal_code_anchor_lifts_a_partially_disagreeing_address() {
        let street_a = "100 Main Street";
        let street_b = "999 Oak Avenue";
        let postal = "62701";

        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.address = Some(PostalAddress {
            street_address: Some(street_a.into()),
            postal_code: Some(postal.into()),
            ..Default::default()
        });
        b.address = Some(PostalAddress {
            // Deliberately dissimilar street so the pre-anchor score sits
            // well below the ceiling — the anchor's effect must be
            // visible, not masked by an already-high base score.
            street_address: Some(street_b.into()),
            postal_code: Some(postal.into()),
            ..Default::default()
        });

        // Reconstruct the pre-anchor weighted average by hand: only
        // street (0.30) and postal (0.20) are present on both sides, and
        // the postal codes are fold-equal so their Jaro-Winkler is 1.0.
        let street_jw = jaro_winkler(&normalize::fold(street_a), &normalize::fold(street_b));
        let pre_anchor = (street_jw * 0.30 + 1.0 * 0.20) / (0.30 + 0.20);
        assert!(
            pre_anchor < POSTAL_CODE_ANCHOR_CEILING,
            "test premise: pre-anchor score must be below the ceiling, got {pre_anchor}"
        );

        let anchored = address_score(&a, &b).expect("some");
        assert!(
            anchored > pre_anchor,
            "expected the postal-code anchor to lift the score above {pre_anchor}, got {anchored}"
        );
        assert!(
            (anchored - (pre_anchor + POSTAL_CODE_ANCHOR_BONUS).min(POSTAL_CODE_ANCHOR_CEILING))
                .abs()
                < 1e-9
        );
        assert!(anchored <= POSTAL_CODE_ANCHOR_CEILING);
    }

    /// The anchor must never fire on a blank (post-fold) postal code
    /// shared by both sides — an empty value carries no identity.
    #[test]
    fn postal_code_anchor_does_not_fire_on_blank_postal_codes() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.address = Some(PostalAddress {
            street_address: Some("100 Main Street".into()),
            postal_code: Some("   ".into()),
            ..Default::default()
        });
        b.address = Some(PostalAddress {
            street_address: Some("999 Oak Avenue".into()),
            postal_code: Some(String::new()),
            ..Default::default()
        });
        let s = address_score(&a, &b).expect("some");
        // Bare Jaro-Winkler on the dissimilar street addresses alone,
        // with no anchor contribution — well under the ceiling.
        assert!(s < POSTAL_CODE_ANCHOR_CEILING, "got {s}");
    }

    /// The anchor requires postal codes on **both** sides; a one-sided
    /// postal code must not spuriously anchor.
    #[test]
    fn postal_code_anchor_does_not_fire_when_one_side_absent() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.address = Some(PostalAddress {
            street_address: Some("100 Main Street".into()),
            postal_code: Some("62701".into()),
            ..Default::default()
        });
        b.address = Some(PostalAddress {
            street_address: Some("999 Oak Avenue".into()),
            ..Default::default()
        });
        let s = address_score(&a, &b).expect("some");
        assert!(s < POSTAL_CODE_ANCHOR_CEILING, "got {s}");
    }

    /// Pins Jaccard: one shared tag of three distinct → 1/3, and that
    /// two empty inputs yield `None` (component skipped).
    #[test]
    fn keywords_jaccard() {
        let a = vec!["software".to_string(), "ai".to_string()];
        let b = vec!["ai".to_string(), "hardware".to_string()];
        let got = set_jaccard(&a, &b).expect("some");
        assert!((got - 1.0 / 3.0).abs() < 1e-9, "got {got}");
        assert_eq!(set_jaccard(&[], &[]), None);
    }

    /// Pins that `rank` orders candidates by descending score (best
    /// first, Globex never first) and `find_matches` keeps only the
    /// above-threshold entries.
    #[test]
    fn rank_orders_by_score_and_find_matches_filters() {
        let engine = MatchingEngine::default_config();
        let query = Organization::new("Acme Corporation");
        let cands = vec![
            Organization::new("Globex Industries"),
            Organization::new("Acme Corp"),
            Organization::new("Acme Corporation"),
        ];
        let ranked = engine.rank(&query, &cands);
        // "Acme Corp" and "Acme Corporation" both normalise to "acme",
        // so both score ~1.0 and outrank Globex (index 0).
        assert!(ranked[0].1.score >= 0.99);
        assert_ne!(ranked[0].0, 0, "Globex must not rank first");
        let matches = engine.find_matches(&query, &cands);
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().all(|(_, r)| r.is_match));
        assert!(matches.iter().all(|(i, _)| *i != 0));
    }

    /// Pins the Soundex bonus path through `name_score` (spec §9): two
    /// names whose literal Jaro-Winkler is below the High band yet share
    /// a Soundex code must score *strictly higher* than the same pair
    /// would without the bonus — confirming `PHONETIC_BONUS` actually
    /// lifts the name component, and that the lift is capped at
    /// `PHONETIC_CEILING`.
    #[test]
    fn name_score_soundex_bonus_lifts_near_miss() {
        // "Smith" vs "Smyth": Soundex-equal (S530) but a literal
        // Jaro-Winkler below the 0.95 ceiling, so the bonus applies.
        let a = Organization::new("Smith");
        let b = Organization::new("Smyth");
        let ak = name_keys(&a);
        let bk = name_keys(&b);
        let raw = jaro_winkler(&ak[0], &bk[0]);
        assert!(
            raw < PHONETIC_CEILING,
            "test premise: raw JW must be below the ceiling, got {raw}"
        );
        assert!(
            phonetic::same(&ak[0], &bk[0]),
            "test premise: the pair must be Soundex-equal"
        );
        let scored = name_score(&a, &b);
        // The bonus lifted the component above the bare Jaro-Winkler…
        assert!(
            scored > raw,
            "expected Soundex bonus to lift name score above raw {raw}, got {scored}"
        );
        // …by exactly PHONETIC_BONUS (capped at the ceiling).
        assert!((scored - (raw + PHONETIC_BONUS).min(PHONETIC_CEILING)).abs() < 1e-9);
        assert!(scored <= PHONETIC_CEILING);
    }

    /// Pins the matcher-level consequence of the `legal_name` never-empty
    /// fallback (spec §8): two organizations whose names are *only* legal
    /// suffixes (`"The Co"` vs `"Inc"`) must NOT spuriously score a
    /// perfect 1.0 against each other. The normaliser keeps a non-empty
    /// fallback rather than collapsing both to `""` (which would match).
    #[test]
    fn suffix_only_names_do_not_spuriously_match() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("The Co");
        let b = Organization::new("Inc");
        let r = engine.match_organizations(&a, &b);
        // Neither side normalises to empty, so they are compared on real
        // (distinct) tokens and must not reach the High band.
        assert!(
            r.score < 0.95,
            "suffix-only names must not score ~1.0, got {}",
            r.score
        );
        assert!(!r.breakdown.deterministic_match);
    }

    /// Pins the one-sided keywords case of `set_jaccard` (spec §14):
    /// exactly one side carrying keywords is a present-vs-absent
    /// disagreement and must yield `Some(0.0)`, not `None` (skip) and not
    /// a higher score.
    #[test]
    fn keywords_jaccard_one_sided_is_zero() {
        let with = vec!["software".to_string(), "ai".to_string()];
        assert_eq!(set_jaccard(&with, &[]), Some(0.0));
        assert_eq!(set_jaccard(&[], &with), Some(0.0));
    }

    /// Pins that `match_one_to_many` returns results in candidate order
    /// (no sorting) and handles an empty candidate slice.
    #[test]
    fn match_one_to_many_preserves_order_and_handles_empty() {
        let engine = MatchingEngine::default_config();
        let query = Organization::new("Acme");
        assert!(engine.match_one_to_many(&query, &[]).is_empty());
        let cands = vec![Organization::new("Zeta"), Organization::new("Acme")];
        let out = engine.match_one_to_many(&query, &cands);
        assert_eq!(out.len(), 2);
        assert!(out[1].score > out[0].score);
    }

    // ---------- relationships & tags (§14a / §14b) ----------

    #[test]
    fn relationships_score_identical_sets_scores_one() {
        let a = vec![RelationshipRef::new(RelationKind::SubOrganizationOf, "org-1").unwrap()];
        let b = a.clone();
        assert_eq!(relationships_score(&a, &b), Some(1.0));
    }

    #[test]
    fn relationships_score_disjoint_sets_scores_zero() {
        let a = vec![RelationshipRef::new(RelationKind::SubOrganizationOf, "org-1").unwrap()];
        let b = vec![RelationshipRef::new(RelationKind::ParentOrganizationOf, "org-1").unwrap()];
        // Same organization id, different relation kind: not the same pair.
        assert_eq!(relationships_score(&a, &b), Some(0.0));
    }

    #[test]
    fn relationships_score_partial_overlap_scores_jaccard_ratio() {
        let a = vec![
            RelationshipRef::new(RelationKind::SubOrganizationOf, "org-1").unwrap(),
            RelationshipRef::new(RelationKind::SuccessorOf, "org-2").unwrap(),
        ];
        let b = vec![
            RelationshipRef::new(RelationKind::SubOrganizationOf, "org-1").unwrap(),
            RelationshipRef::new(RelationKind::SuccessorOf, "org-3").unwrap(),
        ];
        // intersection = {(SubOrganizationOf, org-1)} = 1; union = 3.
        let score = relationships_score(&a, &b).expect("some");
        assert!((score - 1.0 / 3.0).abs() < 1e-9, "got {score}");
    }

    #[test]
    fn relationships_score_empty_either_side_is_none() {
        let empty: Vec<RelationshipRef> = vec![];
        let some = vec![RelationshipRef::new(RelationKind::SubOrganizationOf, "org-1").unwrap()];
        assert_eq!(relationships_score(&empty, &some), None);
        assert_eq!(relationships_score(&some, &empty), None);
        assert_eq!(relationships_score(&empty, &empty), None);
    }

    #[test]
    fn tags_score_case_insensitive_identical_scores_one() {
        let a = vec!["Vendor".to_string(), "Tier1".to_string()];
        let b = vec!["vendor".to_string(), "tier1".to_string()];
        assert_eq!(tags_score(&a, &b), Some(1.0));
    }

    #[test]
    fn tags_score_disjoint_sets_scores_zero() {
        let a = vec!["vendor".to_string()];
        let b = vec!["customer".to_string()];
        assert_eq!(tags_score(&a, &b), Some(0.0));
    }

    #[test]
    fn tags_score_partial_overlap_scores_jaccard_ratio() {
        let a = vec!["vendor".to_string(), "tier1".to_string()];
        let b = vec!["Vendor".to_string(), "tier2".to_string()];
        // intersection = {"vendor"} = 1; union = {"vendor", "tier1", "tier2"} = 3.
        let score = tags_score(&a, &b).expect("some");
        assert!((score - 1.0 / 3.0).abs() < 1e-9, "got {score}");
    }

    #[test]
    fn tags_score_empty_either_side_is_none() {
        let empty: Vec<String> = vec![];
        let some = vec!["vendor".to_string()];
        assert_eq!(tags_score(&empty, &some), None);
        assert_eq!(tags_score(&some, &empty), None);
        assert_eq!(tags_score(&empty, &empty), None);
    }

    /// Renormalisation sanity check: with neither field populated on
    /// either side, `relationships_score`/`tags_score` are `None` and
    /// neither weight enters the denominator — a name-only exact match
    /// still scores a clean ~1.0, not diluted by two "missing"
    /// components treated as zero.
    #[test]
    fn relationships_and_tags_absent_do_not_enter_the_weighted_average() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme Corporation");
        let b = Organization::new("Acme Corporation");
        let r = engine.match_organizations(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert_eq!(r.breakdown.relationships_score, None);
        assert_eq!(r.breakdown.tags_score, None);
    }
}
