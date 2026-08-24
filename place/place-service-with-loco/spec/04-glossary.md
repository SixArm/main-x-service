## 4. Glossary

| Term | Meaning |
|---|---|
| **Place** | A geographic location with name, address, geo, type, identifiers |
| **GLN** | Global Location Number — 13-digit deterministic identifier with check digit |
| **GeoCoordinates** | `{ latitude, longitude, elevation? }` in WGS 84 decimal degrees, held as exact decimals (`BigDecimal` / `NUMERIC`), not binary floats — see spec §5.2.1 |
| **Hierarchy** | `contained_in_place` (parent) + `contains_place` (children); acyclic |
| **Geo-radius search** | Haversine distance + bounding-box pre-filter |
| **Match quality** | Certain / Probable / Possible / Unlikely buckets |
| **Soft delete** | `is_deleted = true`; rows are never `DELETE`d |

