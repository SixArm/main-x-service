-- Add `care_pathways.content_hash`, the row-level integrity digest.
--
-- The audit chain proves the *trail* was not rewritten; this closes the
-- complementary gap by letting an out-of-band edit to an entity row be
-- detected (src/compliance/record_integrity.rs).
--
-- Nullable, so rows written before this migration stay valid:
-- verification reports them as `unhashed` rather than as mismatches, and
-- they are rehashed on their next write.
ALTER TABLE care_pathways ADD COLUMN content_hash character varying NULL;
