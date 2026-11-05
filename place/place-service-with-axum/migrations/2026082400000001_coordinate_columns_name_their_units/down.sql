-- Rename the coordinate columns back to their pre-rename names.
ALTER TABLE places
    RENAME COLUMN geo_latitude_as_decimal_degrees TO geo_latitude;
ALTER TABLE places
    RENAME COLUMN geo_longitude_as_decimal_degrees TO geo_longitude;
ALTER TABLE places
    RENAME COLUMN geo_elevation_as_decimal_metres TO geo_elevation;
