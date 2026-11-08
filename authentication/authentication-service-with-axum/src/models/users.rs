//! `users` model — passwordless magic-link accounts.
//!
//! The `users` row type and every statement that reads or writes it.
//! This is a **passwordless** service: no login path ever checks a
//! password. The `password` column exists only to satisfy `NOT NULL`,
//! and `Model::create_passwordless` fills it with the hash of an
//! unguessable random value. The loco-scaffold password/reset helpers
//! (`RegisterParams`, `Model::create_with_password`,
//! `Model::reset_password`, …) are kept for parity with the axa
//! template but are not wired into any route.
//!
//! `Model::generate_jwt` was removed on 2026-08-21. It was the only
//! reference to axa's `auth` feature anywhere in the family, and that
//! feature pulls `jsonwebtoken` and so `rsa`, which carries
//! RUSTSEC-2023-0071. This family issues PASETO v4.public and never JWT
//! (`agents/share/jwt.md`), so scaffold parity was costing a standing
//! security advisory for a function nothing called.
//!
//! The magic-link surface (`Model::create_magic_link` /
//! `Model::consume_magic_token` / `Model::clear_magic_link`) and the
//! GDPR erasure surface (`Model::erase` + `Model::find_active_by_pid`)
//! are the parts that matter here. Tokens are short-lived
//! (`MAGIC_LINK_EXPIRATION_MIN`) and single-use (cleared on redemption).
//!
//! Every write validates the row's name and email first, on insert and on
//! every update, as the `SeaORM` era's `before_save` hook did; an insert
//! also assigns the `pid` and `api_key` there, never the caller. An update
//! writes only the columns it changes, keyed by `id`, and a write that
//! matches no row is [`ModelError::RecordNotUpdated`].

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, FixedOffset, offset::Local};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use super::{ModelError, ModelResult};

/// A `users` row. Holds the magic-link credentials plus the loco-scaffold
/// password/reset fields (unused by the passwordless flow). Note that
/// `password`/`api_key`/`*_token` are **secrets**: views never serialise
/// them (see [`crate::views::auth`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation time.
    pub created_at: DateTime<FixedOffset>,
    /// Last update time.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal autoincrement primary key (never exposed externally).
    pub id: i64,
    /// Public id — the stable external UUID used in tokens (`sub`) and
    /// URLs; assigned on insert, not by the caller.
    pub pid: Uuid,
    /// Account email (unique). Anonymised to a tombstone on GDPR erasure.
    pub email: String,
    /// Password hash. **Unusable** in the passwordless flow — holds the
    /// hash of a random value no login path checks. Never serialised.
    pub password: String,
    /// Per-user api key (unique), assigned on insert. Secret.
    pub api_key: String,
    /// Display name. Anonymised to a tombstone on GDPR erasure.
    pub name: String,
    /// Password-reset token (Loco scaffold; unused by passwordless flow).
    pub reset_token: Option<String>,
    /// When the reset token was sent (Loco scaffold; unused).
    pub reset_sent_at: Option<DateTime<FixedOffset>>,
    /// Email-verification token (Loco scaffold; unused — magic-link
    /// redemption verifies the email instead).
    pub email_verification_token: Option<String>,
    /// When the verification email was sent (Loco scaffold; unused).
    pub email_verification_sent_at: Option<DateTime<FixedOffset>>,
    /// When the email was verified, if ever (set on first magic-link
    /// redemption).
    pub email_verified_at: Option<DateTime<FixedOffset>>,
    /// Live, single-use magic-link token. Secret; cleared on redemption
    /// and on erasure; never serialised into a response.
    pub magic_link_token: Option<String>,
    /// Expiry of the live magic-link token (a few minutes after issue).
    pub magic_link_expiration: Option<DateTime<FixedOffset>>,
    /// Set by GDPR Art. 17 erasure: when present the account is
    /// soft-deleted (and its `email`/`name` anonymised to a tombstone),
    /// and every read path must treat the user as gone.
    pub deleted_at: Option<DateTime<FixedOffset>>,
    /// ABAC subject attributes — a JSONB string→strings map (e.g.
    /// `{"access": ["write"]}`) minted into the PASETO `attrs` claim.
    /// Defaults to `{}` (read-only under the family's default policy)
    /// until an operator assigns attributes. See
    /// `agents/share/authorization-attributes.md` §6.
    pub attributes: serde_json::Value,
}

/// The columns a new row is inserted with; every other column takes its
/// default. `pid` and `api_key` are not here: [`Model::insert`] assigns
/// them, so they can never be spoofed via the API.
#[derive(Debug, Clone, Copy)]
pub struct NewUser<'a> {
    /// Account email.
    pub email: &'a str,
    /// Password hash.
    pub password: &'a str,
    /// Display name.
    pub name: &'a str,
}

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

/// Password-login params (Loco scaffold; unused in the passwordless flow).
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    /// Account email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Password-registration params (Loco scaffold; unused — see
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

/// Field validator for a `users` row (name length + email).
#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    /// Display name — at least two characters.
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    /// Email — must be a syntactically valid address.
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

/// Check a row's name and email before it is written, reporting failures
/// as [`ModelError::Custom`] whose text is a JSON map of field name to
/// error list.
fn validate(name: &str, email: &str) -> ModelResult<()> {
    let fields = Validator {
        name: name.to_owned(),
        email: email.to_owned(),
    };
    fields
        .validate()
        .map_err(|errors| ModelError::Custom(validation_json(&errors)))
}

/// Render validation errors as `{"field":[{"code":..,"message":..}]}`.
fn validation_json(errors: &validator::ValidationErrors) -> String {
    let fields: BTreeMap<String, Vec<serde_json::Value>> = errors
        .field_errors()
        .into_iter()
        .map(|(field, errors)| {
            let errors = errors
                .iter()
                .map(|error| serde_json::json!({ "code": error.code, "message": error.message }))
                .collect();
            (field.to_string(), errors)
        })
        .collect();
    serde_json::to_string(&fields).unwrap_or_default()
}

/// An update by `id` that returned no row matched none.
fn updated(row: Option<Model>) -> ModelResult<Model> {
    row.ok_or(ModelError::RecordNotUpdated)
}

impl Model {
    /// Insert a new row, validated first. Assigns the public id (`pid`, a
    /// fresh UUID v4 — the stable external identifier, distinct from the
    /// internal autoincrement `id`) and a random `api_key`, so neither can
    /// be spoofed via the API.
    ///
    /// # Errors
    ///
    /// [`ModelError::Custom`] when validation fails, before anything is
    /// written; otherwise when the insert fails (a taken email or api key
    /// is a unique violation).
    pub async fn insert(db: impl Into<axa::sqlx::Db<'_>>, new: NewUser<'_>) -> ModelResult<Self> {
        validate(new.name, new.email)?;
        let mut conn = db.into().acquire().await?;
        let user = sqlx::query_as!(
            Self,
            r#"INSERT INTO users (pid, email, password, api_key, name)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            Uuid::new_v4(),
            new.email,
            new.password,
            format!("lo-{}", Uuid::new_v4()),
            new.name,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(user)
    }

    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &PgPool, email: &str) -> ModelResult<Self> {
        // SEC-A6: email is case-insensitive — look up `LOWER(email)` against
        // the normalised (trimmed + lowercased) input, so `Victim@x` and
        // `victim@x` resolve to the **same** account and no case variant can
        // spawn a duplicate. (Account identity is case-only: `+tag` / Gmail
        // dot folding is deliberately NOT applied here — those are the
        // throttle bucket's concern, not identity; see `rate_limit`.)
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE LOWER(email) = $1
               LIMIT 1"#,
            normalize_email(email),
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(db: &PgPool, token: &str) -> ModelResult<Self> {
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE email_verification_token = $1
               LIMIT 1"#,
            token,
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
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
    pub async fn consume_magic_token(db: &PgPool, token: &str) -> ModelResult<Self> {
        // SEC-A9: the DB stores the token's hash, so match on the hash of the
        // presented plaintext.
        let hashed = crate::secret_hash::hash(token);
        let user = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET magic_link_token = NULL, magic_link_expiration = NULL, updated_at = now()
               WHERE magic_link_token = $1 AND magic_link_expiration >= now()
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            hashed,
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &PgPool, token: &str) -> ModelResult<Self> {
        // SEC-A9: the DB stores the token's hash; match on the hash of the
        // presented plaintext.
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE magic_link_token = $1
               LIMIT 1"#,
            crate::secret_hash::hash(token),
        )
        .fetch_optional(db)
        .await?;

        let user = user.ok_or(ModelError::EntityNotFound)?;
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
    pub async fn find_by_reset_token(db: &PgPool, token: &str) -> ModelResult<Self> {
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE reset_token = $1
               LIMIT 1"#,
            token,
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &PgPool, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE pid = $1
               LIMIT 1"#,
            parse_uuid,
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
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
    pub async fn find_active_by_pid(db: &PgPool, pid: &str) -> ModelResult<Self> {
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
    pub async fn find_by_api_key(db: &PgPool, api_key: &str) -> ModelResult<Self> {
        let user = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, email, password, api_key, name,
                      reset_token, reset_sent_at AS "reset_sent_at: _",
                      email_verification_token,
                      email_verification_sent_at AS "email_verification_sent_at: _",
                      email_verified_at AS "email_verified_at: _",
                      magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                      deleted_at AS "deleted_at: _", attributes
               FROM users
               WHERE api_key = $1
               LIMIT 1"#,
            api_key,
        )
        .fetch_optional(db)
        .await?;
        user.ok_or(ModelError::EntityNotFound)
    }

    /// Verifies whether the provided plain password matches the hashed password
    ///
    /// # Errors
    ///
    /// when could not verify password
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        crate::hash::verify_password(password, &self.password)
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
    pub async fn create_with_password(db: &PgPool, params: &RegisterParams) -> ModelResult<Self> {
        let mut txn = db.begin().await?;

        let taken = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM users WHERE email = $1) AS "taken!""#,
            params.email,
        )
        .fetch_one(&mut *txn)
        .await?;
        if taken {
            return Err(ModelError::EntityAlreadyExists);
        }

        let password_hash =
            crate::hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;
        let user = Self::insert(
            &mut *txn,
            NewUser {
                email: &params.email,
                password: &password_hash,
                name: &params.name,
            },
        )
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Creates a passwordless user for the magic-link flow. The
    /// `password` column is `NOT NULL`, so we store the hash of an
    /// unguessable random value that no login path will ever check.
    ///
    /// # Errors
    ///
    /// When the email already exists or the insert fails.
    pub async fn create_passwordless(db: &PgPool, email: &str, name: &str) -> ModelResult<Self> {
        let mut txn = db.begin().await?;

        // SEC-A6: store and match the email case-insensitively so a case
        // variant of an existing address is treated as the same account
        // (routed to the existing-email path), never inserted as a duplicate.
        let email = normalize_email(email);
        let taken = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM users WHERE LOWER(email) = $1) AS "taken!""#,
            email,
        )
        .fetch_one(&mut *txn)
        .await?;
        if taken {
            return Err(ModelError::EntityAlreadyExists);
        }

        let unusable = crate::hash::hash_password(&Uuid::new_v4().to_string())
            .map_err(|e| ModelError::Any(e.into()))?;
        let user = Self::insert(
            &mut *txn,
            NewUser {
                email: &email,
                password: &unusable,
                name,
            },
        )
        .await?;

        txn.commit().await?;
        Ok(user)
    }

    /// Sets the email verification information for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent and generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(self, db: &PgPool) -> ModelResult<Model> {
        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET email_verification_sent_at = $2, email_verification_token = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            Local::now().fixed_offset(),
            Uuid::new_v4().to_string(),
        )
        .fetch_optional(db)
        .await?;
        updated(row)
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
    pub async fn verified(self, db: &PgPool) -> ModelResult<Model> {
        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET email_verified_at = $2
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            Local::now().fixed_offset(),
        )
        .fetch_optional(db)
        .await?;
        updated(row)
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
    pub async fn reset_password(self, db: &PgPool, password: &str) -> ModelResult<Model> {
        let password =
            crate::hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?;
        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET password = $2, reset_token = NULL, reset_sent_at = NULL
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            password,
        )
        .fetch_optional(db)
        .await?;
        updated(row)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(self, db: &PgPool) -> ModelResult<Model> {
        // SEC-A9: the plaintext token is what gets emailed to the user; the
        // database stores only its hash. Generate the plaintext, persist
        // `hash(plaintext)`, then hand the caller back a model carrying the
        // *plaintext* (for the email/log) — so the usable token is never read
        // back out of the DB.
        let plaintext = crate::hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET magic_link_token = $2, magic_link_expiration = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            crate::secret_hash::hash(&plaintext),
            expired.fixed_offset(),
        )
        .fetch_optional(db)
        .await?;
        let mut model = updated(row)?;
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
    pub async fn clear_magic_link(self, db: &PgPool) -> ModelResult<Model> {
        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET magic_link_token = NULL, magic_link_expiration = NULL
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
        )
        .fetch_optional(db)
        .await?;
        updated(row)
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
        self,
        db: &PgPool,
        attributes: serde_json::Value,
    ) -> ModelResult<Model> {
        validate(&self.name, &self.email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET attributes = $2
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            attributes,
        )
        .fetch_optional(db)
        .await?;
        updated(row)
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
    pub async fn erase(self, db: impl Into<axa::sqlx::Db<'_>>) -> ModelResult<Model> {
        let mut conn = db.into().acquire().await?;
        let email = tombstone_email(&self.pid);
        validate(TOMBSTONE_NAME, &email)?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE users
               SET email = $2, name = $3, magic_link_token = NULL,
                   magic_link_expiration = NULL, deleted_at = $4
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, email, password, api_key, name,
                         reset_token, reset_sent_at AS "reset_sent_at: _",
                         email_verification_token,
                         email_verification_sent_at AS "email_verification_sent_at: _",
                         email_verified_at AS "email_verified_at: _",
                         magic_link_token, magic_link_expiration AS "magic_link_expiration: _",
                         deleted_at AS "deleted_at: _", attributes"#,
            self.id,
            email,
            TOMBSTONE_NAME,
            Local::now().fixed_offset(),
        )
        .fetch_optional(&mut *conn)
        .await?;
        updated(row)
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
