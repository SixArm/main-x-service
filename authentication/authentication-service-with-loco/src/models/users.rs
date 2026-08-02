//! `users` model — passwordless magic-link accounts.
//!
//! Layers the magic-link domain logic over the generated `users` entity.
//! This is a **passwordless** service: no login path ever checks a
//! password. The `password` column exists only to satisfy `NOT NULL`,
//! and `Model::create_passwordless` fills it with the hash of an
//! unguessable random value. The loco-scaffold password/reset helpers
//! (`RegisterParams`, `Model::create_with_password`,
//! `Model::generate_jwt`, `ActiveModel::reset_password`, …) are kept
//! for parity with the loco template but are not wired into any route.
//!
//! The magic-link surface (`ActiveModel::create_magic_link` /
//! `Model::find_by_magic_token` / `ActiveModel::clear_magic_link`)
//! and the GDPR erasure surface (`ActiveModel::erase` +
//! `Model::find_active_by_pid`) are the parts that matter here. Tokens
//! are short-lived (`MAGIC_LINK_EXPIRATION_MIN`) and single-use (cleared
//! on redemption).

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{Duration, offset::Local};
use loco_rs::{auth::jwt, hash, prelude::*};
use sea_orm::FromQueryResult;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;

/// Re-export the generated `users` entity (module + `ActiveModel` /
/// `Entity` / `Model`) so callers use `users::Model` etc. through this
/// domain module rather than reaching into `_entities`.
pub use super::_entities::users::{self, ActiveModel, Entity, Model};

/// SEC-A6 — canonicalise an email for **account identity**: trim + lowercase.
///
/// Email addresses are treated case-insensitively, so `Victim@X.com` and
/// `victim@x.com` are the **same** account. This is deliberately case-only —
/// `+tag` sub-addressing and Gmail dot-folding are **not** applied here,
/// because those are provider-specific and must not silently merge two
/// distinct addresses into one account. (The rate limiter folds those into
/// one throttle *bucket*; see `crate::rate_limit::normalize_key`.)
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// SEC-A6 — the case-insensitive account-identity predicate:
/// `LOWER(email) = <normalised>`.
///
/// Built from sea-query's typed `LOWER()` rather than
/// `Expr::cust_with_values("LOWER(email) = ?", …)`. The custom form emits
/// the `?` **verbatim**, which is the `MySQL` placeholder where Postgres
/// wants `$n` — so the driver sent `... WHERE LOWER(email) = ? LIMIT $1` and
/// Postgres rejected it with `syntax error at or near "LIMIT"`. That
/// broke every email lookup: signup, magic-link request, and the
/// duplicate-account guard. Nothing caught it because the failure needs a
/// real Postgres to appear at all.
fn lower_email_eq(normalised: &str) -> sea_orm::sea_query::SimpleExpr {
    Expr::expr(Func::lower(Expr::col(users::Column::Email))).eq(normalised)
}

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

/// Parse a stored attribute JSONB value (`users.attributes`, or the
/// `attrs` member of `sessions.data`) into the ABAC string→strings
/// subject-attribute map minted into the PASETO `attrs` claim (shared
/// `agents/share/authorization-attributes.md` §2–§3).
///
/// Tolerant by design, mirroring the claim's forward-compatibility rule
/// ("unknown attributes are inert"): a string value coerces to a
/// one-element list, non-string list items are skipped, and any other
/// value shape (or a non-object input) yields no entry — a malformed
/// stored value can never fail token minting, it just grants nothing.
#[must_use]
pub fn attributes_map(value: &serde_json::Value) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    let Some(object) = value.as_object() else {
        return map;
    };
    for (key, entry) in object {
        let values: Vec<String> = match entry {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect(),
            _ => continue,
        };
        map.insert(key.clone(), values);
    }
    map
}

/// Serialize an ABAC subject-attribute map back into the canonical
/// `users.attributes` JSONB shape (`{ "<key>": ["<value>", …], … }`) —
/// the inverse of [`attributes_map`]. Every key maps to a JSON array of
/// string values, so a round-trip through `attributes_map` is lossless.
/// Used by the operator attribute-assignment task (`user_attributes`) to
/// write the updated map back to the column.
#[must_use]
pub fn attributes_to_value(map: &BTreeMap<String, Vec<String>>) -> serde_json::Value {
    let object: Map<String, serde_json::Value> = map
        .iter()
        .map(|(key, values)| {
            (
                key.clone(),
                serde_json::Value::Array(
                    values
                        .iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect(),
                ),
            )
        })
        .collect();
    serde_json::Value::Object(object)
}

/// Password-login params (loco scaffold; unused in the passwordless flow).
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    /// Account email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Password-registration params (loco scaffold; unused — see
/// `Model::create_passwordless`).
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
    /// Yield the field [`Validator`] for this `ActiveModel`, so loco runs
    /// name/email validation in `before_save`.
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    /// `SeaORM` hook run before every insert/update. Always validates the
    /// fields; on **insert** it also assigns the public id (`pid`, a fresh
    /// UUID v4 — the stable external identifier, distinct from the
    /// internal autoincrement `id`) and a random `api_key`. Both are set
    /// here, not by the caller, so they can never be spoofed via the API.
    ///
    /// # Errors
    ///
    /// Returns a `DbErr` when validation fails.
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
    /// loco `Authenticable` hook: resolve a user from an api-key bearer.
    /// Present for loco's middleware; the magic-link surface authenticates
    /// via RS256 JWTs (see [`crate::auth`]) rather than api keys.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when no user has that api key, plus
    /// the usual DB errors.
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

    /// loco `Authenticable` hook: resolve a user from the JWT claims key.
    /// Our claims key is the `pid`, so this delegates to
    /// [`Self::find_by_pid`].
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when no user matches, plus parse / DB
    /// errors.
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
        // SEC-A6: email is case-insensitive — look up `LOWER(email)` against
        // the normalised (trimmed + lowercased) input, so `Victim@x` and
        // `victim@x` resolve to the **same** account and no case variant can
        // spawn a duplicate. (Account identity is case-only: `+tag` / Gmail
        // dot folding is deliberately NOT applied here — those are the
        // throttle bucket's concern, not identity; see `rate_limit`.)
        let user = users::Entity::find()
            .filter(lower_email_eq(&normalize_email(email)))
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

    /// Atomically **consume** a magic-link token (SEC-A4): a single `UPDATE`
    /// clears the token + expiration and `RETURNING`s the row, but only when
    /// the token exists AND is unexpired. Because it is one statement, two
    /// concurrent redemptions of the same link race on it — exactly one gets
    /// a row (the winner), the other gets none — so a magic link is truly
    /// single-use even under concurrency. The prior `find_by_magic_token`
    /// (SELECT) + `clear_magic_link` (UPDATE) let both concurrent requests
    /// pass the read and each mint a session.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when the token is unknown, expired, or
    /// already consumed (all map to the same `401` at the controller).
    pub async fn consume_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        // SEC-A9: the DB stores the token's hash, so match on the hash of the
        // presented plaintext.
        let hashed = crate::secret_hash::hash(token);
        let stmt = sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE users \
             SET magic_link_token = NULL, magic_link_expiration = NULL, updated_at = now() \
             WHERE magic_link_token = $1 AND magic_link_expiration >= now() \
             RETURNING *",
            [hashed.into()],
        );
        match db.query_one_raw(stmt).await? {
            Some(row) => Model::from_query_result(&row, "").map_err(ModelError::from),
            None => Err(ModelError::EntityNotFound),
        }
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        // SEC-A9: the DB stores the token's hash; match on the hash of the
        // presented plaintext.
        let user = users::Entity::find()
            .filter(
                query::condition()
                    .eq(
                        users::Column::MagicLinkToken,
                        crate::secret_hash::hash(token),
                    )
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

    /// This user's ABAC subject attributes, parsed from the
    /// `users.attributes` JSONB column (see [`attributes_map`]). Copied
    /// into the session at establishment and minted into the PASETO
    /// `attrs` claim.
    #[must_use]
    pub fn attrs(&self) -> BTreeMap<String, Vec<String>> {
        attributes_map(&self.attributes)
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

        // SEC-A6: store and match the email case-insensitively so a case
        // variant of an existing address is treated as the same account
        // (routed to the existing-email path), never inserted as a duplicate.
        let email = normalize_email(email);
        if users::Entity::find()
            .filter(lower_email_eq(&email))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }

        let unusable = hash::hash_password(&Uuid::new_v4().to_string())
            .map_err(|e| ModelError::Any(e.into()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(email),
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
        // SEC-A9: the plaintext token is what gets emailed to the user; the
        // database stores only its hash. Generate the plaintext, persist
        // `hash(plaintext)`, then hand the caller back a model carrying the
        // *plaintext* (for the email/log) — so the usable token is never read
        // back out of the DB.
        let plaintext = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = ActiveValue::set(Some(crate::secret_hash::hash(&plaintext)));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
        let mut model = self.update(db).await.map_err(ModelError::from)?;
        model.magic_link_token = Some(plaintext);
        Ok(model)
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

    /// Replace this user's ABAC subject attributes (`users.attributes`)
    /// with `attributes` (the canonical `{ "<key>": ["<value>", …] }`
    /// JSONB shape, e.g. from [`attributes_to_value`]) and persist. Used
    /// by the operator attribute-assignment task (`user_attributes`); the
    /// new map is copied into a session at its next establishment and
    /// minted into the PASETO `attrs` claim from there.
    ///
    /// # Errors
    /// - Returns an error if the database update fails.
    pub async fn set_attributes(
        mut self,
        db: &DatabaseConnection,
        attributes: serde_json::Value,
    ) -> ModelResult<Model> {
        self.attributes = ActiveValue::set(attributes);
        self.update(db).await.map_err(ModelError::from)
    }

    /// GDPR Art. 17 erasure. **Soft-delete + anonymise**: stamp
    /// `deleted_at`, replace `email` with a `pid`-keyed tombstone (so the
    /// `UNIQUE(email)` constraint still holds and the original address is
    /// gone), replace `name` with [`TOMBSTONE_NAME`], and clear any live
    /// magic-link material. The row survives so the `auth_events` audit
    /// trail and any referential history keep their integrity; every read
    /// path treats a `deleted_at` user as gone
    /// (`Model::find_active_by_pid`). Session revocation and the audit
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
    use super::{TOMBSTONE_NAME, attributes_map, attributes_to_value, tombstone_email};
    use std::collections::BTreeMap;
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
    fn attributes_map_parses_the_string_to_strings_shape() {
        // The canonical stored shape: string keys → lists of strings.
        let value = serde_json::json!({
            "access": ["write"],
            "dept": ["cardiology", "oncology"],
        });
        let map = attributes_map(&value);
        assert_eq!(map.len(), 2);
        assert_eq!(map["access"], vec!["write"]);
        assert_eq!(map["dept"], vec!["cardiology", "oncology"]);
    }

    #[test]
    fn attributes_map_is_tolerant_of_malformed_values() {
        // A bare string coerces to a one-element list; non-string list
        // items are skipped; non-list/non-string values yield no entry;
        // a non-object input yields an empty map. A malformed stored
        // value must never fail token minting — it just grants nothing.
        let value = serde_json::json!({
            "svc": "true",
            "mixed": ["ok", 42, null],
            "number": 7,
            "object": {"nested": true},
        });
        let map = attributes_map(&value);
        assert_eq!(map["svc"], vec!["true"]);
        assert_eq!(map["mixed"], vec!["ok"]);
        assert!(!map.contains_key("number"));
        assert!(!map.contains_key("object"));

        assert!(attributes_map(&serde_json::json!({})).is_empty());
        assert!(attributes_map(&serde_json::Value::Null).is_empty());
        assert!(attributes_map(&serde_json::json!(["not", "an", "object"])).is_empty());
    }

    #[test]
    fn attributes_to_value_round_trips_through_attributes_map() {
        // Serialising a map to the canonical JSONB shape and parsing it
        // back must be lossless — the invariant the assignment task
        // relies on when it reads, mutates, and writes the column.
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        map.insert("access".to_string(), vec!["write".to_string()]);
        map.insert(
            "dept".to_string(),
            vec!["cardiology".to_string(), "oncology".to_string()],
        );
        let value = attributes_to_value(&map);
        // Canonical shape: every value is a JSON array of strings.
        assert_eq!(value["access"], serde_json::json!(["write"]));
        assert_eq!(value["dept"], serde_json::json!(["cardiology", "oncology"]));
        assert_eq!(attributes_map(&value), map);
    }

    #[test]
    fn attributes_to_value_of_empty_map_is_an_empty_object() {
        let value = attributes_to_value(&BTreeMap::new());
        assert_eq!(value, serde_json::json!({}));
        assert!(attributes_map(&value).is_empty());
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
