//! Pure rules for per-user saved views (T-28l): payload validation.
//! DB-free and unit-tested.

use crate::validation::{MAX_ARRAY_LEN, MAX_ITEM_LEN, MAX_TEXT_LEN};

/// A saved view's short, human-facing name.
pub const MAX_NAME_LEN: usize = 128;
/// A route string's cap — a front-end path, never a whole document.
pub const MAX_ROUTE_LEN: usize = 256;

/// Validate a saved-view payload's shape: every problem, not just the
/// first (family convention).
#[must_use]
pub fn problems(route: &str, name: &str, columns: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if route.trim().is_empty() {
        out.push("route is required".to_string());
    } else if route.len() > MAX_ROUTE_LEN {
        out.push(format!("route: exceeds {MAX_ROUTE_LEN} characters"));
    }
    if name.trim().is_empty() {
        out.push("name is required".to_string());
    } else if name.len() > MAX_NAME_LEN {
        out.push(format!("name: exceeds {MAX_NAME_LEN} characters"));
    }
    if columns.len() > MAX_ARRAY_LEN {
        out.push(format!("columns: exceeds {MAX_ARRAY_LEN} entries"));
    }
    for (i, col) in columns.iter().enumerate() {
        if col.trim().is_empty() {
            out.push(format!("columns[{i}]: must not be blank"));
        } else if col.chars().count() > MAX_ITEM_LEN {
            out.push(format!("columns[{i}]: exceeds {MAX_ITEM_LEN} characters"));
        }
    }
    out
}

/// The cap on the JSON-encoded `filter`/`sort` blobs, so a caller
/// cannot use a "preference" endpoint to store an arbitrarily large
/// document.
#[must_use]
pub fn json_blob_too_large(value: &serde_json::Value) -> bool {
    serde_json::to_string(value).is_ok_and(|s| s.len() > MAX_TEXT_LEN * 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_route_and_name_are_named() {
        let found = problems("", "", &[]);
        assert!(found.iter().any(|p| p.contains("route is required")));
        assert!(found.iter().any(|p| p.contains("name is required")));
    }

    #[test]
    fn oversized_route_and_name_are_named() {
        let found = problems(&"a".repeat(300), &"b".repeat(200), &[]);
        assert!(found.iter().any(|p| p.contains("route")));
        assert!(found.iter().any(|p| p.contains("name")));
    }

    #[test]
    fn blank_and_oversized_columns_are_each_named() {
        let found = problems("/plans", "My view", &[String::new(), "x".repeat(600)]);
        assert!(found.iter().any(|p| p.contains("columns[0]")));
        assert!(found.iter().any(|p| p.contains("columns[1]")));
    }

    #[test]
    fn a_valid_payload_has_no_problems() {
        assert!(problems("/plans", "My view", &["name".to_string()]).is_empty());
    }

    #[test]
    fn json_blob_size_cap() {
        assert!(!json_blob_too_large(&serde_json::json!({"a": 1})));
        let huge = serde_json::json!({ "q": "x".repeat(MAX_TEXT_LEN * 9) });
        assert!(json_blob_too_large(&huge));
    }
}
