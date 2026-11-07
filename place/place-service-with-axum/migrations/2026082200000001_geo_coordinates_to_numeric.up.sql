-- Geo coordinates: DOUBLE PRECISION -> NUMERIC.
--
-- A coordinate is a decimal quantity, not a binary one. `DOUBLE
-- PRECISION` cannot hold 40.7829 -- it holds 40.78289999999999793 -- and
-- cannot distinguish 40.7829 from 40.78290000000000001 at all. NUMERIC
-- stores the digits the caller sent, so a coordinate round-trips exactly.
--
-- Unlike event-service's equivalent change, this one fixes no
-- deserialization break: place-service has no internally-tagged enum or
-- flattened struct in its request path, so its f64 coordinates survived
-- serde_json's `arbitrary_precision` feature. This is the same
-- correctness argument applied for its own sake, and for consistency
-- across the two services that model geography.
--
-- USING is an exact widening -- every double has a NUMERIC form -- so no
-- stored value is lost or rounded here. Existing rows keep the float
-- artefacts they were written with; only values written from here on are
-- exact. Back-filling a "cleaner" number would invent precision the
-- caller never sent.
--
-- No scale is declared, matching event_locations: NUMERIC(9,6) would
-- silently round anything finer, while DOUBLE PRECISION accepted ~15
-- significant digits. The service caps decimal places at validation time
-- rather than letting the database truncate without telling anyone.
--
-- idx_places_geo (geo_latitude, geo_longitude) is rebuilt automatically
-- by Postgres as part of the type change.
ALTER TABLE places
    ALTER COLUMN geo_latitude  TYPE NUMERIC USING geo_latitude::NUMERIC,
    ALTER COLUMN geo_longitude TYPE NUMERIC USING geo_longitude::NUMERIC,
    ALTER COLUMN geo_elevation TYPE NUMERIC USING geo_elevation::NUMERIC;
