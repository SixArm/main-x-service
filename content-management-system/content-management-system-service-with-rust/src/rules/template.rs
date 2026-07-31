//! Template **region contracts** (CMS-R1, CMS-D6) — pure, DB-free.
//!
//! A template declares the named regions a channel must lay out, which
//! block kinds each region accepts, and how many blocks it holds. The
//! service renders nothing; this is the contract a channel reads, and
//! (from CMS-T6) the rule an entry's blocks are checked against.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::rules::tokens::BLOCK_KINDS;

/// Maximum regions one template may declare.
pub const MAX_REGIONS: usize = 32;
/// Maximum blocks a region may require or permit.
pub const MAX_REGION_BLOCKS: u32 = 512;

/// One declared region of a template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionSpec {
    /// Stable machine key, unique within the template.
    pub key: String,
    /// Human label for editors.
    pub label: String,
    /// Block kinds this region accepts; empty ⇒ any kind.
    #[serde(default)]
    pub allowed_block_kinds: Vec<String>,
    /// Minimum blocks the region requires.
    #[serde(default)]
    pub min: u32,
    /// Maximum blocks the region accepts; `None` ⇒ unbounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

/// Validate a declared region set, returning every problem found
/// (empty ⇒ valid).
#[must_use]
pub fn validate_regions(regions: &[RegionSpec]) -> Vec<String> {
    let mut problems = Vec::new();
    if regions.is_empty() {
        problems.push("regions must declare at least one region".to_string());
    }
    if regions.len() > MAX_REGIONS {
        problems.push(format!("regions exceeds {MAX_REGIONS} entries"));
    }
    let mut seen = BTreeSet::new();
    for region in regions {
        let key = region.key.as_str();
        if !crate::rules::schema::is_field_key(key) {
            problems.push(format!(
                "regions[{key}].key must be snake_case starting with a letter"
            ));
        }
        if !seen.insert(key) {
            problems.push(format!("regions[{key}].key is duplicated"));
        }
        if region.label.trim().is_empty() {
            problems.push(format!("regions[{key}].label is required"));
        }
        for kind in &region.allowed_block_kinds {
            if !BLOCK_KINDS.contains(&kind.as_str()) {
                problems.push(format!(
                    "regions[{key}].allowed_block_kinds entry {kind:?} is not a block kind"
                ));
            }
        }
        if region.min > MAX_REGION_BLOCKS {
            problems.push(format!("regions[{key}].min exceeds {MAX_REGION_BLOCKS}"));
        }
        match region.max {
            Some(max) if max > MAX_REGION_BLOCKS => {
                problems.push(format!("regions[{key}].max exceeds {MAX_REGION_BLOCKS}"));
            }
            // A region that requires more blocks than it accepts can
            // never be satisfied — an editor would be stuck with no way
            // to publish and no explanation.
            Some(max) if region.min > max => {
                problems.push(format!("regions[{key}].min exceeds max"));
            }
            _ => {}
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(key: &str) -> RegionSpec {
        RegionSpec {
            key: key.to_string(),
            label: key.to_string(),
            allowed_block_kinds: Vec::new(),
            min: 0,
            max: None,
        }
    }

    #[test]
    fn a_well_formed_region_set_validates() {
        let mut body = region("body");
        body.allowed_block_kinds = vec!["paragraph".to_string(), "image".to_string()];
        body.min = 1;
        body.max = Some(200);
        assert!(validate_regions(&[region("hero"), body]).is_empty());
    }

    #[test]
    fn keys_are_snake_case_and_unique() {
        let problems = validate_regions(&[region("Body"), region("a"), region("a")]);
        assert!(problems.iter().any(|p| p.contains("snake_case")));
        assert!(problems.iter().any(|p| p.contains("is duplicated")));
    }

    #[test]
    fn block_kinds_are_allow_listed() {
        let mut r = region("body");
        r.allowed_block_kinds = vec!["script".to_string()];
        assert!(
            validate_regions(&[r])
                .iter()
                .any(|p| p.contains("is not a block kind"))
        );
    }

    /// An unsatisfiable region would block publishing with no recourse.
    #[test]
    fn min_may_not_exceed_max() {
        let mut r = region("body");
        r.min = 3;
        r.max = Some(2);
        assert!(
            validate_regions(&[r])
                .iter()
                .any(|p| p.contains("min exceeds max"))
        );
    }

    #[test]
    fn an_empty_region_set_is_refused() {
        assert!(
            validate_regions(&[])
                .iter()
                .any(|p| p.contains("at least one region"))
        );
    }
}
