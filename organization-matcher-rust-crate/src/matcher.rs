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

use strsim::jaro_winkler;

use crate::config::MatchConfig;
use crate::normalize;
use crate::organization::{IdentifierScheme, Organization};
use crate::phonetic;
use crate::scoring::{Confidence, MatchBreakdown, MatchResult, weighted_average};

const PHONETIC_BONUS: f64 = 0.05;
const PHONETIC_CEILING: f64 = 0.95;

/// The organization matcher: holds a [`MatchConfig`] and scores pairs.
pub struct MatchingEngine {
    config: MatchConfig,
}

impl MatchingEngine {
    /// Build a matcher with the given configuration.
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
        let address_score = address_score(a, b);
        let url_score = url_score(a, b);
        let jurisdiction_score = jurisdiction_score(a, b);
        let founding_date_score = founding_date_score(a, b);
        let keywords_score = set_jaccard(&a.keywords, &b.keywords);

        let score = weighted_average(&[
            (name_score, self.config.name_weight),
            (address_score, self.config.address_weight),
            (url_score, self.config.url_weight),
            (jurisdiction_score, self.config.jurisdiction_weight),
            (founding_date_score, self.config.founding_date_weight),
            (keywords_score, self.config.keywords_weight),
        ]);

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
                deterministic_match: false,
            },
        }
    }

    /// One-to-many: score `query` against each candidate, results in
    /// input order.
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

    /// One-to-many: `(index, result)` sorted by descending score.
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

fn deterministic_match(a: &Organization, b: &Organization) -> bool {
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

    // R-1 — same jurisdiction + same tax id.
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

/// Names contributing to the name score: `name`, `legal_name`, and
/// `alternate_names`, each compared in legal-suffix-normalised form.
fn name_keys(o: &Organization) -> Vec<String> {
    let mut keys = vec![normalize::legal_name(&o.name)];
    if let Some(ln) = &o.legal_name {
        keys.push(normalize::legal_name(ln));
    }
    for alt in &o.alternate_names {
        keys.push(normalize::legal_name(alt));
    }
    keys.retain(|k| !k.is_empty());
    keys
}

fn name_score(a: &Organization, b: &Organization) -> f64 {
    let a_keys = name_keys(a);
    let b_keys = name_keys(b);
    let mut best = 0.0_f64;
    for ak in &a_keys {
        for bk in &b_keys {
            best = best.max(jaro_winkler(ak, bk));
        }
    }
    // Soundex bonus on the primary normalised names, capped.
    let ap = a_keys.first().map_or("", String::as_str);
    let bp = b_keys.first().map_or("", String::as_str);
    if best < PHONETIC_CEILING && phonetic::same(ap, bp) {
        best = (best + PHONETIC_BONUS).min(PHONETIC_CEILING);
    }
    best
}

fn address_score(a: &Organization, b: &Organization) -> Option<f64> {
    let (Some(aa), Some(ba)) = (&a.address, &b.address) else {
        return None;
    };
    // Weighted field-by-field Jaro-Winkler; only fields present on both
    // sides contribute.
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
        if let (Some(x), Some(y)) = (x, y) {
            let (xf, yf) = (normalize::fold(x), normalize::fold(y));
            if xf.is_empty() && yf.is_empty() {
                continue;
            }
            sum += jaro_winkler(&xf, &yf) * w;
            wsum += w;
        }
    }
    if wsum > 0.0 { Some(sum / wsum) } else { None }
}

fn url_score(a: &Organization, b: &Organization) -> Option<f64> {
    let (Some(au), Some(bu)) = (a.url.as_deref(), b.url.as_deref()) else {
        return None;
    };
    let (ad, bd) = (normalize::domain(au), normalize::domain(bu));
    if ad.is_empty() || bd.is_empty() {
        return None;
    }
    Some(if ad == bd {
        1.0
    } else {
        jaro_winkler(&ad, &bd)
    })
}

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

fn founding_year(s: &str) -> Option<i32> {
    let head: String = s.trim().chars().take(4).collect();
    head.parse::<i32>().ok()
}

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
    use crate::organization::{IdentifierScheme, OrgIdentifier, PostalAddress};

    fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme,
            value: value.into(),
        }
    }

    #[test]
    fn identical_orgs_score_high() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme Corporation");
        let b = Organization::new("Acme Corporation");
        let r = engine.match_organizations(&a, &b);
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
        assert!(r.is_match);
    }

    #[test]
    fn legal_suffix_variants_match() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme, Inc.");
        let b = Organization::new("ACME Corporation");
        let r = engine.match_organizations(&a, &b);
        // Both normalise to "acme" → name score 1.0.
        assert!(r.score >= 0.99, "expected ~1.0, got {}", r.score);
    }

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

    #[test]
    fn unrelated_orgs_score_low() {
        let engine = MatchingEngine::default_config();
        let a = Organization::new("Acme Corporation");
        let b = Organization::new("Globex Industries");
        let r = engine.match_organizations(&a, &b);
        assert!(r.score < 0.5, "expected low, got {}", r.score);
        assert!(!r.is_match);
    }

    #[test]
    fn url_domain_equality_scores_one() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("B");
        a.url = Some("https://www.Acme.com/about".into());
        b.url = Some("http://acme.com/contact".into());
        assert_eq!(url_score(&a, &b), Some(1.0));
    }

    #[test]
    fn url_none_when_one_side_missing() {
        let mut a = Organization::new("A");
        let b = Organization::new("B");
        a.url = Some("https://acme.com".into());
        assert_eq!(url_score(&a, &b), None);
    }

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

    #[test]
    fn keywords_jaccard() {
        let a = vec!["software".to_string(), "ai".to_string()];
        let b = vec!["ai".to_string(), "hardware".to_string()];
        let got = set_jaccard(&a, &b).expect("some");
        assert!((got - 1.0 / 3.0).abs() < 1e-9, "got {got}");
        assert_eq!(set_jaccard(&[], &[]), None);
    }

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
}
