//! SEC-A9 — hash the three **bearer-equivalent** secrets at rest.
//!
//! `users.magic_link_token`, `sessions.jid` (the opaque session id in the
//! `__Host-mxi_session` cookie / PASETO `sid` claim), and the CSRF
//! synchroniser token at `sessions.data.csrf` were all stored in
//! **plaintext** — a read at rest (leaked backup, injection, log) yielded a
//! directly replayable credential. Going forward the application stores only
//! the SHA-256 hash (`crate::secret_hash`); this migration brings **existing**
//! rows into line by hashing them **in place**, so live magic links and
//! sessions keep working (the client still presents the plaintext; lookups
//! hash it and match the migrated hash).
//!
//! The hash is `encode(digest(x, 'sha256'), 'hex')` (lowercase hex), the
//! exact encoding `crate::secret_hash::hash` produces, so the two agree.
//! Each `UPDATE` is guarded on `length(...) <> 64` so it hashes only
//! not-yet-hashed values and is safe to re-run (a SHA-256 hex is always 64
//! chars; the plaintext tokens are 32 and the session id 36).
//!
//! No schema change: the columns already hold `TEXT` / `JSONB`, and a 64-char
//! hex fits. `down` is a deliberate no-op — a one-way hash cannot be
//! reversed, and the column shapes are unchanged.

use sea_orm_migration::prelude::*;

/// The credential-hash-at-rest data migration (name from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Enable `pgcrypto` (for `digest`) then hash existing plaintext
    /// credentials in place.
    ///
    /// # Errors
    ///
    /// Propagates any SQL failure from the connection.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("CREATE EXTENSION IF NOT EXISTS pgcrypto")
            .await?;
        // Magic-link tokens (short-lived; guarded so already-hashed rows are
        // skipped and the statement is idempotent).
        conn.execute_unprepared(
            "UPDATE users \
             SET magic_link_token = encode(digest(magic_link_token, 'sha256'), 'hex') \
             WHERE magic_link_token IS NOT NULL AND length(magic_link_token) <> 64",
        )
        .await?;
        // Opaque session ids.
        conn.execute_unprepared(
            "UPDATE sessions \
             SET jid = encode(digest(jid, 'sha256'), 'hex') \
             WHERE length(jid) <> 64",
        )
        .await?;
        // Per-session CSRF synchroniser token inside the `data` JSONB.
        conn.execute_unprepared(
            "UPDATE sessions \
             SET data = jsonb_set(data, '{csrf}', \
                 to_jsonb(encode(digest(data->>'csrf', 'sha256'), 'hex'))) \
             WHERE data ? 'csrf' AND length(data->>'csrf') <> 64",
        )
        .await?;
        Ok(())
    }

    /// No-op: hashing is one-way, so existing plaintext cannot be restored,
    /// and no schema was altered.
    ///
    /// # Errors
    ///
    /// Never returns an error.
    async fn down(&self, _m: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
