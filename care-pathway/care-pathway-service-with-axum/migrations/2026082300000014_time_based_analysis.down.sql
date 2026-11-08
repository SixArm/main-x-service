DROP TABLE IF EXISTS instance_segments;
ALTER TABLE pathway_instances DROP COLUMN IF EXISTS clock_stop_at;
ALTER TABLE pathway_instances DROP COLUMN IF EXISTS clock_start_at;
