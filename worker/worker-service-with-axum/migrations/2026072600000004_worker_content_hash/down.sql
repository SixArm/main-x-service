-- Drop the row-level integrity column.
ALTER TABLE workers DROP COLUMN IF EXISTS content_hash;
