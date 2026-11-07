-- Rename the coordinate columns so their names carry their units --
-- `latitude` -> `latitude_as_decimal_degrees`, `longitude` ->
-- `longitude_as_decimal_degrees` on `event_locations`.
--
-- A companion to 2026082200000001_location_coordinates_to_numeric, which
-- changed the *type* from double precision to numeric so a coordinate
-- survives a round trip exactly. That migration made the values right;
-- this one makes the names say what they are, so a reader of the schema
-- cannot mistake degrees for radians without opening the spec
-- (spec/latitude-longitude-as-decimal-degrees).
--
-- A rename, not a new column plus a copy: metadata-only, so it is fast
-- on a large table and the two names cannot disagree in between.
--
-- Raw-SQL counterpart of migration/src/m20260824_000001_coordinate_columns_name_their_units.rs
-- (the sea-orm-migration crate). This file was missing until now, so
-- scripts/ci-check.sh's test-db stage — which applies migrations/*/up.sql
-- in lexicographic order rather than driving the sea-orm migrator — never
-- ran this rename, leaving a real Postgres in CI at the old column names
-- while every query already asked for the new ones.
ALTER TABLE event_locations
    RENAME COLUMN latitude TO latitude_as_decimal_degrees;
ALTER TABLE event_locations
    RENAME COLUMN longitude TO longitude_as_decimal_degrees;
