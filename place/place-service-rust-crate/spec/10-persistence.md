## 10. Persistence

PostgreSQL 18+ via SeaORM.

### 10.1 Tables (13)

`places`, `place_addresses`, `place_geo_coordinates`,
`place_identifiers`, `place_amenities`, `place_opening_hours`,
`place_same_as`, `place_hierarchy`, `place_links`,
`organizations`, `organization_addresses`,
`place_match_scores`, `audit_log`.

### 10.2 Extensions

Required: `pg_stat_statements`, `uuid-ossp`, `pgcrypto`, `pg_trgm`,
`citext`, `unaccent`, **`postgis`** (planned use: spatial indexing
and bounding-box pre-filter on geo-radius search).

