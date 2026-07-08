//! `user_attributes` task — the operator surface for **ABAC attribute
//! assignment**.
//!
//! Implements the sourcing-side operator action deferred in the shared
//! [`agents/share/authorization-attributes.md`] §6: assigning a user's
//! `users.attributes` (the string→strings subject-attribute map that is
//! copied into the session at establishment and minted into the PASETO
//! `attrs` claim). Until an operator assigns attributes a user has `{}`
//! and is read-only under the default policy; machine peers are given a
//! `svc=true` token by ops.
//!
//! This is a **CLI task first** (per §6): an HTTP admin API is a later
//! follow-up once an operator UI needs it. Run with:
//!
//! ```text
//! # show one user's current attributes
//! cargo loco task user_attributes email:alice@example.com
//! cargo loco task user_attributes op:show pid:0c4f1e2a-…
//!
//! # set (replace) one key's value list
//! cargo loco task user_attributes op:set email:alice@example.com key:access values:write
//! cargo loco task user_attributes op:set email:alice@example.com key:dept   values:cardiology,oncology
//!
//! # grant a machine peer everything
//! cargo loco task user_attributes op:set email:peer@svc key:svc values:true
//!
//! # remove one key, or clear every attribute (back to read-only)
//! cargo loco task user_attributes op:unset email:alice@example.com key:dept
//! cargo loco task user_attributes op:clear email:alice@example.com
//! ```
//!
//! The parsing / validation / map-mutation core (`AttrCommand`,
//! [`parse_values`], [`validate_key`], [`validate_value`], [`apply`]) is
//! pure and unit-tested here; the [`Task`] impl only adds the DB lookup,
//! the write via [`users::ActiveModel::set_attributes`], and the printed
//! report.
//!
//! [`agents/share/authorization-attributes.md`]:
//!     ../../../agents/share/authorization-attributes.md

use std::collections::BTreeMap;

use loco_rs::prelude::*;

use crate::models::{auth_events::Model as AuthEvent, users};

/// How the target user is named on the command line: exactly one of
/// `email:<addr>` or `pid:<uuid>`.
enum Selector {
    /// `email:<addr>` — the operator-friendly handle.
    Email(String),
    /// `pid:<uuid>` — the stable public id (handy once an address has
    /// been GDPR-erased to a tombstone).
    Pid(String),
}

/// A parsed attribute-assignment command over one user's attribute map.
///
/// `Show` is read-only; the rest mutate exactly one key (or, for
/// `Clear`, the whole map). Value lists **replace** a key's values —
/// there is no in-place add/remove, so the stored state is always
/// exactly what the last `set` specified (simplest to reason about for
/// an operator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrCommand {
    /// Print the current attributes; change nothing.
    Show,
    /// Replace `key`'s value list with `values` (creating the key if
    /// absent). `values` is non-empty (an empty list would match no
    /// policy value anyway — use `Unset` to remove a key).
    Set {
        /// The attribute key (validated by [`validate_key`]).
        key: String,
        /// The replacement value list (each validated by
        /// [`validate_value`]; de-duplicated, order preserved).
        values: Vec<String>,
    },
    /// Remove `key` entirely (no-op if absent).
    Unset {
        /// The attribute key to remove.
        key: String,
    },
    /// Remove every attribute (the user returns to read-only under the
    /// default policy).
    Clear,
}

impl AttrCommand {
    /// The short op name for the audit trail (`show` / `set` / `unset` /
    /// `clear`).
    #[must_use]
    pub fn op_name(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Set { .. } => "set",
            Self::Unset { .. } => "unset",
            Self::Clear => "clear",
        }
    }

    /// The attribute key this command changes, for the audit trail
    /// (`Set` / `Unset`); `None` for `Show` / `Clear`.
    #[must_use]
    pub fn changed_key(&self) -> Option<&str> {
        match self {
            Self::Set { key, .. } | Self::Unset { key } => Some(key),
            Self::Show | Self::Clear => None,
        }
    }
}

/// Split a `values:` argument (`"write,admin"`) into a validated,
/// de-duplicated, order-preserving list. Empty items (from stray commas
/// or trailing separators) are dropped; each surviving item is checked
/// by [`validate_value`].
///
/// # Errors
///
/// Returns a human-readable message if any item fails value validation,
/// or if the list is empty after trimming (use `op:unset` to remove a
/// key rather than setting it to nothing).
pub fn parse_values(raw: &str) -> std::result::Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for item in raw.split(',') {
        let value = item.trim();
        if value.is_empty() {
            continue;
        }
        validate_value(value)?;
        let value = value.to_string();
        if !out.contains(&value) {
            out.push(value);
        }
    }
    if out.is_empty() {
        return Err(
            "`values` is empty — provide at least one value, or use `op:unset` to remove a key"
                .to_string(),
        );
    }
    Ok(out)
}

/// Whether `s` is a valid attribute **key**: non-empty, lowercase, first
/// character `a`–`z`, remainder from `[a-z0-9_-]`. Keys are the
/// deployment-defined attribute names (`access`, `dept`, `svc`, …); the
/// design fixes them as "short lowercase strings"
/// (`authorization-attributes.md` §3).
///
/// # Errors
///
/// Returns a message naming the offending key, including the reserved
/// pseudo-attributes `sub` / `email` / `entity`, which resolve from the
/// verified claims / resource and must not be assigned as `attrs`.
pub fn validate_key(key: &str) -> std::result::Result<(), String> {
    if matches!(key, "sub" | "email" | "entity") {
        return Err(format!(
            "`{key}` is a reserved pseudo-attribute (resolved from the token/resource); it cannot be assigned as an attribute"
        ));
    }
    let mut chars = key.chars();
    let valid = match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid attribute key `{key}`: use lowercase, starting with a letter, from [a-z0-9_-]"
        ))
    }
}

/// Whether `s` is a valid attribute **value**: non-empty, lowercase,
/// from `[a-z0-9_.-]`. Values are the short lowercase tokens policies
/// match on (`write`, `admin`, `cardiology`, `true`, …).
///
/// # Errors
///
/// Returns a message naming the offending value.
pub fn validate_value(value: &str) -> std::result::Result<(), String> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-' | '.'));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid attribute value `{value}`: use lowercase from [a-z0-9_.-]"
        ))
    }
}

/// A per-deployment **attribute vocabulary** — the optional allow-set of
/// attribute **keys** and, per key, the allowed **values**. Configured
/// via `AUTH_ATTRIBUTE_VOCABULARY` (inline JSON) or
/// `AUTH_ATTRIBUTE_VOCABULARY_FILE` (path), e.g.
/// `{ "access": ["read","write","admin"], "dept": ["cardiology","oncology"], "svc": ["true"] }`.
///
/// It exists to catch **typos at assignment time**: without it,
/// `dept=cardiolgy` is a syntactically-valid attribute that silently
/// grants nothing; with it, the assignment is rejected. A key mapping to
/// an **empty** value list allows the key with **any** value. When
/// unconfigured the vocabulary is **unrestricted** — assignment keeps
/// today's behaviour (only the lowercase-token shape check applies).
#[derive(Debug, Clone, Default)]
pub struct AttributeVocabulary {
    /// `None` ⇒ unrestricted; `Some(map)` ⇒ allowed keys → allowed
    /// values (empty list ⇒ any value for that key).
    allowed: Option<BTreeMap<String, Vec<String>>>,
}

impl AttributeVocabulary {
    /// An unrestricted vocabulary (the default when nothing is
    /// configured): every shape-valid key/value is accepted.
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { allowed: None }
    }

    /// Parse a vocabulary from its JSON object form (`{ "<key>":
    /// ["<value>", …], … }`).
    ///
    /// # Errors
    ///
    /// The underlying [`serde_json::Error`] when the document is not a
    /// JSON object of string → string-array.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let map: BTreeMap<String, Vec<String>> = serde_json::from_str(json)?;
        Ok(Self { allowed: Some(map) })
    }

    /// Check that setting `key` to `values` is permitted by the
    /// vocabulary. Unrestricted ⇒ always `Ok`. Otherwise the key must be
    /// listed, and — when that key's allowed-value list is non-empty —
    /// every value must be in it.
    ///
    /// # Errors
    ///
    /// A human-readable message naming the offending key or value and,
    /// for a value, the allowed set.
    pub fn check(&self, key: &str, values: &[String]) -> std::result::Result<(), String> {
        let Some(allowed) = &self.allowed else {
            return Ok(());
        };
        let Some(allowed_values) = allowed.get(key) else {
            return Err(format!(
                "attribute key `{key}` is not in the configured vocabulary (AUTH_ATTRIBUTE_VOCABULARY)"
            ));
        };
        if allowed_values.is_empty() {
            return Ok(()); // key allowed, any value
        }
        for value in values {
            if !allowed_values.contains(value) {
                return Err(format!(
                    "value `{value}` is not allowed for `{key}` (allowed: {})",
                    allowed_values.join(", ")
                ));
            }
        }
        Ok(())
    }
}

/// The process-wide attribute vocabulary, read once from
/// `AUTH_ATTRIBUTE_VOCABULARY` (inline JSON) / `_FILE` (path). Unset or
/// unparsable ⇒ [`AttributeVocabulary::unrestricted`] (warn-logged on a
/// parse error) — assignment always works, matching the policy-load
/// posture. Read once; restart to change.
#[must_use]
pub fn vocabulary() -> &'static AttributeVocabulary {
    use std::sync::OnceLock;
    static VOCAB: OnceLock<AttributeVocabulary> = OnceLock::new();
    VOCAB.get_or_init(|| {
        let source = std::env::var("AUTH_ATTRIBUTE_VOCABULARY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                let path = std::env::var("AUTH_ATTRIBUTE_VOCABULARY_FILE")
                    .ok()
                    .filter(|v| !v.trim().is_empty())?;
                std::fs::read_to_string(path.trim()).ok()
            });
        match source {
            Some(json) => AttributeVocabulary::from_json(&json).unwrap_or_else(|error| {
                tracing::warn!(%error, "AUTH_ATTRIBUTE_VOCABULARY invalid; no vocabulary restriction applied");
                AttributeVocabulary::unrestricted()
            }),
            None => AttributeVocabulary::unrestricted(),
        }
    })
}

/// Apply a parsed [`AttrCommand`] to an attribute map, returning the new
/// map. Pure and total: `Show` returns the map unchanged; `Set` replaces
/// (or inserts) one key; `Unset` removes one key (no-op if absent);
/// `Clear` empties the map.
#[must_use]
pub fn apply(
    mut attrs: BTreeMap<String, Vec<String>>,
    command: &AttrCommand,
) -> BTreeMap<String, Vec<String>> {
    match command {
        AttrCommand::Show => {}
        AttrCommand::Set { key, values } => {
            attrs.insert(key.clone(), values.clone());
        }
        AttrCommand::Unset { key } => {
            attrs.remove(key);
        }
        AttrCommand::Clear => attrs.clear(),
    }
    attrs
}

/// Parse the CLI `op` / `key` / `values` arguments into an
/// [`AttrCommand`], validating as it goes.
///
/// `op` defaults to `show`. `set` requires both `key` and `values`;
/// `unset` requires `key`; `show` / `clear` take neither.
///
/// # Errors
///
/// Returns a human-readable message for an unknown `op`, a missing or
/// invalid `key`, or missing / invalid `values`.
fn parse_command(
    op: &str,
    key: Option<&str>,
    values: Option<&str>,
) -> std::result::Result<AttrCommand, String> {
    match op {
        "show" => Ok(AttrCommand::Show),
        "clear" => Ok(AttrCommand::Clear),
        "set" => {
            let key = key.ok_or("`op:set` requires `key:<name>`")?;
            validate_key(key)?;
            let values = values.ok_or("`op:set` requires `values:<v1,v2,…>`")?;
            Ok(AttrCommand::Set {
                key: key.to_string(),
                values: parse_values(values)?,
            })
        }
        "unset" => {
            let key = key.ok_or("`op:unset` requires `key:<name>`")?;
            validate_key(key)?;
            Ok(AttrCommand::Unset {
                key: key.to_string(),
            })
        }
        other => Err(format!(
            "unknown op `{other}`: expected one of show | set | unset | clear"
        )),
    }
}

/// Render an attribute map as stable, human-readable lines
/// (`key = v1, v2`), one per key, sorted (the `BTreeMap` order); or
/// `(none)` when empty. Used in the task's before/after report.
fn render(attrs: &BTreeMap<String, Vec<String>>) -> String {
    if attrs.is_empty() {
        return "(none — read-only under the default policy)".to_string();
    }
    attrs
        .iter()
        .map(|(key, values)| format!("  {key} = {}", values.join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `user_attributes` CLI task: assign / inspect a user's ABAC
/// subject attributes. See the module docs for usage.
pub struct UserAttributes;

#[async_trait]
impl Task for UserAttributes {
    /// Task metadata (name + one-line description) for the loco CLI listing.
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_attributes".to_string(),
            detail: "Assign or inspect a user's ABAC subject attributes (users.attributes → PASETO `attrs` claim). Args: (email:<addr>|pid:<uuid>) [op:show|set|unset|clear] [key:<name>] [values:<v1,v2,…>].".to_string(),
        }
    }

    /// Resolve the target user, parse the command, apply it (persisting
    /// on a mutation), and print a before/after report.
    ///
    /// # Errors
    ///
    /// Returns an error for bad / conflicting arguments, an unknown user,
    /// or a database failure.
    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let selector = parse_selector(vars).map_err(|e| Error::string(&e))?;
        let op = vars.cli.get("op").map_or("show", String::as_str);
        let command = parse_command(
            op,
            vars.cli.get("key").map(String::as_str),
            vars.cli.get("values").map(String::as_str),
        )
        .map_err(|e| Error::string(&e))?;

        let user = find_target(&ctx.db, &selector).await?;
        let before = user.attrs();

        if matches!(command, AttrCommand::Show) {
            println!("user {} <{}>", user.pid, user.email);
            println!("attributes:\n{}", render(&before));
            return Ok(());
        }

        // Enforce the configured attribute vocabulary on a `set` (catches
        // typos like `dept=cardiolgy`). Unset/clear need no check.
        if let AttrCommand::Set { key, values } = &command {
            vocabulary()
                .check(key, values)
                .map_err(|e| Error::string(&e))?;
        }

        let after = apply(before.clone(), &command);
        let pid = user.pid;
        let email = user.email.clone();
        user.into_active_model()
            .set_attributes(&ctx.db, users::attributes_to_value(&after))
            .await?;

        // Durable audit row for the assignment (best-effort — never
        // fails the task). Actor is `cli`; values are omitted (see
        // `record_attribute_assignment_best_effort`).
        AuthEvent::record_attribute_assignment_best_effort(
            &ctx.db,
            Some(&email),
            pid,
            command.op_name(),
            command.changed_key(),
            "cli",
        )
        .await;
        tracing::info!(user_pid = %pid, op = op, "operator updated user ABAC attributes");
        println!("user {pid} <{email}>");
        println!("before:\n{}", render(&before));
        println!("after:\n{}", render(&after));
        Ok(())
    }
}

/// Parse the target [`Selector`] from the CLI args: exactly one of
/// `email:` or `pid:` must be present.
fn parse_selector(vars: &task::Vars) -> std::result::Result<Selector, String> {
    match (vars.cli.get("email"), vars.cli.get("pid")) {
        (Some(_), Some(_)) => {
            Err("provide exactly one of `email:<addr>` or `pid:<uuid>`, not both".to_string())
        }
        (Some(email), None) => Ok(Selector::Email(email.clone())),
        (None, Some(pid)) => Ok(Selector::Pid(pid.clone())),
        (None, None) => {
            Err("provide the target user as `email:<addr>` or `pid:<uuid>`".to_string())
        }
    }
}

/// Look up the target user by the [`Selector`], mapping a not-found into
/// a clear operator-facing message.
async fn find_target(db: &DatabaseConnection, selector: &Selector) -> Result<users::Model> {
    let found = match selector {
        Selector::Email(email) => users::Model::find_by_email(db, email).await,
        Selector::Pid(pid) => users::Model::find_by_pid(db, pid).await,
    };
    found.map_err(|_| {
        let who = match selector {
            Selector::Email(email) => format!("email `{email}`"),
            Selector::Pid(pid) => format!("pid `{pid}`"),
        };
        Error::string(&format!("no user found for {who}"))
    })
}

#[cfg(test)]
mod tests {
    use super::{AttrCommand, apply, parse_command, parse_values, validate_key, validate_value};
    use std::collections::BTreeMap;

    fn map(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, vs)| {
                (
                    (*k).to_string(),
                    vs.iter().map(|v| (*v).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn parse_values_splits_trims_and_dedups_preserving_order() {
        assert_eq!(
            parse_values(" write , admin ,write").unwrap(),
            vec!["write".to_string(), "admin".to_string()]
        );
        assert_eq!(
            parse_values("cardiology").unwrap(),
            vec!["cardiology".to_string()]
        );
    }

    fn vals(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn attribute_vocabulary_enforces_keys_and_values() {
        let vocab = super::AttributeVocabulary::from_json(
            r#"{ "access": ["read", "write", "admin"], "dept": ["cardiology", "oncology"], "svc": [] }"#,
        )
        .expect("vocabulary parses");

        // Known key + allowed value ⇒ ok.
        assert!(vocab.check("access", &vals(&["write"])).is_ok());
        assert!(vocab.check("dept", &vals(&["cardiology"])).is_ok());
        // `svc` maps to an empty list ⇒ any value allowed.
        assert!(vocab.check("svc", &vals(&["true"])).is_ok());

        // Unknown key ⇒ rejected (the typo-catching case).
        assert!(vocab.check("departmnt", &vals(&["cardiology"])).is_err());
        // Known key, disallowed value ⇒ rejected (`dept=cardiolgy`).
        assert!(vocab.check("dept", &vals(&["cardiolgy"])).is_err());
        // One bad value among good ones still rejects.
        assert!(
            vocab
                .check("access", &vals(&["write", "superuser"]))
                .is_err()
        );
    }

    #[test]
    fn unrestricted_vocabulary_allows_anything() {
        let vocab = super::AttributeVocabulary::unrestricted();
        assert!(vocab.check("anything", &vals(&["at", "all"])).is_ok());
        assert!(vocab.check("access", &vals(&["write"])).is_ok());
    }

    #[test]
    fn parse_values_rejects_empty_and_invalid() {
        assert!(parse_values("").is_err());
        assert!(parse_values("  , ,").is_err());
        assert!(parse_values("Write").is_err()); // uppercase
        assert!(parse_values("a b").is_err()); // space
    }

    #[test]
    fn validate_key_accepts_lowercase_and_rejects_reserved_and_bad() {
        assert!(validate_key("access").is_ok());
        assert!(validate_key("dept_2").is_ok());
        assert!(validate_key("svc").is_ok());
        // Reserved pseudo-attributes cannot be assigned.
        for reserved in ["sub", "email", "entity"] {
            assert!(
                validate_key(reserved).is_err(),
                "{reserved} must be rejected"
            );
        }
        // Shape violations.
        assert!(validate_key("Access").is_err()); // uppercase
        assert!(validate_key("1abc").is_err()); // leading digit
        assert!(validate_key("").is_err());
        assert!(validate_key("a.b").is_err()); // dot not allowed in keys
    }

    #[test]
    fn validate_value_charset() {
        assert!(validate_value("write").is_ok());
        assert!(validate_value("true").is_ok());
        assert!(validate_value("purpose.care").is_ok());
        assert!(validate_value("Write").is_err());
        assert!(validate_value("").is_err());
        assert!(validate_value("a b").is_err());
    }

    #[test]
    fn parse_command_defaults_and_shapes() {
        assert_eq!(
            parse_command("show", None, None).unwrap(),
            AttrCommand::Show
        );
        assert_eq!(
            parse_command("clear", None, None).unwrap(),
            AttrCommand::Clear
        );
        assert_eq!(
            parse_command("set", Some("access"), Some("write,admin")).unwrap(),
            AttrCommand::Set {
                key: "access".to_string(),
                values: vec!["write".to_string(), "admin".to_string()],
            }
        );
        assert_eq!(
            parse_command("unset", Some("dept"), None).unwrap(),
            AttrCommand::Unset {
                key: "dept".to_string(),
            }
        );
    }

    #[test]
    fn parse_command_reports_missing_and_unknown() {
        assert!(parse_command("set", None, Some("write")).is_err()); // no key
        assert!(parse_command("set", Some("access"), None).is_err()); // no values
        assert!(parse_command("unset", None, None).is_err()); // no key
        assert!(parse_command("frob", None, None).is_err()); // unknown op
        assert!(parse_command("set", Some("email"), Some("x")).is_err()); // reserved key
    }

    #[test]
    fn apply_set_replaces_unset_removes_clear_empties_show_is_noop() {
        let base = map(&[("access", &["write"]), ("dept", &["cardiology"])]);

        // Show leaves the map untouched.
        assert_eq!(apply(base.clone(), &AttrCommand::Show), base);

        // Set replaces one key's whole value list, leaving others alone.
        let set = apply(
            base.clone(),
            &AttrCommand::Set {
                key: "access".to_string(),
                values: vec!["admin".to_string()],
            },
        );
        assert_eq!(set["access"], vec!["admin"]);
        assert_eq!(set["dept"], vec!["cardiology"]);

        // Set can also insert a brand-new key.
        let added = apply(
            base.clone(),
            &AttrCommand::Set {
                key: "svc".to_string(),
                values: vec!["true".to_string()],
            },
        );
        assert_eq!(added["svc"], vec!["true"]);
        assert_eq!(added.len(), 3);

        // Unset removes one key; unsetting an absent key is a no-op.
        let unset = apply(
            base.clone(),
            &AttrCommand::Unset {
                key: "dept".to_string(),
            },
        );
        assert!(!unset.contains_key("dept"));
        assert!(unset.contains_key("access"));
        assert_eq!(
            apply(
                base.clone(),
                &AttrCommand::Unset {
                    key: "missing".to_string()
                }
            ),
            base
        );

        // Clear empties the map.
        assert!(apply(base, &AttrCommand::Clear).is_empty());
    }
}
