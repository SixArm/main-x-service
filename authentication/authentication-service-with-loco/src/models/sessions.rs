//! `sessions` model — issuance and revocation of access tokens.

use std::collections::BTreeMap;

use chrono::offset::Local;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use uuid::Uuid;

pub use super::_entities::sessions::{self, ActiveModel, Entity, Model};

/// Build the `sessions.data` JSONB payload for a new session, copying the
/// user's ABAC `attributes` under the `attrs` member (shared
/// `authorization-attributes.md` §6: session establishment copies the
/// user's attributes into the session so token minting needs no users
/// read). Pure, so the copy shape is unit-testable DB-free.
#[must_use]
pub fn session_data(attributes: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "attrs": attributes })
}

impl ActiveModelBehavior for super::_entities::sessions::ActiveModel {}

impl Model {
    /// Record a freshly issued token so it can be revoked later. `data`
    /// is the session payload JSONB — build it with [`session_data`] so
    /// the user's ABAC attributes ride along for token minting.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn issue(
        db: &DatabaseConnection,
        jid: &str,
        user_pid: Uuid,
        expires_at: chrono::DateTime<chrono::FixedOffset>,
        user_agent: Option<String>,
        data: serde_json::Value,
    ) -> ModelResult<Self> {
        let session = sessions::ActiveModel {
            jid: ActiveValue::set(jid.to_string()),
            user_pid: ActiveValue::set(user_pid),
            expires_at: ActiveValue::set(expires_at),
            revoked_at: ActiveValue::set(None),
            user_agent: ActiveValue::set(user_agent),
            data: ActiveValue::set(data),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(session)
    }

    /// Find a session by its JWT id.
    ///
    /// # Errors
    ///
    /// When the session does not exist or the query fails.
    pub async fn find_by_jid(db: &DatabaseConnection, jid: &str) -> ModelResult<Self> {
        let session = sessions::Entity::find()
            .filter(
                model::query::condition()
                    .eq(sessions::Column::Jid, jid)
                    .build(),
            )
            .one(db)
            .await?;
        session.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// All sessions for a user, newest first. Used by the GDPR account
    /// export (right of access) — issuance/expiry/revocation timestamps
    /// and the captured `user_agent`, never any token or secret.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_all_by_user_pid(
        db: &DatabaseConnection,
        user_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sessions::Entity::find()
            .filter(
                model::query::condition()
                    .eq(sessions::Column::UserPid, user_pid)
                    .build(),
            )
            .order_by_desc(sessions::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Revoke every still-active session for a user (GDPR erasure stamps
    /// `revoked_at` on each). Already-revoked sessions are left as-is so
    /// their original revocation timestamp survives. Returns the number
    /// of sessions newly revoked.
    ///
    /// # Errors
    ///
    /// When a query or update fails.
    pub async fn revoke_all_for_user(
        db: &DatabaseConnection,
        user_pid: Uuid,
    ) -> ModelResult<usize> {
        let sessions = Self::find_all_by_user_pid(db, user_pid).await?;
        let mut revoked = 0;
        for session in sessions {
            if session.is_active() {
                session.into_active_model().revoke(db).await?;
                revoked += 1;
            }
        }
        Ok(revoked)
    }

    /// True when the session exists and has not been revoked.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// The ABAC subject attributes copied into this session at
    /// establishment (`data.attrs`), parsed with the same tolerant rules
    /// as the users column ([`super::users::attributes_map`]). A session
    /// predating the copy (or holding no `attrs`) yields an empty map —
    /// read-only under the family's default policy.
    #[must_use]
    pub fn attrs(&self) -> BTreeMap<String, Vec<String>> {
        self.data
            .get("attrs")
            .map(super::users::attributes_map)
            .unwrap_or_default()
    }
}

impl ActiveModel {
    /// Mark this session revoked (signout).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn revoke(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.revoked_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, session_data};
    use chrono::{FixedOffset, TimeZone};
    use uuid::Uuid;

    fn session_with_data(data: serde_json::Value) -> Model {
        let ts = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 4, 12, 0, 0)
            .unwrap();
        Model {
            created_at: ts,
            updated_at: ts,
            id: 1,
            jid: "sid-1".to_string(),
            user_pid: Uuid::new_v4(),
            expires_at: ts,
            revoked_at: None,
            user_agent: None,
            data,
        }
    }

    #[test]
    fn attributes_copy_round_trips_user_column_to_session_attrs() {
        // The §6 copy path: users.attributes → session_data(...) at
        // establishment → Model::attrs() at token minting.
        let attributes = serde_json::json!({ "access": ["write"], "svc": ["true"] });
        let session = session_with_data(session_data(&attributes));
        let attrs = session.attrs();
        assert_eq!(attrs["access"], vec!["write"]);
        assert_eq!(attrs["svc"], vec!["true"]);
        assert_eq!(attrs.len(), 2);
    }

    #[test]
    fn session_without_attrs_yields_an_empty_map() {
        // A pre-ABAC session ({} default) or a data payload with no
        // `attrs` member grants nothing — read-only under the family's
        // default policy.
        assert!(session_with_data(serde_json::json!({})).attrs().is_empty());
        assert!(
            session_with_data(serde_json::json!({ "mfa": "done" }))
                .attrs()
                .is_empty()
        );
    }
}
