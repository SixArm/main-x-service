-- Revert geo coordinates to DOUBLE PRECISION.
--
-- Lossy by nature: a NUMERIC carrying more precision than a double can
-- represent is rounded to the nearest double on the way back. That is
-- the cost of rolling back, and is why the up migration is the direction
-- worth being in.
ALTER TABLE event_locations
    ALTER COLUMN latitude  TYPE DOUBLE PRECISION USING latitude::DOUBLE PRECISION,
    ALTER COLUMN longitude TYPE DOUBLE PRECISION USING longitude::DOUBLE PRECISION;
