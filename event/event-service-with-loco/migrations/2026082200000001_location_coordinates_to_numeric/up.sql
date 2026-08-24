-- Geo coordinates: DOUBLE PRECISION -> NUMERIC.
--
-- A latitude is a decimal quantity, not a binary one. `DOUBLE PRECISION`
-- cannot hold 37.87 -- it holds 37.869999999999997, and every read back
-- was that value re-rounded for display. NUMERIC stores the digits the
-- caller sent, so a coordinate now round-trips exactly.
--
-- The change is also what lets these columns survive serde_json's
-- `arbitrary_precision` feature: `Location` is an internally-tagged enum
-- (`#[serde(tag = "kind")]`), and under that feature serde buffers the
-- variant's fields, at which point an `f64` field can no longer be
-- deserialized. An exact decimal can.
--
-- USING is an exact widening -- every double has a NUMERIC form -- so no
-- stored value is lost or rounded by this migration. Existing rows keep
-- the float artefacts they were stored with (37.869999999999997 stays
-- that); only values written from here on are exact. Back-filling a
-- "cleaner" number would be inventing precision the caller never sent.
--
-- No scale is declared. NUMERIC(9,6) is the usual geo choice, but it
-- would silently round anything finer, and the previous DOUBLE PRECISION
-- column accepted ~15 significant digits. An unconstrained NUMERIC keeps
-- every value a client could previously send; the service caps decimal
-- places at validation time (MAX_COORDINATE_SCALE) rather than letting
-- the database truncate without telling anyone.
ALTER TABLE event_locations
    ALTER COLUMN latitude  TYPE NUMERIC USING latitude::NUMERIC,
    ALTER COLUMN longitude TYPE NUMERIC USING longitude::NUMERIC;
