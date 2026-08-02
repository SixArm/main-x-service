//! Admin HTTP surface — **ABAC attribute assignment over HTTP**.
//!
//! The operator counterpart to the `user_attributes` CLI task (per
//! [`agents/share/authorization-attributes.md`] §6): assign / inspect a
//! user's `users.attributes` (the map minted into the PASETO `attrs`
//! claim) from an authenticated admin caller, for when an operator UI
//! needs it rather than shell/DB access.
//!
//! | Method | Path | Purpose |
//! |---|---|---|
//! | `GET` | `/api/auth/admin/users/{pid}/attributes` | The user's current attribute map. |
//! | `PUT` | `/api/auth/admin/users/{pid}/attributes` | **Replace** the whole attribute map (body `{ "attributes": { "<key>": ["<value>", …] } }`). |
//!
//! **Authorization.** The caller presents a valid PASETO bearer
//! ([`AuthUser`]) whose own attributes include `access=admin` — the
//! bootstrap admin is assigned via the CLI task (DB access only), after
//! which admins manage others over HTTP. A missing/invalid token is
//! `401` (the extractor); a valid non-admin token is `403`. Revocation
//! honours the family's short-token window (~5 min), same as the peer
//! services — this endpoint trusts the offline-verifiable token, it does
//! not re-check the session.
//!
//! **Validation.** Keys and values are validated exactly as the CLI task
//! validates them (short lowercase tokens; the reserved pseudo-attributes
//! `sub`/`email`/`entity` refused; no empty value lists) — a bad body is
//! `422`. Every successful change writes an `attributes_assigned`
//! `auth_events` audit row whose actor is the admin's `pid:<uuid>`.
//!
//! [`agents/share/authorization-attributes.md`]:
//!     ../../../agents/share/authorization-attributes.md

use std::collections::BTreeMap;

use axum::extract::Path;
use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthUser, Claims},
    models::{auth_events::Model as AuthEvent, sessions, users},
    tasks::attributes::{validate_key, validate_value},
};

/// Response body for both endpoints: the user's public id, email, and
/// current ABAC attribute map.
#[derive(Debug, Serialize)]
struct AttributesResponse {
    /// The user's public id (`pid`).
    pid: String,
    /// The user's email.
    email: String,
    /// The current ABAC subject-attribute map.
    attributes: BTreeMap<String, Vec<String>>,
}

impl AttributesResponse {
    fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            email: user.email.clone(),
            attributes: user.attrs(),
        }
    }
}

/// Request body for `PUT …/attributes`: the full replacement map.
#[derive(Debug, Deserialize)]
struct ReplaceAttributesBody {
    /// The complete attribute map to store (replaces the existing one).
    attributes: BTreeMap<String, Vec<String>>,
}

/// `403 Forbidden` unless the caller carries `access=admin`.
fn require_admin(claims: &Claims) -> Result<()> {
    let is_admin = claims
        .attrs
        .get("access")
        .is_some_and(|values| values.iter().any(|v| v == "admin"));
    if is_admin {
        Ok(())
    } else {
        Err(Error::CustomError(
            StatusCode::FORBIDDEN,
            ErrorDetail::new(
                "forbidden",
                "admin attribute required (access=admin) to assign attributes",
            ),
        ))
    }
}

/// Map a validation message to a `422 Unprocessable Entity`.
fn unprocessable(message: impl Into<String>) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable_entity", message.into()),
    )
}

/// Validate a submitted attribute map with the same rules as the CLI
/// task: every key is a short lowercase token and not a reserved
/// pseudo-attribute; every value is a short lowercase token; no key may
/// map to an empty value list (omit the key to remove it).
fn validate_map(map: &BTreeMap<String, Vec<String>>) -> Result<()> {
    for (key, values) in map {
        validate_key(key).map_err(unprocessable)?;
        if values.is_empty() {
            return Err(unprocessable(format!(
                "attribute `{key}` has no values; omit the key to remove it"
            )));
        }
        for value in values {
            validate_value(value).map_err(unprocessable)?;
        }
        // Enforce the configured attribute vocabulary (catches typos).
        crate::tasks::attributes::vocabulary()
            .check(key, values)
            .map_err(unprocessable)?;
    }
    Ok(())
}

/// `GET /api/auth/admin/users/{pid}/attributes` — the user's current
/// ABAC attribute map. `403` unless the caller is an admin; `404` for an
/// unknown or GDPR-erased user.
#[debug_handler]
async fn show_attributes(
    auth: AuthUser,
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let AuthUser(claims) = auth;
    require_admin(&claims)?;
    let user = users::Model::find_active_by_pid(&ctx.db, &pid)
        .await
        .map_err(|_| Error::NotFound)?;
    format::json(AttributesResponse::new(&user))
}

/// `PUT /api/auth/admin/users/{pid}/attributes` — **replace** the user's
/// whole ABAC attribute map. `403` unless the caller is an admin; `422`
/// on an invalid body; `404` for an unknown or GDPR-erased user. Writes
/// an `attributes_assigned` audit row (actor = the admin's pid).
#[debug_handler]
async fn replace_attributes(
    auth: AuthUser,
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    Json(body): Json<ReplaceAttributesBody>,
) -> Result<Response> {
    let AuthUser(claims) = auth;
    require_admin(&claims)?;
    validate_map(&body.attributes)?;

    let user = users::Model::find_active_by_pid(&ctx.db, &pid)
        .await
        .map_err(|_| Error::NotFound)?;
    let target_pid = user.pid;
    let target_email = user.email.clone();
    let updated = user
        .into_active_model()
        .set_attributes(&ctx.db, users::attributes_to_value(&body.attributes))
        .await?;
    // SEC-A8: a live session snapshotted the OLD attributes (per
    // authorization-attributes.md §6, attrs are copied into the session at
    // establishment and minted into the token from there), so it would keep
    // issuing tokens with the stale attributes until its absolute TTL.
    // Revoke the user's sessions so the change takes effect now — the next
    // login copies the new attributes into a fresh session.
    sessions::Model::revoke_all_for_user(&ctx.db, target_pid).await?;

    AuthEvent::record_attribute_assignment_best_effort(
        &ctx.db,
        Some(&target_email),
        target_pid,
        "replace",
        None,
        &format!("pid:{}", claims.sub),
    )
    .await;
    tracing::info!(target_pid = %target_pid, actor_pid = %claims.sub, "admin replaced user ABAC attributes");
    format::json(AttributesResponse::new(&updated))
}

/// Routes for the admin surface, mounted under `/api/auth/admin`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth/admin")
        .add("/users/{pid}/attributes", get(show_attributes))
        .add("/users/{pid}/attributes", put(replace_attributes))
}

#[cfg(test)]
mod tests {
    use super::{require_admin, validate_map};
    use crate::auth::Claims;
    use std::collections::BTreeMap;

    fn claims_with_access(values: &[&str]) -> Claims {
        let mut attrs = BTreeMap::new();
        if !values.is_empty() {
            attrs.insert(
                "access".to_string(),
                values.iter().map(ToString::to_string).collect(),
            );
        }
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "admin@example.com".to_string(),
            name: "Admin".to_string(),
            iss: "authentication-service".to_string(),
            aud: "main-x-service".to_string(),
            exp: 2_000_000_000,
            iat: 1_900_000_000,
            nbf: None,
            sid: "22222222-2222-2222-2222-222222222222".to_string(),
            scope: vec![],
            roles: vec![],
            attrs,
        }
    }

    #[test]
    fn require_admin_gates_on_access_admin() {
        assert!(require_admin(&claims_with_access(&["admin"])).is_ok());
        // write tier is not admin
        assert!(require_admin(&claims_with_access(&["write"])).is_err());
        // no access attr at all
        assert!(require_admin(&claims_with_access(&[])).is_err());
        // admin among several values still counts
        assert!(require_admin(&claims_with_access(&["write", "admin"])).is_ok());
    }

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
    fn validate_map_accepts_good_and_rejects_bad() {
        assert!(validate_map(&map(&[("access", &["write"]), ("dept", &["cardiology"])])).is_ok());
        assert!(validate_map(&BTreeMap::new()).is_ok()); // empty map clears
        // reserved key
        assert!(validate_map(&map(&[("email", &["x"])])).is_err());
        // uppercase key
        assert!(validate_map(&map(&[("Access", &["write"])])).is_err());
        // bad value
        assert!(validate_map(&map(&[("access", &["Write"])])).is_err());
        // empty value list for a key
        assert!(validate_map(&map(&[("access", &[])])).is_err());
    }
}
