-- `auth_events.source_ip` (T-14): the same best-effort address, captured
-- on every audit row.
--
-- Deliberately excluded from the keyed-integrity pre-image
-- (`crate::compliance::audit_integrity`): that format has no per-row
-- version marker, so widening it would make every pre-existing row's
-- stored digest unrecomputable and report as tampered on the next
-- `/api/compliance/audit/verify`. `source_ip` is metadata alongside the
-- row, not content the MAC attests to.
ALTER TABLE auth_events ADD COLUMN IF NOT EXISTS source_ip character varying;
