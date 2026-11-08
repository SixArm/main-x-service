-- `users.attributes` for ABAC authorization sourcing (shared
-- `agents/share/authorization-attributes.md` §6). A string→strings map
-- (e.g. `{"access": ["write"]}`) of the subject attributes the auth
-- service mints into the PASETO `attrs` claim. Existing users default to
-- `{}` — read-only under the family's default policy — until an operator
-- assigns attributes.
ALTER TABLE users ADD COLUMN attributes jsonb NOT NULL DEFAULT '{}';
