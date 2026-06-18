## 1. Purpose and Vision

### 1.1 Purpose

The Place Service is a centralised registry of **geographic places**:
sites, branches, civic structures, landforms, administrative areas,
business locations — anything modelled by
[schema.org/Place](https://schema.org/Place).

### 1.2 Vision

One canonical place record regardless of how many source systems
(CRMs, OSM imports, GLN registries, GIS feeds) hold a shard:

- Match probabilistically and deterministically by name, address,
  geo coordinates, identifier (GLN, FIPS, GNIS, OSM ID), and
  hierarchy (`containedInPlace`).
- Detect duplicates in real time on create *and* in batch on demand
  with a review queue + auto-merge.
- Expose place identity over REST (and gRPC, planned) for downstream
  routing, geo-search, analytics, and mapping systems.
- Provide **geo-radius search**: find places within R km of a
  coordinate via Haversine + bounding-box pre-filter.
- Emit audit logs and event-streaming records for every CRUD / merge
  / link.

### 1.3 Non-goals

- **Not** a tile server — the API does not serve map tiles.
- **Not** a routing engine — places have coordinates; turn-by-turn
  is out of scope.
- **Not** a geocoder — reverse-geocoding endpoint is a roadmap item.

