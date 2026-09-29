//! Session-cookie [`AuthUser`] for builds without the `paseto` feature.
//!
//! No token is ever issued in this configuration, so the routes that a
//! PASETO bearer would gate (`me`, `signout`, the GDPR account routes,
//! admin attributes, the audit routes) authenticate the opaque
//! `__Host-mxi_session` cookie instead: the session must exist and be
//! active, the account must not be GDPR-erased, and an unsafe method must
//! echo the session's CSRF synchroniser token in `X-CSRF-Token`
//! (constant-time compared against the stored hash). The resulting
//! [`Claims`] carry the session's `sid` and ABAC `attrs`.

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use loco_rs::app::AppContext;

use super::{AuthUser, Claims};
use crate::models::{sessions, users};

/// Lifetime stamped into synthesized claims (unused for authorization;
/// the session itself carries the real TTLs).
const SESSION_CLAIM_SECS: i64 = 300;

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppContext: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        let headers = &parts.headers;
        let sid = headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(crate::cookie::read_session)
            .ok_or((StatusCode::UNAUTHORIZED, "no session cookie"))?;
        let session = sessions::Model::find_by_jid(&ctx.db, &sid)
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "unknown session"))?;
        if !session.is_active() {
            return Err((StatusCode::UNAUTHORIZED, "session revoked or expired"));
        }
        let safe = matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS);
        if !safe {
            let provided = headers
                .get(crate::csrf::CSRF_HEADER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            let expected = session.csrf().unwrap_or_default();
            if provided.is_empty()
                || expected.is_empty()
                || !crate::csrf::matches(expected, &crate::secret_hash::hash(provided))
            {
                return Err((StatusCode::FORBIDDEN, "csrf token missing or invalid"));
            }
        }
        let user = users::Model::find_active_by_pid(&ctx.db, &session.user_pid.to_string())
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "account not found"))?;
        let now = chrono::Utc::now().timestamp();
        Ok(AuthUser(Claims {
            sub: user.pid.to_string(),
            email: user.email.clone(),
            name: user.name.clone(),
            iss: "session".to_string(),
            aud: "session".to_string(),
            exp: now + SESSION_CLAIM_SECS,
            iat: now,
            nbf: None,
            sid,
            scope: Vec::new(),
            roles: Vec::new(),
            attrs: session.attrs(),
        }))
    }
}
