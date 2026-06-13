//! HTTP response bodies for the auth controllers.

use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

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
