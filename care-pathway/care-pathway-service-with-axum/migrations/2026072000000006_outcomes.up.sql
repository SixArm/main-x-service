-- Instance outcomes: an `outcome` on the instance and the per-instance
-- measures log (`instance_measures`: a named numeric or text value
-- recorded on a date).
ALTER TABLE pathway_instances ADD COLUMN IF NOT EXISTS outcome VARCHAR NULL;
CREATE TABLE IF NOT EXISTS instance_measures (
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    id SERIAL PRIMARY KEY,
    pid UUID NOT NULL UNIQUE,
    instance_pid UUID NOT NULL,
    name VARCHAR NOT NULL,
    value_numeric DOUBLE PRECISION NULL,
    value_text VARCHAR NULL,
    unit VARCHAR NULL,
    recorded_on DATE NOT NULL DEFAULT CURRENT_DATE
);
CREATE INDEX IF NOT EXISTS instance_measures_instance
    ON instance_measures (instance_pid, name, recorded_on);
