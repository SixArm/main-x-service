//! Attribute-based access control (ABAC) — the family's shared
//! authorization engine.
//!
//! Implements the policy model fixed by
//! [`agents/share/authorization-attributes.md`] (§2–§5): a decision is a
//! pure evaluation of an ordered rule list over the **subject
//! attributes** carried in the verified [`Claims::attrs`] map, the
//! derived **action** ([`Action`]), the coarse **resource** (the entity
//! name), and — for services that load the target record or supply
//! request context — optional **record-level resource attributes**
//! (`resource.*`) and **environment attributes** (`env.*`) matched by
//! prefixed `when` keys (§9–§10; see [`Policy::evaluate_with_context`]).
//! A `when` value of `$sub` / `$email` is a template resolving to the
//! caller's identity, so a rule can express ownership. First match wins;
//! when no rule matches, the default decision is **allow read, deny
//! everything else**. An allow rule may attach **obligations** (e.g.
//! `"mask"`, `"audit"`) that the [`Decision`] carries for the
//! enforcement point to honour — the engine does not interpret them.
//!
//! The engine is pure data + pure evaluation: no I/O, no clock, no
//! panics on any input. Nine entity services embed it inside their
//! blanket `/api/*` guards so the family shares one tested
//! implementation instead of nine copies.
//!
//! # Example
//!
//! ```
//! use authentication_verifier::{Action, Claims, Policy};
//! use std::collections::BTreeMap;
//!
//! // Subject attributes as minted by the auth-service into the token.
//! let mut attrs = BTreeMap::new();
//! attrs.insert("access".to_string(), vec!["write".to_string()]);
//! let claims = Claims {
//!     sub: "11111111-1111-1111-1111-111111111111".to_string(),
//!     email: "alice@example.com".to_string(),
//!     name: "Alice".to_string(),
//!     iss: "authentication-service".to_string(),
//!     aud: "main-x-service".to_string(),
//!     exp: 2_000_000_000,
//!     iat: 1_900_000_000,
//!     nbf: None,
//!     sid: "22222222-2222-2222-2222-222222222222".to_string(),
//!     scope: vec![],
//!     roles: vec![],
//!     attrs,
//! };
//!
//! let policy = Policy::default_policy();
//! assert!(policy.evaluate(&claims, Action::Read, "place").allowed);
//! assert!(policy.evaluate(&claims, Action::Write, "place").allowed);
//! assert!(!policy.evaluate(&claims, Action::Delete, "place").allowed);
//! ```
//!
//! [`agents/share/authorization-attributes.md`]:
//!     https://github.com/sixarm/main-x-service

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Claims;

/// The derived action attribute of a request, fixed family-wide.
///
/// Each service's guard derives one `Action` per request from the HTTP
/// method plus its documented destructive named POSTs:
///
/// | Action | Derivation |
/// |---|---|
/// | `Read` | GET / HEAD / OPTIONS |
/// | `Write` | POST / PUT / PATCH — **except** destructive named POSTs |
/// | `Delete` | DELETE |
/// | `Destructive` | the crate's destructive named POSTs (record merge, batch deduplicate, bulk import) |
///
/// **`Delete` implies `Destructive` for rule matching**: a rule listing
/// the `destructive` action matches both `Delete` and `Destructive`
/// requests, while a rule listing `delete` matches only `Delete`.
/// Destructive named POSTs are deliberately *not* `Write`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// GET / HEAD / OPTIONS — default-allowed when no rule matches.
    Read,
    /// POST / PUT / PATCH, excluding destructive named POSTs.
    Write,
    /// DELETE. Matched by both `delete` and `destructive` rule actions.
    Delete,
    /// A destructive named POST (merge / deduplicate / import).
    Destructive,
}

/// One entry of a rule's `actions` list: a concrete [`Action`] name or
/// the `"*"` wildcard covering every action.
///
/// Deserializes from the lowercase action names (`"read"`, `"write"`,
/// `"delete"`, `"destructive"`) or `"*"`. The `Destructive` pattern
/// matches both [`Action::Delete`] and [`Action::Destructive`] (delete
/// implies destructive); every other named pattern matches exactly its
/// own action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionPattern {
    /// Matches [`Action::Read`] only.
    Read,
    /// Matches [`Action::Write`] only.
    Write,
    /// Matches [`Action::Delete`] only.
    Delete,
    /// Matches [`Action::Delete`] and [`Action::Destructive`].
    Destructive,
    /// `"*"` — matches every action.
    #[serde(rename = "*")]
    Any,
}

impl ActionPattern {
    /// Whether this pattern covers the given derived action.
    #[must_use]
    pub fn matches(self, action: Action) -> bool {
        match self {
            Self::Any => true,
            Self::Read => action == Action::Read,
            Self::Write => action == Action::Write,
            Self::Delete => action == Action::Delete,
            Self::Destructive => matches!(action, Action::Delete | Action::Destructive),
        }
    }
}

/// A rule's effect: grant or refuse the covered actions.
///
/// Deny rules make exceptions expressible; combined with
/// first-match-wins ordering (see [`Policy::evaluate`]) evaluation stays
/// O(rules) and auditable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// The matching request is allowed.
    Allow,
    /// The matching request is denied (surfaced as HTTP 403 + reason).
    Deny,
}

/// One ordered policy rule: an [`Effect`], the actions it covers, and a
/// conjunction of subject-attribute conditions.
///
/// Deserializes from the JSON shape fixed in
/// `authorization-attributes.md` §4, e.g.
/// `{ "effect": "allow", "actions": ["write"], "when": { "access": ["write", "admin"] } }`.
/// Unknown JSON fields are **ignored** (forward-compatible: a newer
/// policy vocabulary still parses here; unrecognised extensions are
/// inert rather than a boot failure).
///
/// `when` semantics (§4):
///
/// - The map is a **conjunction**: every listed key must match.
/// - A value list means the subject has **any** of these values
///   (`["write", "admin"]` = write OR admin). An **empty** value list
///   never matches.
/// - A `!`-prefixed value negates: it matches when the subject does
///   **not** have that value (including when the subject lacks the
///   attribute entirely).
/// - An empty `when` map matches every authenticated subject.
/// - Keys resolve against [`Claims::attrs`], except the reserved
///   pseudo-attributes `sub` and `email` (from the verified claims) and
///   `entity` (the resource entity passed to [`Policy::evaluate`]),
///   which always resolve from those sources and cannot be shadowed by
///   an identically-named `attrs` entry.
/// - A key prefixed **`resource.`** resolves against the **resource
///   attributes** passed to [`Policy::evaluate_with_resource`] (record-
///   level attributes, e.g. `resource.sensitivity`), and a key prefixed
///   **`env.`** against the **environment attributes** passed to
///   [`Policy::evaluate_with_context`] (request-time / network context,
///   e.g. `env.hour`), each with the prefix stripped — so a deployment
///   can gate on properties of the specific record or the request
///   context. Under a call that does not supply them every such key
///   resolves empty, so the rule never matches a positive value (and a
///   `!`-negated value always matches). Both namespaces are disjoint
///   from subject attributes, so a subject can never spoof either
///   through its token.
/// - A `when` **value** of `$sub` or `$email` is a **template**: it
///   resolves to the caller's `sub` / `email` before comparison, so a
///   rule can compare an attribute to the caller's own identity — e.g.
///   `{ "resource.owner": ["$sub"] }` matches when the record's owner
///   is the caller (the ownership pattern). Any other value (including
///   one merely containing `$`) is a literal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Whether a match allows or denies the request.
    pub effect: Effect,
    /// The derived actions this rule covers (`"*"` = all).
    pub actions: Vec<ActionPattern>,
    /// Conjunction over subject attributes; empty matches everyone.
    #[serde(default)]
    pub when: BTreeMap<String, Vec<String>>,
    /// **Obligations** the enforcement point must honour when this rule
    /// **allows** — advisory instructions the engine carries but does
    /// not interpret (e.g. `"mask"` ⇒ return the masked view,
    /// `"audit"` ⇒ write an audit record). Short lowercase tokens, like
    /// attribute values. Empty by default; ignored on a `deny` rule
    /// (a denial is a 403, not a conditional allow). Surfaced on the
    /// [`Decision`] of the deciding rule so the caller can act on them.
    #[serde(default)]
    pub obligations: Vec<String>,
}

impl Rule {
    /// Whether this rule matches the given subject, action, entity,
    /// resource attributes, and environment attributes.
    #[must_use]
    fn matches(
        &self,
        claims: &Claims,
        action: Action,
        entity: &str,
        resource: &BTreeMap<String, Vec<String>>,
        env: &BTreeMap<String, Vec<String>>,
    ) -> bool {
        if !self.actions.iter().any(|pattern| pattern.matches(action)) {
            return false;
        }
        self.when.iter().all(|(key, wanted)| {
            let have = values_for(claims, entity, resource, env, key);
            wanted.iter().any(|want| {
                let (negate, raw) = match want.strip_prefix('!') {
                    Some(rest) => (true, rest),
                    None => (false, want.as_str()),
                };
                // Resolve a `$sub` / `$email` template on the wanted
                // value against the subject before comparing, so a rule
                // can compare an attribute to the caller's identity
                // (e.g. `resource.owner: ["$sub"]` = owned by the caller).
                let resolved = resolve_template(claims, raw);
                have.contains(&resolved) != negate
            })
        })
    }
}

/// Resolve a `when` **value** template against the subject: `$sub` →
/// the caller's `sub`, `$email` → the caller's `email`; any other value
/// (including one that merely contains `$`) is returned unchanged. This
/// lets a rule compare an attribute to the caller's own identity rather
/// than to a fixed literal — the ownership pattern
/// (`authorization-attributes.md` §4).
fn resolve_template<'a>(claims: &'a Claims, want: &'a str) -> &'a str {
    match want {
        "$sub" => claims.sub.as_str(),
        "$email" => claims.email.as_str(),
        other => other,
    }
}

/// The values for one `when` key. A `resource.<name>` key resolves from
/// the request's **resource attributes** (record-level) and an
/// `env.<name>` key from the **environment attributes** (request-time /
/// network / …), each with the prefix stripped; every other key
/// resolves the **subject** side — the reserved pseudo-attributes
/// `sub` / `email` / `entity` first, then the token's [`Claims::attrs`]
/// map, else empty. The `resource.` / `env.` namespaces are disjoint
/// from subject attributes, so a subject can never spoof either through
/// its token.
fn values_for<'a>(
    claims: &'a Claims,
    entity: &'a str,
    resource: &'a BTreeMap<String, Vec<String>>,
    env: &'a BTreeMap<String, Vec<String>>,
    key: &str,
) -> Vec<&'a str> {
    if let Some(name) = key.strip_prefix("resource.") {
        return resource
            .get(name)
            .map(|values| values.iter().map(String::as_str).collect())
            .unwrap_or_default();
    }
    if let Some(name) = key.strip_prefix("env.") {
        return env
            .get(name)
            .map(|values| values.iter().map(String::as_str).collect())
            .unwrap_or_default();
    }
    match key {
        "sub" => vec![claims.sub.as_str()],
        "email" => vec![claims.email.as_str()],
        "entity" => vec![entity],
        _ => claims
            .attrs
            .get(key)
            .map(|values| values.iter().map(String::as_str).collect())
            .unwrap_or_default(),
    }
}

/// An ordered list of [`Rule`]s, evaluated top-down with
/// **first match wins**; when nothing matches, the default decision is
/// allow-read / deny-everything-else (`authorization-attributes.md` §5).
///
/// Pure data: load once at boot (from `<ENTITY>_ABAC_POLICY` /
/// `<ENTITY>_ABAC_POLICY_FILE` via [`Policy::from_json`], falling back
/// to [`Policy::default_policy`] on parse failure), then call
/// [`Policy::evaluate`] per request. Unknown JSON fields are ignored
/// (see [`Rule`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// The ordered rules; index order is evaluation order.
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Parse a policy from its JSON document form
    /// (`{ "rules": [ { "effect": ..., "actions": [...], "when": {...} }, ... ] }`).
    ///
    /// # Errors
    ///
    /// Returns the underlying [`serde_json::Error`] when the document is
    /// not valid JSON or does not fit the policy shape (unknown `effect`
    /// / action names, non-list `when` values, ...). Callers fall back
    /// to [`Policy::default_policy`] — a bad policy must never take a
    /// service down at boot.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The built-in default policy (`authorization-attributes.md` §5),
    /// used when no policy is configured:
    ///
    /// 1. `svc=true` ⇒ allow `write` + `destructive` (machine peers may
    ///    do everything; `read` is default-allowed anyway),
    /// 2. `access=admin` ⇒ allow `destructive` (which covers `delete`),
    /// 3. `access=write` or `access=admin` ⇒ allow `write`.
    ///
    /// Everything else falls through to the default decision: read
    /// allowed, mutation denied.
    #[must_use]
    pub fn default_policy() -> Self {
        let when = |key: &str, values: &[&str]| {
            let mut map = BTreeMap::new();
            map.insert(
                key.to_string(),
                values.iter().map(ToString::to_string).collect(),
            );
            map
        };
        Self {
            rules: vec![
                Rule {
                    effect: Effect::Allow,
                    actions: vec![ActionPattern::Write, ActionPattern::Destructive],
                    when: when("svc", &["true"]),
                    obligations: Vec::new(),
                },
                Rule {
                    effect: Effect::Allow,
                    actions: vec![ActionPattern::Destructive],
                    when: when("access", &["admin"]),
                    obligations: Vec::new(),
                },
                Rule {
                    effect: Effect::Allow,
                    actions: vec![ActionPattern::Write],
                    when: when("access", &["write", "admin"]),
                    obligations: Vec::new(),
                },
            ],
        }
    }

    /// Evaluate the policy for one request: the verified subject
    /// ([`Claims`], whose [`attrs`](Claims::attrs) carry the subject
    /// attributes), the derived [`Action`], and the resource `entity`
    /// (the crate's entity type, e.g. `"place"`; reserved for resource
    /// rules — a `when` key `entity` matches against it).
    ///
    /// Considers **no record-level resource attributes** — every
    /// `resource.*` `when` key resolves empty. This is the coarse,
    /// no-record-load path the blanket guards use; a service that has
    /// loaded the target record calls [`Policy::evaluate_with_resource`]
    /// instead (handler-level record checks).
    ///
    /// Rules are checked top-down; the **first** matching rule decides.
    /// When no rule matches, the default decision applies: [`Action::Read`]
    /// is allowed, every other action is denied.
    ///
    /// Pure and total: no I/O, and no input — claims, attrs, policy, or
    /// entity — can make it panic.
    #[must_use]
    pub fn evaluate(&self, claims: &Claims, action: Action, entity: &str) -> Decision {
        self.evaluate_with_resource(claims, action, entity, &BTreeMap::new())
    }

    /// Evaluate the policy with **record-level resource attributes** —
    /// a string→strings map describing the specific target record (e.g.
    /// `{"sensitivity": ["high"]}`), matched by `when` keys prefixed
    /// `resource.` (`resource.sensitivity`). Otherwise identical to
    /// [`Policy::evaluate`]: same subject attributes, same action, same
    /// entity, same first-match-wins ordering and default decision.
    ///
    /// A service derives the resource attributes from the record it just
    /// loaded (handler-level, after fetch) and passes them here, so a
    /// deployment can express e.g. "deny write on a high-sensitivity
    /// record unless `access=admin`" as ordered policy rules
    /// (`authorization-attributes.md` §9). The `resource.` namespace is
    /// disjoint from subject attributes, so a caller can never spoof a
    /// resource attribute through its token. Considers **no environment
    /// attributes** (every `env.*` key resolves empty) — call
    /// [`Policy::evaluate_with_context`] to pass those too.
    ///
    /// Pure and total: no I/O, no panics on any input.
    #[must_use]
    pub fn evaluate_with_resource(
        &self,
        claims: &Claims,
        action: Action,
        entity: &str,
        resource: &BTreeMap<String, Vec<String>>,
    ) -> Decision {
        self.evaluate_with_context(claims, action, entity, resource, &BTreeMap::new())
    }

    /// Evaluate the policy with both **record-level resource attributes**
    /// (matched by `resource.*` `when` keys) and **environment
    /// attributes** (matched by `env.*` keys) — a string→strings map of
    /// request-time / network context, e.g.
    /// `{"hour": ["22"], "network": ["office"]}`. Otherwise identical to
    /// [`Policy::evaluate`].
    ///
    /// A service derives the environment attributes per request (clock,
    /// source IP, …) and passes them here so a deployment can express
    /// e.g. "deny write outside working hours" or geofencing as ordered
    /// policy rules (`authorization-attributes.md` §10). Like
    /// `resource.*`, the `env.*` namespace is disjoint from subject
    /// attributes, so a caller cannot spoof it through its token.
    ///
    /// Pure and total: no I/O (the caller supplies the clock/network,
    /// keeping the engine deterministic), no panics on any input.
    #[must_use]
    pub fn evaluate_with_context(
        &self,
        claims: &Claims,
        action: Action,
        entity: &str,
        resource: &BTreeMap<String, Vec<String>>,
        env: &BTreeMap<String, Vec<String>>,
    ) -> Decision {
        for (index, rule) in self.rules.iter().enumerate() {
            if rule.matches(claims, action, entity, resource, env) {
                return match rule.effect {
                    // An allow carries the rule's obligations for the
                    // enforcement point to honour (e.g. "mask"); a deny
                    // carries none (it is a 403, not a conditional allow).
                    Effect::Allow => Decision {
                        allowed: true,
                        reason: format!("allow (rule {index})"),
                        obligations: rule.obligations.clone(),
                    },
                    Effect::Deny => Decision {
                        allowed: false,
                        reason: format!("deny (rule {index})"),
                        obligations: Vec::new(),
                    },
                };
            }
        }
        if action == Action::Read {
            Decision {
                allowed: true,
                reason: "default allow (read)".to_string(),
                obligations: Vec::new(),
            }
        } else {
            Decision {
                allowed: false,
                reason: "default deny".to_string(),
                obligations: Vec::new(),
            }
        }
    }
}

/// The outcome of one [`Policy::evaluate`] call.
///
/// `reason` names the deciding rule by index (`"allow (rule 0)"` /
/// `"deny (rule 2)"`) or the default decision (`"default allow (read)"`
/// / `"default deny"`), so a 403 response body and the audit trail can
/// state exactly why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Whether the request is allowed.
    pub allowed: bool,
    /// The deciding rule index or the default decision.
    pub reason: String,
    /// **Obligations** the enforcement point must honour on an allow —
    /// the deciding allow rule's [`Rule::obligations`] (e.g. `"mask"`,
    /// `"audit"`). Empty on a deny or the default decision. Advisory: the
    /// engine carries them; the caller (PEP) interprets and acts on them
    /// (e.g. `"mask"` ⇒ return the masked view). `#[serde(default)]` so
    /// a decision serialized by an older peer still deserializes.
    #[serde(default)]
    pub obligations: Vec<String>,
}

impl Decision {
    /// Whether the decision carries the named obligation (a convenience
    /// for the enforcement point, e.g. `decision.requires("mask")`).
    #[must_use]
    pub fn requires(&self, obligation: &str) -> bool {
        self.obligations.iter().any(|o| o == obligation)
    }
}

/// A **hot-reloadable** [`Policy`] holder for the enforcement point: the
/// active policy can be swapped at runtime (e.g. when the policy file
/// changes) **without a restart**, while readers keep the coarse
/// `evaluate` path lock-light.
///
/// It wraps an `Arc<Policy>` behind an `RwLock`. Per request the guard
/// calls [`current`](Self::current) — a brief read-lock returning a
/// cheap `Arc` clone it then evaluates against; a reload calls
/// [`store`](Self::store) — a brief write-lock swapping the `Arc`. A
/// request in flight during a reload finishes against the snapshot it
/// took, so a swap is never observed mid-evaluation. Poison-safe: a
/// panic elsewhere never makes `current`/`store` panic (verifier rule:
/// no panics in the API).
///
/// The **trigger** (a file-mtime watch, a signal, an admin endpoint) is
/// the service's concern — this type only holds and swaps the value.
#[derive(Debug)]
pub struct ReloadablePolicy {
    inner: std::sync::RwLock<std::sync::Arc<Policy>>,
}

impl ReloadablePolicy {
    /// Wrap an initial policy (e.g. the one loaded at boot).
    #[must_use]
    pub fn new(policy: Policy) -> Self {
        Self {
            inner: std::sync::RwLock::new(std::sync::Arc::new(policy)),
        }
    }

    /// The currently active policy — a cheap `Arc` clone taken under a
    /// brief read-lock. Evaluate against the returned snapshot; a
    /// concurrent [`store`](Self::store) does not affect it.
    #[must_use]
    pub fn current(&self) -> std::sync::Arc<Policy> {
        self.inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Atomically replace the active policy (a brief write-lock). New
    /// requests see the new policy; in-flight requests finish against
    /// the snapshot they already took.
    pub fn store(&self, policy: Policy) {
        *self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = std::sync::Arc::new(policy);
    }
}

/// Engine unit tests — pure, offline, per
/// `authorization-attributes.md` §7.
#[cfg(test)]
mod tests {
    use super::*;

    /// Claims with the given subject attributes; everything else fixed.
    fn claims_with_attrs(attrs: &[(&str, &[&str])]) -> Claims {
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            iss: "authentication-service".to_string(),
            aud: "main-x-service".to_string(),
            exp: 2_000_000_000,
            iat: 1_900_000_000,
            nbf: None,
            sid: "22222222-2222-2222-2222-222222222222".to_string(),
            scope: vec![],
            roles: vec![],
            attrs: attrs
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).to_string(),
                        values.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn empty_attrs_default_policy_allows_read_denies_everything_else() {
        let policy = Policy::default_policy();
        let claims = claims_with_attrs(&[]);
        let read = policy.evaluate(&claims, Action::Read, "place");
        assert!(read.allowed);
        assert_eq!(read.reason, "default allow (read)");
        for action in [Action::Write, Action::Delete, Action::Destructive] {
            let decision = policy.evaluate(&claims, action, "place");
            assert!(!decision.allowed, "{action:?} must default-deny");
            assert_eq!(decision.reason, "default deny");
        }
    }

    #[test]
    fn access_write_allows_write_but_not_delete_or_destructive() {
        let policy = Policy::default_policy();
        let claims = claims_with_attrs(&[("access", &["write"])]);
        assert!(policy.evaluate(&claims, Action::Read, "place").allowed);
        let write = policy.evaluate(&claims, Action::Write, "place");
        assert!(write.allowed);
        assert_eq!(write.reason, "allow (rule 2)");
        assert!(!policy.evaluate(&claims, Action::Delete, "place").allowed);
        assert!(
            !policy
                .evaluate(&claims, Action::Destructive, "place")
                .allowed
        );
    }

    #[test]
    fn access_admin_allows_destructive_and_delete_and_write() {
        let policy = Policy::default_policy();
        let claims = claims_with_attrs(&[("access", &["admin"])]);
        let destructive = policy.evaluate(&claims, Action::Destructive, "place");
        assert!(destructive.allowed);
        assert_eq!(destructive.reason, "allow (rule 1)");
        // Delete implies destructive: the `destructive` rule covers it.
        assert!(policy.evaluate(&claims, Action::Delete, "place").allowed);
        assert!(policy.evaluate(&claims, Action::Write, "place").allowed);
        assert!(policy.evaluate(&claims, Action::Read, "place").allowed);
    }

    #[test]
    fn svc_true_allows_everything() {
        let policy = Policy::default_policy();
        let claims = claims_with_attrs(&[("svc", &["true"])]);
        for action in [
            Action::Read,
            Action::Write,
            Action::Delete,
            Action::Destructive,
        ] {
            assert!(
                policy.evaluate(&claims, action, "place").allowed,
                "{action:?} must be allowed for svc=true"
            );
        }
    }

    #[test]
    fn first_match_wins_deny_before_allow() {
        // A deny rule ahead of a matching allow rule must pin the deny.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "dept": ["cardiology"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let denied = claims_with_attrs(&[("access", &["write"]), ("dept", &["cardiology"])]);
        let decision = policy.evaluate(&denied, Action::Write, "case");
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "deny (rule 0)");
        // Without the denied attribute, the later allow rule matches.
        let allowed = claims_with_attrs(&[("access", &["write"])]);
        let decision = policy.evaluate(&allowed, Action::Write, "case");
        assert!(decision.allowed);
        assert_eq!(decision.reason, "allow (rule 1)");
    }

    #[test]
    fn negated_value_matches_absence_of_the_value() {
        // §4 example: deny read outside cardiology.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny", "actions": ["read"], "when": { "dept": ["!cardiology"] } }
            ] }"#,
        )
        .expect("policy parses");
        // Has another dept ⇒ "does not have cardiology" ⇒ deny matches.
        let outsider = claims_with_attrs(&[("dept", &["oncology"])]);
        assert!(!policy.evaluate(&outsider, Action::Read, "case").allowed);
        // Lacks the attribute entirely ⇒ also does not have it ⇒ deny.
        let no_dept = claims_with_attrs(&[]);
        assert!(!policy.evaluate(&no_dept, Action::Read, "case").allowed);
        // Has cardiology ⇒ the negation does not match ⇒ default allow read.
        let insider = claims_with_attrs(&[("dept", &["cardiology"])]);
        assert!(policy.evaluate(&insider, Action::Read, "case").allowed);
    }

    #[test]
    fn wildcard_actions_cover_every_action() {
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["*"], "when": { "svc": ["true"] } }
            ] }"#,
        )
        .expect("policy parses");
        let claims = claims_with_attrs(&[("svc", &["true"])]);
        for action in [
            Action::Read,
            Action::Write,
            Action::Delete,
            Action::Destructive,
        ] {
            let decision = policy.evaluate(&claims, action, "thing");
            assert!(decision.allowed);
            assert_eq!(decision.reason, "allow (rule 0)");
        }
    }

    #[test]
    fn value_list_means_any_of_these_values() {
        // §4: ["write", "admin"] = write OR admin.
        let policy = Policy::default_policy();
        for tier in ["write", "admin"] {
            let claims = claims_with_attrs(&[("access", &[tier])]);
            assert!(
                policy.evaluate(&claims, Action::Write, "place").allowed,
                "access={tier} must allow write"
            );
        }
        let claims = claims_with_attrs(&[("access", &["other"])]);
        assert!(!policy.evaluate(&claims, Action::Write, "place").allowed);
    }

    #[test]
    fn empty_when_matches_every_subject_and_empty_value_list_matches_none() {
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"] },
                { "effect": "allow", "actions": ["delete"], "when": { "access": [] } }
            ] }"#,
        )
        .expect("policy parses");
        let claims = claims_with_attrs(&[("access", &["admin"])]);
        // Rule 0 has no `when` ⇒ matches everyone.
        assert!(policy.evaluate(&claims, Action::Write, "event").allowed);
        // Rule 1's empty value list matches nothing ⇒ default deny.
        assert!(!policy.evaluate(&claims, Action::Delete, "event").allowed);
    }

    #[test]
    fn pseudo_attributes_sub_email_and_entity_are_matchable() {
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"],
                  "when": { "sub": ["11111111-1111-1111-1111-111111111111"],
                            "email": ["alice@example.com"],
                            "entity": ["place"] } }
            ] }"#,
        )
        .expect("policy parses");
        let claims = claims_with_attrs(&[]);
        assert!(policy.evaluate(&claims, Action::Write, "place").allowed);
        // Same subject, different entity ⇒ the rule no longer matches.
        assert!(!policy.evaluate(&claims, Action::Write, "person").allowed);
    }

    #[test]
    fn pseudo_attributes_cannot_be_shadowed_by_attrs() {
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "sub": ["spoofed"] } }
            ] }"#,
        )
        .expect("policy parses");
        // An `attrs` entry named "sub" must not override the claims `sub`.
        let claims = claims_with_attrs(&[("sub", &["spoofed"])]);
        assert!(!policy.evaluate(&claims, Action::Write, "place").allowed);
    }

    #[test]
    fn bad_policy_json_is_an_error_not_a_panic() {
        assert!(Policy::from_json("not json at all").is_err());
        assert!(Policy::from_json(r#"{ "rules": "not-a-list" }"#).is_err());
        assert!(
            Policy::from_json(r#"{ "rules": [ { "effect": "shrug", "actions": ["read"] } ] }"#)
                .is_err()
        );
        assert!(
            Policy::from_json(r#"{ "rules": [ { "effect": "allow", "actions": ["frob"] } ] }"#)
                .is_err()
        );
    }

    #[test]
    fn unknown_rule_fields_are_ignored() {
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "svc": ["true"] },
                  "comment": "future field", "advice": ["someday"] }
            ] }"#,
        )
        .expect("unknown fields must be ignored");
        let claims = claims_with_attrs(&[("svc", &["true"])]);
        assert!(policy.evaluate(&claims, Action::Write, "worker").allowed);
    }

    #[test]
    fn allow_rule_carries_its_obligations_to_the_decision() {
        // A rule can attach obligations (e.g. "mask") the enforcement
        // point must honour on an allow; a deny / default carries none.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["read"],
                  "when": { "access": ["read-masked"] },
                  "obligations": ["mask", "audit"] },
                { "effect": "deny", "actions": ["read"],
                  "when": { "resource.sensitivity": ["secret"] },
                  "obligations": ["mask"] }
            ] }"#,
        )
        .expect("policy parses");

        // The allow rule surfaces its obligations.
        let masked = claims_with_attrs(&[("access", &["read-masked"])]);
        let decision = policy.evaluate(&masked, Action::Read, "case");
        assert!(decision.allowed);
        assert!(decision.requires("mask"));
        assert!(decision.requires("audit"));
        assert!(!decision.requires("delete"));

        // A deny carries no obligations (it's a 403, not a conditional
        // allow) even though the rule lists one.
        let onto_secret = policy.evaluate_with_resource(
            &claims_with_attrs(&[("access", &["read-masked"]), ("other", &["x"])]),
            Action::Read,
            "case",
            &resource(&[("sensitivity", &["secret"])]),
        );
        // rule 0 matches first (access=read-masked) → allow+mask; the
        // deny never reached. Confirm first-match precedence holds.
        assert!(onto_secret.allowed);
        assert!(onto_secret.requires("mask"));

        // A subject with no matching allow rule: default decision, no
        // obligations.
        let plain = policy.evaluate(&claims_with_attrs(&[]), Action::Read, "case");
        assert!(plain.allowed); // default allow-read
        assert!(plain.obligations.is_empty());
    }

    #[test]
    fn default_policy_allows_carry_no_obligations() {
        let policy = Policy::default_policy();
        let admin = claims_with_attrs(&[("access", &["admin"])]);
        let d = policy.evaluate(&admin, Action::Write, "place");
        assert!(d.allowed);
        assert!(d.obligations.is_empty());
    }

    #[test]
    fn reloadable_policy_swaps_the_active_policy() {
        // Start with a policy that denies all writes (empty rules ⇒
        // default deny-mutation), then hot-swap to one that allows a
        // writer — without recreating the holder.
        let holder = ReloadablePolicy::new(Policy { rules: vec![] });
        let writer = claims_with_attrs(&[("access", &["write"])]);
        assert!(
            !holder
                .current()
                .evaluate(&writer, Action::Write, "case")
                .allowed,
            "before reload: default deny-mutation"
        );

        holder.store(
            Policy::from_json(
                r#"{ "rules": [
                    { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
                ] }"#,
            )
            .expect("policy parses"),
        );
        assert!(
            holder
                .current()
                .evaluate(&writer, Action::Write, "case")
                .allowed,
            "after reload: the new policy allows the writer"
        );

        // A snapshot taken before a reload is unaffected by it.
        let snapshot = holder.current();
        holder.store(Policy { rules: vec![] });
        assert!(
            snapshot.evaluate(&writer, Action::Write, "case").allowed,
            "an in-flight snapshot keeps the policy it captured"
        );
        assert!(
            !holder
                .current()
                .evaluate(&writer, Action::Write, "case")
                .allowed,
            "new readers see the latest (deny) policy"
        );
    }

    #[test]
    fn default_policy_round_trips_through_its_json_form() {
        // The built-in policy is exactly the §5 subset of the §4 example.
        let from_json = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write", "destructive"], "when": { "svc": ["true"] } },
                { "effect": "allow", "actions": ["destructive"], "when": { "access": ["admin"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write", "admin"] } }
            ] }"#,
        )
        .expect("policy parses");
        assert_eq!(from_json, Policy::default_policy());
    }

    #[test]
    fn delete_pattern_does_not_match_destructive_posts() {
        // `write` excludes destructive POSTs and `delete` covers only
        // DELETE; only `destructive` (or `*`) covers a merge-POST.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["delete"], "when": { "access": ["ops"] } }
            ] }"#,
        )
        .expect("policy parses");
        let claims = claims_with_attrs(&[("access", &["ops"])]);
        assert!(policy.evaluate(&claims, Action::Delete, "case").allowed);
        assert!(
            !policy
                .evaluate(&claims, Action::Destructive, "case")
                .allowed
        );
    }

    /// Build a resource-attribute map from `(key, values)` pairs.
    fn resource(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(key, values)| {
                (
                    (*key).to_string(),
                    values.iter().map(ToString::to_string).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn resource_attribute_gates_a_deny_on_record_sensitivity() {
        // §9 motivating example: deny write on a high-sensitivity record
        // unless the subject is an admin. Order matters — the admin
        // allow sits above the sensitivity deny.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "access": ["admin"] } },
                { "effect": "deny",  "actions": ["write"], "when": { "resource.sensitivity": ["high"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");

        let writer = claims_with_attrs(&[("access", &["write"])]);
        let admin = claims_with_attrs(&[("access", &["admin"])]);
        let high = resource(&[("sensitivity", &["high"])]);
        let low = resource(&[("sensitivity", &["low"])]);

        // Writer on a high-sensitivity record → denied by rule 1.
        let d = policy.evaluate_with_resource(&writer, Action::Write, "case", &high);
        assert!(!d.allowed);
        assert_eq!(d.reason, "deny (rule 1)");
        // Writer on a low-sensitivity record → allowed by rule 2.
        assert!(
            policy
                .evaluate_with_resource(&writer, Action::Write, "case", &low)
                .allowed
        );
        // Admin on a high-sensitivity record → allowed by rule 0 (the
        // admin allow precedes the sensitivity deny).
        let a = policy.evaluate_with_resource(&admin, Action::Write, "case", &high);
        assert!(a.allowed);
        assert_eq!(a.reason, "allow (rule 0)");
    }

    #[test]
    fn resource_keys_resolve_empty_without_resource_attrs() {
        // Under plain `evaluate` (no record loaded), a `resource.*`
        // positive match never fires; the writer falls through to the
        // later allow. This is why the coarse blanket guard stays sound.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "resource.sensitivity": ["high"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let writer = claims_with_attrs(&[("access", &["write"])]);
        // No resource attrs ⇒ the deny cannot match ⇒ later allow wins.
        let d = policy.evaluate(&writer, Action::Write, "case");
        assert!(d.allowed);
        assert_eq!(d.reason, "allow (rule 1)");
        // The same call via evaluate_with_resource + an empty map is
        // identical (evaluate delegates to it).
        assert_eq!(
            d,
            policy.evaluate_with_resource(&writer, Action::Write, "case", &BTreeMap::new())
        );
    }

    #[test]
    fn negated_resource_value_matches_when_absent_or_empty() {
        // "allow read only on records the caller's dept owns": deny read
        // when the record's owning dept is NOT the caller's dept. Here we
        // pin the simpler negation semantics against a literal.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny", "actions": ["read"], "when": { "resource.sensitivity": ["!public"] } }
            ] }"#,
        )
        .expect("policy parses");
        let anyone = claims_with_attrs(&[]);
        // Record is public ⇒ negation does not match ⇒ default allow read.
        assert!(
            policy
                .evaluate_with_resource(
                    &anyone,
                    Action::Read,
                    "case",
                    &resource(&[("sensitivity", &["public"])])
                )
                .allowed
        );
        // Record is restricted ⇒ "does not have public" ⇒ deny matches.
        assert!(
            !policy
                .evaluate_with_resource(
                    &anyone,
                    Action::Read,
                    "case",
                    &resource(&[("sensitivity", &["restricted"])])
                )
                .allowed
        );
        // No resource attr at all ⇒ also "does not have public" ⇒ deny.
        assert!(
            !policy
                .evaluate_with_resource(&anyone, Action::Read, "case", &BTreeMap::new())
                .allowed
        );
    }

    #[test]
    fn resource_namespace_is_disjoint_from_subject_attrs() {
        // A subject cannot spoof a resource attribute through its token:
        // an `attrs` entry literally named "resource.sensitivity" is a
        // plain subject key, not the resource value.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "resource.sensitivity": ["high"] } }
            ] }"#,
        )
        .expect("policy parses");
        // Subject carries a same-named attr, but no resource attrs are
        // passed ⇒ the resource key resolves empty ⇒ no match ⇒ default
        // deny for write.
        let spoofer = claims_with_attrs(&[("resource.sensitivity", &["high"])]);
        assert!(!policy.evaluate(&spoofer, Action::Write, "case").allowed);
    }

    /// The subject's fixed `sub` in [`claims_with_attrs`], reused by the
    /// ownership-template test as the record's `owner`.
    const SUBJECT_PID: &str = "11111111-1111-1111-1111-111111111111";

    #[test]
    fn value_template_sub_expresses_ownership() {
        // "a writer may write a record they own": allow write when the
        // record's owner equals the caller's sub (`$sub`).
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"],
                  "when": { "access": ["write"], "resource.owner": ["$sub"] } }
            ] }"#,
        )
        .expect("policy parses");
        let writer = claims_with_attrs(&[("access", &["write"])]);

        // Owner == caller ⇒ allowed.
        assert!(
            policy
                .evaluate_with_resource(
                    &writer,
                    Action::Write,
                    "case",
                    &resource(&[("owner", &[SUBJECT_PID])])
                )
                .allowed
        );
        // Owner is someone else ⇒ the `$sub` template does not match ⇒
        // default deny for write.
        assert!(
            !policy
                .evaluate_with_resource(
                    &writer,
                    Action::Write,
                    "case",
                    &resource(&[("owner", &["99999999-9999-9999-9999-999999999999"])])
                )
                .allowed
        );
    }

    #[test]
    fn literal_dollar_value_is_not_a_template() {
        // Only exactly `$sub` / `$email` are templates; any other value
        // (incl. one containing `$`) is a literal.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "tier": ["$gold"] } }
            ] }"#,
        )
        .expect("policy parses");
        // A subject whose `tier` is literally "$gold" matches.
        let claims = claims_with_attrs(&[("tier", &["$gold"])]);
        assert!(policy.evaluate(&claims, Action::Write, "thing").allowed);
    }

    #[test]
    fn env_attribute_gates_a_time_window_deny() {
        // §10 example: deny write outside working hours (here, hour 22).
        // The service supplies the clock as an env attribute — the engine
        // stays deterministic.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "allow", "actions": ["write"], "when": { "access": ["admin"] } },
                { "effect": "deny",  "actions": ["write"], "when": { "env.after_hours": ["true"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let writer = claims_with_attrs(&[("access", &["write"])]);
        let admin = claims_with_attrs(&[("access", &["admin"])]);
        let after_hours = resource(&[("after_hours", &["true"])]);
        let during_hours = resource(&[("after_hours", &["false"])]);

        // Writer after hours ⇒ denied by rule 1.
        let denied = policy.evaluate_with_context(
            &writer,
            Action::Write,
            "case",
            &BTreeMap::new(),
            &after_hours,
        );
        assert!(!denied.allowed);
        assert_eq!(denied.reason, "deny (rule 1)");
        // Writer during hours ⇒ allowed by rule 2.
        assert!(
            policy
                .evaluate_with_context(
                    &writer,
                    Action::Write,
                    "case",
                    &BTreeMap::new(),
                    &during_hours
                )
                .allowed
        );
        // Admin overrides the after-hours deny (rule 0 precedes it).
        assert!(
            policy
                .evaluate_with_context(
                    &admin,
                    Action::Write,
                    "case",
                    &BTreeMap::new(),
                    &after_hours
                )
                .allowed
        );
    }

    #[test]
    fn env_namespace_empty_without_context() {
        // Under evaluate / evaluate_with_resource (no env supplied), an
        // `env.*` positive match never fires; the later allow wins.
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "env.after_hours": ["true"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let writer = claims_with_attrs(&[("access", &["write"])]);
        let d = policy.evaluate(&writer, Action::Write, "case");
        assert!(d.allowed);
        assert_eq!(d.reason, "allow (rule 1)");
        // evaluate_with_resource (resource but no env) is identical.
        assert_eq!(
            d,
            policy.evaluate_with_resource(&writer, Action::Write, "case", &BTreeMap::new())
        );
    }
}
