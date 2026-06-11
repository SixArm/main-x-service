//! `sessions` model — issuance and revocation of access tokens.

use chrono::offset::Local;
use loco_rs::prelude::*;
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
