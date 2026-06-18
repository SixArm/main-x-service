//! HTTP response bodies for the auth controllers.

use serde::{Deserialize, Serialize};

use crate::models::_entities::{auth_events, sessions, users};

/// Response returned after a redeemed magic link: the access token plus
/// the authenticated user's public fields.
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    /// RS256 access token (bearer).
    pub token: String,
    /// User public id (`pid`).
    pub pid: String,
    /// Display name.
    pub name: String,
    /// Account email.
    pub email: String,
    /// Whether the email has been verified.
    pub is_verified: bool,
}

impl LoginResponse {
    /// Build a login response for `user`, carrying `token`.
    #[must_use]
    pub fn new(user: &users::Model, token: &str) -> Self {
        Self {
            token: token.to_string(),
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            is_verified: user.email_verified_at.is_some(),
        }
    }
}

/// Response for `GET /api/auth/me` — the current user's public fields.
#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentResponse {
    /// User public id (`pid`).
    pub pid: String,
    /// Display name.
    pub name: String,
    /// Account email.
    pub email: String,
}

impl CurrentResponse {
    /// Build a current-user response from `user`.
    #[must_use]
    pub fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}

/// The `users` row, as exported for GDPR right of access (Art. 15). The
/// `password` (an unusable random hash), `api_key`, and live magic-link
/// material are **deliberately excluded** — they are secrets/credentials,
/// not subject data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountUserExport {
    /// User public id (`pid`).
    pub pid: String,
    /// Account email.
    pub email: String,
    /// Display name.
    pub name: String,
    /// When the email was verified, if ever (RFC 3339).
    pub email_verified_at: Option<String>,
    /// Row creation time (RFC 3339).
    pub created_at: String,
    /// Last update time (RFC 3339).
    pub updated_at: String,
}

impl AccountUserExport {
    /// Build the user portion of the export from `user`.
    #[must_use]
    pub fn new(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            email: user.email.clone(),
            name: user.name.clone(),
            email_verified_at: user.email_verified_at.map(|t| t.to_rfc3339()),
            created_at: user.created_at.to_rfc3339(),
            updated_at: user.updated_at.to_rfc3339(),
        }
    }
}

/// One session row, as exported for GDPR right of access. Carries the
/// issuance/expiry/revocation timestamps and the captured `user_agent` —
/// **never** the token: the `jid` is the JWT `jti` (an opaque id), not a
/// credential.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountSessionExport {
    /// JWT id (`jti`) — an opaque identifier, not a credential.
    pub jid: String,
    /// When the session/token was issued (RFC 3339).
    pub issued_at: String,
    /// Token expiry (RFC 3339).
    pub expires_at: String,
    /// When the session was revoked, if ever (RFC 3339).
    pub revoked_at: Option<String>,
    /// Best-effort user agent captured at issuance.
    pub user_agent: Option<String>,
}

impl AccountSessionExport {
    /// Build a session export entry from `session`.
    #[must_use]
    pub fn new(session: &sessions::Model) -> Self {
        Self {
            jid: session.jid.clone(),
            issued_at: session.created_at.to_rfc3339(),
            expires_at: session.expires_at.to_rfc3339(),
            revoked_at: session.revoked_at.map(|t| t.to_rfc3339()),
            user_agent: session.user_agent.clone(),
        }
    }
}

/// One audit-trail row, as exported for GDPR right of access. Mirrors the
/// `auth_events` row; never carries a token or secret.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountAuditExport {
    /// Event name, e.g. `signup` / `magic_link_redeemed` / `signout` /
    /// `account_erased`.
    pub event: String,
    /// Normalised subject email where applicable.
    pub email: Option<String>,
    /// Subject user pid when known.
    pub user_pid: Option<String>,
    /// Outcome marker, e.g. `ok` / `issued` / `unknown_email`.
    pub detail: Option<String>,
    /// When the event was recorded (RFC 3339).
    pub created_at: String,
}

impl AccountAuditExport {
    /// Build an audit export entry from an `auth_events` row.
    #[must_use]
    pub fn new(row: &auth_events::Model) -> Self {
        Self {
            event: row.event.clone(),
            email: row.email.clone(),
            user_pid: row.user_pid.map(|p| p.to_string()),
            detail: row.detail.clone(),
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

/// The full GDPR Art. 15 (right of access) account export: everything the
/// service holds about the authenticated subject — their `users` row,
/// their `sessions`, and their `auth_events` audit trail. No tokens, key
/// material, password hash, or api key are ever included.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountExport {
    /// The subject's account record.
    pub user: AccountUserExport,
    /// Every session ever issued to the subject (newest first).
    pub sessions: Vec<AccountSessionExport>,
    /// The subject's authentication audit trail (newest first).
    pub auth_events: Vec<AccountAuditExport>,
}

impl AccountExport {
    /// Assemble the export document from the gathered in-memory rows.
    /// Pure (no I/O): the controller does the queries, this only shapes
    /// the response, which keeps the assembly unit-testable.
    #[must_use]
    pub fn new(
        user: &users::Model,
        sessions: &[sessions::Model],
        auth_events: &[auth_events::Model],
    ) -> Self {
        Self {
            user: AccountUserExport::new(user),
            sessions: sessions.iter().map(AccountSessionExport::new).collect(),
            auth_events: auth_events.iter().map(AccountAuditExport::new).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AccountExport;
    use crate::models::_entities::{auth_events, sessions, users};
    use chrono::{FixedOffset, TimeZone};
    use uuid::Uuid;

    fn ts() -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 6, 13, 12, 0, 0)
            .unwrap()
    }

    fn user(pid: Uuid) -> users::Model {
        users::Model {
            created_at: ts(),
            updated_at: ts(),
            id: 1,
            pid,
            email: "alice@example.com".to_string(),
            password: "unusable-hash".to_string(),
            api_key: "lo-secret".to_string(),
            name: "Alice".to_string(),
            reset_token: None,
            reset_sent_at: None,
            email_verification_token: None,
            email_verification_sent_at: None,
            email_verified_at: Some(ts()),
            magic_link_token: Some("live-token".to_string()),
            magic_link_expiration: Some(ts()),
            deleted_at: None,
        }
    }

    #[test]
    fn export_assembles_user_sessions_and_audit_rows() {
        let pid = Uuid::new_v4();
        let sessions = vec![sessions::Model {
            created_at: ts(),
            updated_at: ts(),
            id: 7,
            jid: "jti-1".to_string(),
            user_pid: pid,
            expires_at: ts(),
            revoked_at: None,
            user_agent: Some("curl".to_string()),
        }];
        let events = vec![auth_events::Model {
            created_at: ts(),
            updated_at: ts(),
            id: 3,
            event: "signup".to_string(),
            email: Some("alice@example.com".to_string()),
            user_pid: Some(pid),
            detail: Some("created".to_string()),
        }];

        let export = AccountExport::new(&user(pid), &sessions, &events);
        assert_eq!(export.user.pid, pid.to_string());
        assert_eq!(export.user.email, "alice@example.com");
        assert_eq!(export.sessions.len(), 1);
        assert_eq!(export.sessions[0].jid, "jti-1");
        assert_eq!(export.auth_events.len(), 1);
        assert_eq!(export.auth_events[0].event, "signup");
    }

    #[test]
    fn export_excludes_secrets_and_credentials() {
        // Right of access exports subject data, never credentials/secrets:
        // no password hash, api key, or live magic-link token may appear.
        let export = AccountExport::new(&user(Uuid::new_v4()), &[], &[]);
        let json = serde_json::to_string(&export).unwrap();
        assert!(
            !json.contains("unusable-hash"),
            "password hash leaked: {json}"
        );
        assert!(!json.contains("lo-secret"), "api_key leaked: {json}");
        assert!(
            !json.contains("live-token"),
            "magic-link token leaked: {json}"
        );
        assert!(!json.contains("password"), "password field leaked: {json}");
        assert!(!json.contains("api_key"), "api_key field leaked: {json}");
    }
}
