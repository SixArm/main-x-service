//! Field-level validation for incoming `WorkItem` payloads.
//!
//! The service stores the matcher's `WorkItem` verbatim, so payload
//! validation is the *service's* responsibility — the matcher is a pure
//! scoring library and performs none. These checks return human-readable
//! problem strings that the controller surfaces as a single `422
//! Unprocessable Entity` (every problem reported at once).
//!
//! ## Rules
//!
//! - **`name`** — required; must not be blank.
//! - **`goals[i].title`** — each goal title must not be blank.
//! - **`identifiers[i].value`** — must not be blank; a `Uuid`-scheme
//!   value must be a valid UUID, a `Uri`-scheme value must look like a URI.
//! - **`portfolio_ref`** — when present, must be a valid UUID.
//! - **`in_language`** — when present, must look like a BCP-47 tag.
//! - **`keywords[i]` / `tags[i]` / `alternate_names[i]`** — non-blank.
//! - **`relationships[i].work_item_id`** — non-blank.

use portfolio_matcher::{IdentifierScheme, WorkItem};

/// Collect every validation problem for `wi`. An empty vector means the
/// payload is valid. The controller joins these into one `422`.
#[must_use]
pub fn problems(wi: &WorkItem) -> Vec<String> {
    let mut out = Vec::new();

    if wi.name.trim().is_empty() {
        out.push("name is required".to_string());
    }

    for (i, alt) in wi.alternate_names.iter().enumerate() {
        if alt.trim().is_empty() {
            out.push(format!("alternate_names[{i}]: must not be blank"));
        }
    }

    for (i, goal) in wi.goals.iter().enumerate() {
        if goal.title.trim().is_empty() {
            out.push(format!("goals[{i}]: title must not be blank"));
        }
    }

    for (i, ident) in wi.identifiers.iter().enumerate() {
        let value = ident.value.trim();
        if value.is_empty() {
            out.push(format!("identifiers[{i}]: value must not be blank"));
            continue;
        }
        match ident.scheme {
            IdentifierScheme::Uuid if uuid::Uuid::parse_str(value).is_err() => {
                out.push(format!("identifiers[{i}]: {value:?} is not a valid UUID"));
            }
            IdentifierScheme::Uri if !looks_like_uri(value) => {
                out.push(format!("identifiers[{i}]: {value:?} is not a valid URI"));
            }
            _ => {}
        }
    }

    if let Some(parent) = &wi.portfolio_ref
        && uuid::Uuid::parse_str(parent.trim()).is_err()
    {
        out.push(format!("portfolio_ref: {parent:?} is not a valid UUID"));
    }

    if let Some(lang) = &wi.in_language
        && !looks_like_bcp47(lang.trim())
    {
        out.push(format!("in_language: {lang:?} is not a valid BCP-47 tag"));
    }

    for (i, kw) in wi.keywords.iter().enumerate() {
        if kw.trim().is_empty() {
            out.push(format!("keywords[{i}]: must not be blank"));
        }
    }
    for (i, tag) in wi.tags.iter().enumerate() {
        if tag.trim().is_empty() {
            out.push(format!("tags[{i}]: must not be blank"));
        }
    }
    for (i, rel) in wi.relationships.iter().enumerate() {
        if rel.work_item_id.trim().is_empty() {
            out.push(format!(
                "relationships[{i}]: work_item_id must not be blank"
            ));
        }
    }

    out
}

/// A pragmatic URI shape check: a non-empty alpha-led scheme followed by
/// `:` and a non-empty remainder. Not a full RFC-3986 parser — enough to
/// reject obviously non-URI free text.
fn looks_like_uri(s: &str) -> bool {
    match s.split_once(':') {
        Some((scheme, rest)) => {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                && !rest.is_empty()
        }
        None => false,
    }
}

/// A pragmatic BCP-47 shape check: hyphen-separated subtags of ASCII
/// alphanumerics, the first being a 2–3 letter primary language. Not the
/// full registry — enough to reject obvious junk.
fn looks_like_bcp47(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut parts = s.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use portfolio_matcher::{Goal, WorkItem, WorkItemIdentifier, WorkItemKind};

    fn project(name: &str) -> WorkItem {
        WorkItem::new(WorkItemKind::Project, name)
    }

    #[test]
    fn valid_work_item_has_no_problems() {
        let mut wi = project("Apollo platform migration");
        wi.goals = vec![Goal {
            title: "Cut latency".into(),
            ..Default::default()
        }];
        wi.keywords = vec!["infra".into()];
        wi.in_language = Some("en".into());
        wi.portfolio_ref = Some(uuid::Uuid::new_v4().to_string());
        wi.identifiers = vec![WorkItemIdentifier {
            scheme: IdentifierScheme::JiraProjectKey,
            value: "APOLLO".into(),
        }];
        assert!(problems(&wi).is_empty(), "{:?}", problems(&wi));
    }

    #[test]
    fn blank_name_is_a_problem() {
        assert_eq!(
            problems(&project("   ")),
            vec!["name is required".to_string()]
        );
    }

    #[test]
    fn blank_goal_title_is_a_problem() {
        let mut wi = project("Apollo");
        wi.goals = vec![Goal {
            title: "  ".into(),
            ..Default::default()
        }];
        let p = problems(&wi);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("goals[0]"));
    }

    #[test]
    fn malformed_uuid_identifier_is_a_problem() {
        let mut wi = project("Apollo");
        wi.identifiers = vec![WorkItemIdentifier {
            scheme: IdentifierScheme::Uuid,
            value: "not-a-uuid".into(),
        }];
        let p = problems(&wi);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("identifiers[0]"));
    }

    #[test]
    fn malformed_uri_identifier_is_a_problem() {
        let mut wi = project("Apollo");
        wi.identifiers = vec![WorkItemIdentifier {
            scheme: IdentifierScheme::Uri,
            value: "just text".into(),
        }];
        assert_eq!(problems(&wi).len(), 1);
        wi.identifiers = vec![WorkItemIdentifier {
            scheme: IdentifierScheme::Uri,
            value: "https://pm.example.com/p/1".into(),
        }];
        assert!(problems(&wi).is_empty());
    }

    #[test]
    fn malformed_portfolio_ref_is_a_problem() {
        let mut wi = project("Apollo");
        wi.portfolio_ref = Some("nope".into());
        let p = problems(&wi);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("portfolio_ref"));
    }

    #[test]
    fn malformed_in_language_is_a_problem() {
        let mut wi = project("Apollo");
        wi.in_language = Some("english!".into());
        assert_eq!(problems(&wi).len(), 1);
        wi.in_language = Some("en-GB".into());
        assert!(problems(&wi).is_empty());
    }

    #[test]
    fn reports_every_issue_at_once() {
        let mut wi = project(""); // blank name
        wi.in_language = Some("zzzz!".into()); // bad
        wi.identifiers = vec![WorkItemIdentifier {
            scheme: IdentifierScheme::Uuid,
            value: " ".into(), // blank
        }];
        let p = problems(&wi);
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(p.iter().any(|m| m.contains("name is required")));
        assert!(p.iter().any(|m| m.contains("in_language")));
        assert!(p.iter().any(|m| m.contains("identifiers[0]")));
    }
}
