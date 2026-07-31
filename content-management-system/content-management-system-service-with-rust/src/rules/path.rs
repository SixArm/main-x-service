//! Paths and redirects (CMS-R17, CMS-D10) — pure, DB-free.
//!
//! Two rules carry this module, and both are enforced **at write
//! time**:
//!
//! 1. **A path has exactly one normal form.** `/About/`, `//about`, and
//!    `/about` are the same page, so they must not be able to become
//!    three different rows that disagree about which one is live.
//! 2. **A redirect chain is bounded and acyclic, by construction.** A
//!    loop discovered at request time is a request-time hang with an
//!    editorial cause; a loop refused at write time is a `422` an
//!    editor can fix in the moment. Over-long chains are collapsed to
//!    their final target when created, so resolution stays one lookup.
//!
//! The auto-`301` on a slug change lives on top of these: renaming a
//! page without leaving a redirect is the most common self-inflicted
//! injury in a CMS, so it is the default rather than an option.

use std::collections::BTreeSet;

/// Maximum characters in a path.
pub const MAX_PATH_LEN: usize = 512;
/// Maximum segments in a path.
pub const MAX_SEGMENTS: usize = 24;
/// Default maximum redirect hops followed at resolution.
pub const DEFAULT_MAX_HOPS: usize = 5;

/// Normalize `path` to its single canonical form.
///
/// - a leading slash is added when missing;
/// - a trailing slash is removed (except for the root);
/// - duplicate slashes collapse;
/// - the path is lowercased (ASCII);
/// - `.` and `..` segments, control characters, and whitespace are
///   **refused** rather than stripped — silently rewriting a path an
///   editor typed produces a page at an address they did not choose.
///
/// # Errors
///
/// A message naming what is wrong with the path.
pub fn normalize(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("a path must not be empty".to_string());
    }
    if trimmed.len() > MAX_PATH_LEN {
        return Err(format!("a path must be at most {MAX_PATH_LEN} characters"));
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("a path must not contain whitespace or control characters".to_string());
    }
    if trimmed.contains('?') || trimmed.contains('#') {
        return Err("a path must not contain a query string or fragment".to_string());
    }
    // A percent escape is refused rather than decoded: decoding once
    // invites the question of what a doubly-encoded `..` means, and the
    // honest answer is that content paths do not need escapes.
    if trimmed.contains('%') {
        return Err("a path must not contain percent escapes".to_string());
    }

    let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    if segments.iter().any(|s| *s == "." || *s == "..") {
        return Err("a path must not contain `.` or `..` segments".to_string());
    }
    if segments.len() > MAX_SEGMENTS {
        return Err(format!("a path must have at most {MAX_SEGMENTS} segments"));
    }
    for segment in &segments {
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(format!(
                "path segment {segment:?} may contain only letters, digits, `-`, `_`, and `.`"
            ));
        }
    }
    if segments.is_empty() {
        return Ok("/".to_string());
    }
    Ok(format!("/{}", segments.join("/")).to_ascii_lowercase())
}

/// What a redirect points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    /// The path being redirected.
    pub from: String,
    /// Where it goes; `None` for a `410 Gone` marker.
    pub to: Option<String>,
    /// `301`, `302`, or `410`.
    pub status: u16,
}

/// The outcome of following a redirect chain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Followed {
    /// The final path, when there is one.
    pub target: Option<String>,
    /// The status to answer with.
    pub status: u16,
    /// The hops walked, in order.
    pub hops: Vec<String>,
    /// Why the walk stopped short, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<&'static str>,
}

/// Follow the chain starting at `from`, bounded by `max_hops`.
///
/// Loops are refused at write time ([`would_cycle`]), so meeting one
/// here means the table was edited behind the service's back. It is
/// still handled — reported, not followed — because a request-time hang
/// is the worst possible way to find out.
#[must_use]
pub fn follow(from: &str, redirects: &[Redirect], max_hops: usize) -> Followed {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut current = from.to_string();
    let mut hops = Vec::new();
    seen.insert(current.clone());

    for _ in 0..max_hops {
        let Some(redirect) = redirects.iter().find(|r| r.from == current) else {
            return Followed {
                target: (hops.is_empty()).then(|| current.clone()).or(Some(current)),
                status: if hops.is_empty() { 200 } else { 301 },
                hops,
                problem: None,
            };
        };
        // A `410` marker is a terminus, not a hop: the page is gone and
        // saying so is the answer.
        let Some(next) = redirect.to.clone() else {
            return Followed {
                target: None,
                status: redirect.status,
                hops,
                problem: None,
            };
        };
        if !seen.insert(next.clone()) {
            return Followed {
                target: None,
                status: 508,
                hops,
                problem: Some("redirect loop"),
            };
        }
        hops.push(next.clone());
        current = next;
    }
    Followed {
        target: None,
        status: 508,
        hops,
        problem: Some("too many redirect hops"),
    }
}

/// Whether adding `from → to` would create a cycle in `existing`.
///
/// Called before the write, so a loop never reaches storage.
#[must_use]
pub fn would_cycle(from: &str, to: &str, existing: &[Redirect]) -> bool {
    if from == to {
        return true;
    }
    let mut current = to.to_string();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(from.to_string());
    // Bounded by the table size: even a corrupt table terminates.
    for _ in 0..=existing.len() {
        if !seen.insert(current.clone()) {
            return true;
        }
        let Some(next) = existing
            .iter()
            .find(|r| r.from == current)
            .and_then(|r| r.to.clone())
        else {
            return false;
        };
        if next == from {
            return true;
        }
        current = next;
    }
    true
}

/// The final target of `to`, following `existing` — so a new redirect
/// is stored pointing at the end of the chain rather than at another
/// redirect.
///
/// This is why resolution stays one lookup no matter how many times a
/// page is renamed.
#[must_use]
pub fn collapse(to: &str, existing: &[Redirect], max_hops: usize) -> String {
    let followed = follow(to, existing, max_hops);
    followed.target.unwrap_or_else(|| to.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redirect(from: &str, to: &str) -> Redirect {
        Redirect {
            from: from.to_string(),
            to: Some(to.to_string()),
            status: 301,
        }
    }

    #[test]
    fn a_path_has_one_normal_form() {
        for input in ["/about", "about", "/about/", "//about//", "/About/"] {
            assert_eq!(normalize(input).as_deref(), Ok("/about"), "{input}");
        }
        assert_eq!(normalize("/").as_deref(), Ok("/"));
        assert_eq!(normalize("/a/b/c").as_deref(), Ok("/a/b/c"));
    }

    /// Refused, not silently rewritten: a page at an address the editor
    /// did not choose is worse than an error.
    #[test]
    fn dangerous_or_ambiguous_paths_are_refused() {
        for input in [
            "",
            "   ",
            "/a/../b",
            "/./a",
            "/a b",
            "/a\nb",
            "/a?x=1",
            "/a#frag",
            "/a%2e%2e/b",
            "/a/b!c",
        ] {
            assert!(normalize(input).is_err(), "{input:?} should be refused");
        }
        assert!(normalize(&format!("/{}", "a".repeat(MAX_PATH_LEN))).is_err());
        let deep = format!("/{}", vec!["a"; MAX_SEGMENTS + 1].join("/"));
        assert!(normalize(&deep).is_err());
    }

    #[test]
    fn a_path_with_no_redirect_resolves_to_itself() {
        let followed = follow("/about", &[], DEFAULT_MAX_HOPS);
        assert_eq!(followed.target.as_deref(), Some("/about"));
        assert_eq!(followed.status, 200);
        assert!(followed.hops.is_empty());
    }

    #[test]
    fn a_chain_is_followed_and_reported() {
        let table = vec![redirect("/old", "/middle"), redirect("/middle", "/new")];
        let followed = follow("/old", &table, DEFAULT_MAX_HOPS);
        assert_eq!(followed.target.as_deref(), Some("/new"));
        assert_eq!(followed.status, 301);
        assert_eq!(followed.hops, vec!["/middle", "/new"]);
    }

    /// A `410` is a terminus: the page is gone, and saying so is the
    /// answer rather than sending the reader somewhere else.
    #[test]
    fn a_gone_marker_ends_the_walk() {
        let table = vec![Redirect {
            from: "/retired".to_string(),
            to: None,
            status: 410,
        }];
        let followed = follow("/retired", &table, DEFAULT_MAX_HOPS);
        assert_eq!(followed.status, 410);
        assert!(followed.target.is_none());
    }

    /// A loop is reported, never followed — a request-time hang is the
    /// worst way to discover one.
    #[test]
    fn a_loop_is_reported_not_followed() {
        let table = vec![redirect("/a", "/b"), redirect("/b", "/a")];
        let followed = follow("/a", &table, DEFAULT_MAX_HOPS);
        assert_eq!(followed.problem, Some("redirect loop"));
        assert!(followed.target.is_none());
    }

    #[test]
    fn an_over_long_chain_stops_at_the_hop_cap() {
        let table: Vec<Redirect> = (0..10)
            .map(|i| redirect(&format!("/p{i}"), &format!("/p{}", i + 1)))
            .collect();
        let followed = follow("/p0", &table, DEFAULT_MAX_HOPS);
        assert_eq!(followed.problem, Some("too many redirect hops"));
        assert_eq!(followed.hops.len(), DEFAULT_MAX_HOPS);
    }

    /// The write-time check: a loop never reaches storage.
    #[test]
    fn a_cycle_is_detected_before_it_is_written() {
        let table = vec![redirect("/b", "/c"), redirect("/c", "/a")];
        // /a -> /b would close the ring a -> b -> c -> a.
        assert!(would_cycle("/a", "/b", &table));
        // /d -> /b is fine: it joins the chain without closing it.
        assert!(!would_cycle("/d", "/b", &table));
        // A self-redirect is the degenerate cycle.
        assert!(would_cycle("/a", "/a", &[]));
    }

    /// Even a corrupt table terminates.
    #[test]
    fn cycle_detection_terminates_on_a_pre_existing_loop() {
        let table = vec![redirect("/x", "/y"), redirect("/y", "/x")];
        assert!(would_cycle("/z", "/x", &table));
    }

    /// A new redirect points at the end of the chain, so resolution
    /// stays one lookup however many times a page is renamed.
    #[test]
    fn a_new_redirect_collapses_to_the_final_target() {
        let table = vec![redirect("/v1", "/v2"), redirect("/v2", "/v3")];
        assert_eq!(collapse("/v1", &table, DEFAULT_MAX_HOPS), "/v3");
        assert_eq!(collapse("/fresh", &table, DEFAULT_MAX_HOPS), "/fresh");
    }
}
