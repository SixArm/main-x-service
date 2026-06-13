use async_trait::async_trait;
use chrono::{offset::Local, Duration};
use loco_rs::{auth::jwt, hash, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};

/// Length (chars) of a generated magic-link token.
pub const MAGIC_LINK_LENGTH: i8 = 32;
/// Magic-link validity window, in minutes.
pub const MAGIC_LINK_EXPIRATION_MIN: i8 = 5;

/// Tombstone email written by GDPR erasure, keyed by the user `pid` so
/// the `UNIQUE(email)` constraint still holds across many erased rows
/// while the original address is gone. Shape: `deleted+<pid>@invalid`
/// (`.invalid` is RFC 2606 reserved — it can never route).
#[must_use]
pub fn tombstone_email(pid: &Uuid) -> String {
    format!("deleted+{pid}@invalid")
}

/// Display name written by GDPR erasure in place of the real name.
pub const TOMBSTONE_NAME: &str = "deleted user";

/// Password-login params (loco scaffold; unused in the passwordless flow).
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    /// Account email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Password-registration params (loco scaffold; unused — see
/// [`Model::create_passwordless`]).
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    /// Account email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
    /// Display name.
    pub name: String,
}

/// Field validator for the user `ActiveModel` (name length + email).
#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    /// Display name — at least two characters.
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    /// Email — must be a syntactically valid address.
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

#[async_trait]
impl Authenticable for Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}

impl Model {
    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, email)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::EmailVerificationToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                query::condition()
                    .eq(users::Column::MagicLinkToken, token)
                    .build(),
            )
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(expired_at) = user.magic_link_expiration {
            if expired_at >= Local::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    token_expiration = expired_at.to_string(),
                    "magic token expired for the user."
                );
                Err(ModelError::msg("magic token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "magic link expiration time not exists"
            );
            Err(ModelError::msg("expiration token not exists"))
        }
    }

    /// finds a user by the provided reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ResetToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Pid, parse_uuid)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Finds a *non-erased* user by `pid`. A GDPR-erased account
    /// (`deleted_at` set) is treated as gone: the read paths (`/me`,
    /// account export) use this so a still-cryptographically-valid
    /// bearer token cannot reach a deleted user's data.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when no live user matches, plus
    /// the usual parse / DB errors.
    pub async fn find_active_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let user = Self::find_by_pid(db, pid).await?;
        if user.deleted_at.is_some() {
            return Err(ModelError::EntityNotFound);
        }
        Ok(user)
    }

    /// True when this account has been GDPR-erased (soft-deleted).
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// finds a user by the provided api key
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Verifies whether the provided plain password matches the hashed password
    ///
    /// # Errors
    ///
    /// when could not verify password
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        hash::verify_password(password, &self.password)
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, &params.email)
                    .build(),
            )
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        let password_hash =
            hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.clone()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        jwt::JWT::new(secret)
            .generate_token(expiration, self.pid.to_string(), Map::new())
            .map_err(ModelError::from)
    }

    /// Creates a passwordless user for the magic-link flow. The
    /// `password` column is `NOT NULL`, so we store the hash of an
    /// unguessable random value that no login path will ever check.
    ///
    /// # Errors
    ///
    /// When the email already exists or the insert fails.
    pub async fn create_passwordless(
        db: &DatabaseConnection,
        email: &str,
        name: &str,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Email, email)
                    .build(),
            )
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        let unusable = hash::hash_password(&Uuid::new_v4().to_string())
            .map_err(|e| ModelError::Any(e.into()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(email.to_string()),
            password: ActiveValue::set(unusable),
            name: ActiveValue::set(name.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(user)
    }
}

impl ActiveModel {
    /// Sets the email verification information for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent and generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Sets the information for a reset password request,
    /// generates a unique reset password token, and updates it in the
    /// database.
    ///
    /// This method records the timestamp when the reset password token is sent
    /// and generates a unique token for the user.
    ///
    /// # Arguments
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Records the verification time when a user verifies their
    /// email and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// email.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Resets the current user password with a new password and
    /// updates it in the database.
    ///
    /// This method hashes the provided password and sets it as the new password
    /// for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password =
            ActiveValue::set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        self.reset_token = ActiveValue::Set(None);
        self.reset_sent_at = ActiveValue::Set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let random_str = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = ActiveValue::set(Some(random_str));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
        self.update(db).await.map_err(ModelError::from)
    }

    /// Verifies and invalidates the magic link after successful authentication.
    ///
    /// Clears the magic link token and expiration time after the user has
    /// successfully authenticated using the magic link.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn clear_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
        self.update(db).await.map_err(ModelError::from)
    }

    /// GDPR Art. 17 erasure. **Soft-delete + anonymise**: stamp
    /// `deleted_at`, replace `email` with a `pid`-keyed tombstone (so the
    /// `UNIQUE(email)` constraint still holds and the original address is
    /// gone), replace `name` with [`TOMBSTONE_NAME`], and clear any live
    /// magic-link material. The row survives so the `auth_events` audit
    /// trail and any referential history keep their integrity; every read
    /// path treats a `deleted_at` user as gone
    /// ([`Model::find_active_by_pid`]). Session revocation and the audit
    /// row are written by the caller (the controller), which also owns
    /// the transaction boundary.
    ///
    /// # Errors
    /// - Returns an error if the database update fails.
    pub async fn erase(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let pid = self.pid.as_ref();
        self.email = ActiveValue::set(tombstone_email(pid));
        self.name = ActiveValue::set(TOMBSTONE_NAME.to_string());
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
        self.deleted_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{TOMBSTONE_NAME, tombstone_email};
    use uuid::Uuid;

    #[test]
    fn tombstone_email_is_pid_keyed_and_unroutable() {
        let pid = Uuid::parse_str("00000000-0000-0000-0000-0000000000ab").unwrap();
        let email = tombstone_email(&pid);
        // Carries the pid (keeps UNIQUE(email) across many erased rows)
        // and ends in the RFC 2606 reserved, never-routable `.invalid`.
        assert_eq!(
            email,
            "deleted+00000000-0000-0000-0000-0000000000ab@invalid"
        );
        assert!(email.ends_with("@invalid"));
        assert!(email.contains(&pid.to_string()));
    }

    #[test]
    fn tombstone_email_differs_per_pid() {
        let a = tombstone_email(&Uuid::new_v4());
        let b = tombstone_email(&Uuid::new_v4());
        assert_ne!(a, b, "distinct pids must yield distinct tombstone emails");
    }

    #[test]
    fn tombstone_email_carries_no_original_address() {
        // The transform is pid-only: it cannot reconstruct the original
        // address, so erasure is irreversible from the stored value.
        let pid = Uuid::new_v4();
        let email = tombstone_email(&pid);
        assert!(!email.contains("@example.com"));
        assert_eq!(TOMBSTONE_NAME, "deleted user");
    }
}
