-- Drop the row-level integrity column.
ALTER TABLE persons DROP COLUMN IF EXISTS content_hash;
