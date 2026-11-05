-- Revert geo coordinates to DOUBLE PRECISION.
--
-- Lossy by nature: a NUMERIC carrying more precision than a double can
-- represent is rounded to the nearest double. That is the cost of rolling
-- back, and is why the up migration is the direction worth being in.
ALTER TABLE places
    ALTER COLUMN geo_latitude  TYPE DOUBLE PRECISION USING geo_latitude::DOUBLE PRECISION,
    ALTER COLUMN geo_longitude TYPE DOUBLE PRECISION USING geo_longitude::DOUBLE PRECISION,
    ALTER COLUMN geo_elevation TYPE DOUBLE PRECISION USING geo_elevation::DOUBLE PRECISION;
