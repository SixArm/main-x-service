-- Which catalogued instrument a measure is a score on (spec §13 T-15b).
-- The id names an entry in `src/instruments.rs`, which owns the valid
-- range and the screen threshold; the database holds only the id.
--
-- Nullable: every existing measure is free-named and stays NULL. Nothing
-- is backfilled, because guessing an instrument from a free `name` would
-- attach a range and a threshold to a value nobody recorded against one.
ALTER TABLE instance_measures ADD COLUMN IF NOT EXISTS instrument VARCHAR NULL;
