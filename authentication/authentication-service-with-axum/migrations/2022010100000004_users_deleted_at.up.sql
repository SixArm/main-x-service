-- `users.deleted_at` for GDPR Art. 17 erasure (soft delete +
-- anonymisation). When set, the account is treated as gone — `/me`, the
-- data export, and any future read path must reject a deleted user —
-- while the row survives so referential history and the `auth_events`
-- audit trail keep their integrity. The erasure transform also
-- anonymises `email` + `name` to a tombstone (see `crate::models::users`);
-- this column records *when* that happened.
ALTER TABLE users ADD COLUMN deleted_at timestamp with time zone;
