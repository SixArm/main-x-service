-- `sessions.data`: the shared-design session payload column
-- (`agents/share/authentication-sessions.md` §3: "data JSONB — roles,
-- scopes, MFA state, …"). Session establishment copies the user's ABAC
-- attributes into `data.attrs` so `POST /api/auth/token` mints the
-- `attrs` claim from the session alone (shared
-- `authorization-attributes.md` §6), and holds the hash of the session's
-- CSRF token under `data.csrf`.
ALTER TABLE sessions ADD COLUMN data jsonb NOT NULL DEFAULT '{}';
