-- Drop the database-side audit triggers.
--
-- These fired AFTER INSERT/UPDATE/DELETE on workers and organizations and
-- INSERTed into audit_log themselves. They are removed because they made
-- the tamper-evident audit chain partial while adding nothing the
-- application does not already record better:
--
--   1. NO TAMPER EVIDENCE. A trigger cannot compute the chain: it has
--      neither the application's hashing nor its advisory lock, so its
--      rows land with a NULL hash. Verification skips them, which means a
--      row inserted with a NULL hash does not register as a break, and a
--      trigger row can be deleted without breaking linkage either. The
--      rows were a log, not evidence -- while making roughly half the
--      trail look unverifiable.
--
--   2. WORSE PROVENANCE THAN THE APPLICATION'S OWN ROW. The trigger set
--      user_id from the row's own created_by / updated_by column rather
--      than the authenticated caller, and could not populate ip_address
--      or user_agent at all. The application writes all three.
--
--   3. DUPLICATION. Every event a trigger caught, the repository already
--      audits -- and chains -- in the same transaction.
--
--   4. NARROWER THAN IT LOOKED. The triggers existed only on the parent
--      workers and organizations tables, not on the child tables where
--      most personal data lives (names, identifiers, addresses, contacts,
--      documents). They never covered a change to any of those.
--
-- The genuine gap a trigger gestures at -- detecting a raw-SQL edit to an
-- entity row, which no application-level audit can see -- is properly
-- served by row-level record integrity (a per-row content hash, as in the
-- care-pathway service's src/compliance/record_integrity.rs). That is
-- tracked as open work in this crate's spec 12-compliance.md. Keeping an
-- unchained trigger row was never a substitute for it: the trigger row is
-- as forgeable as the edit it claims to witness.
--
-- Existing trigger-written rows are deliberately LEFT IN PLACE. Deleting
-- them would destroy audit history, and rewriting their entity_type would
-- be pointless since they carry no hash to invalidate. They remain
-- readable, and the verification report keeps counting them under
-- `unchained` so the historical gap stays visible rather than being
-- quietly rounded away.

DROP TRIGGER IF EXISTS audit_workers_changes ON workers;
DROP TRIGGER IF EXISTS audit_organizations_changes ON organizations;
DROP FUNCTION IF EXISTS audit_worker_changes();
DROP FUNCTION IF EXISTS audit_organization_changes();
