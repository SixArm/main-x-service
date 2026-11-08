-- SEC-A9: hash the three **bearer-equivalent** secrets at rest.
--
-- `users.magic_link_token`, `sessions.jid` (the opaque session id in the
-- `__Host-mxi_session` cookie / PASETO `sid` claim), and the CSRF
-- synchroniser token at `sessions.data.csrf` were stored in plaintext. The
-- application now stores only the SHA-256 hash (`crate::secret_hash`);
-- this brings existing rows into line by hashing them in place, so live
-- magic links and sessions keep working (the client still presents the
-- plaintext; lookups hash it and match the migrated hash).
--
-- The hash is `encode(digest(x, 'sha256'), 'hex')` (lowercase hex), the
-- exact encoding `crate::secret_hash::hash` produces. Each UPDATE is
-- guarded on `length(...) <> 64` so it hashes only not-yet-hashed values
-- and is safe to re-run (a SHA-256 hex is always 64 chars; the plaintext
-- tokens are 32 and the session id 36).
--
-- No schema change. The down migration is a deliberate no-op: a one-way
-- hash cannot be reversed.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Magic-link tokens.
UPDATE users
SET magic_link_token = encode(digest(magic_link_token, 'sha256'), 'hex')
WHERE magic_link_token IS NOT NULL AND length(magic_link_token) <> 64;

-- Opaque session ids.
UPDATE sessions
SET jid = encode(digest(jid, 'sha256'), 'hex')
WHERE length(jid) <> 64;

-- Per-session CSRF synchroniser token inside the `data` JSONB.
UPDATE sessions
SET data = jsonb_set(data, '{csrf}',
    to_jsonb(encode(digest(data->>'csrf', 'sha256'), 'hex')))
WHERE data ? 'csrf' AND length(data->>'csrf') <> 64;
