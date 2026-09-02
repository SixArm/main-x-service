-- Revert the coordinate-column rename.
ALTER TABLE event_locations
    RENAME COLUMN latitude_as_decimal_degrees TO latitude;
ALTER TABLE event_locations
    RENAME COLUMN longitude_as_decimal_degrees TO longitude;
