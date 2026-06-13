//! `sessions` model — issuance and revocation of access tokens.

use chrono::offset::Local;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use uuid::Uuid;

pub use super::_entities::sessions::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::sessions::ActiveModel {}

impl Model {
    /// Record a freshly issued token so it can be revoked later.
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
    ) -> ModelResult<Self> {
        let session = sessions::ActiveModel {
            jid: ActiveValue::set(jid.to_string()),
            user_pid: ActiveValue::set(user_pid),
            expires_at: ActiveValue::set(expires_at),
            revoked_at: ActiveValue::set(None),
            user_agent: ActiveValue::set(user_agent),
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
