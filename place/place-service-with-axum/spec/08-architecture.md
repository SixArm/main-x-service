## 8. Architecture

### 8.1 Module layout

```
src/
├── lib.rs                 # Library root
├── main.rs                # Binary entry point (REST + gRPC API)
├── models/
│   ├── place.rs           # Place
│   ├── address.rs         # PostalAddress
│   ├── geo.rs             # GeoCoordinates + Haversine
│   ├── place_type.rs      # PlaceType enum
│   ├── identifier.rs      # PlaceIdentifier + IdentifierType (GLN, FIPS, GNIS, OSM)
│   ├── amenity.rs         # AmenityFeature
│   ├── opening_hours.rs   # OpeningHoursSpecification + DayOfWeek
│   └── consent.rs         # Consent (GDPR)
├── matching/
│   ├── name.rs            # Jaro-Winkler
│   ├── address.rs         # Weighted-field comparison
│   ├── geo.rs             # Haversine + sigmoid decay + within_radius
│   ├── identifier.rs      # Exact + has_gln_match
│   ├── phonetic.rs        # Soundex
│   └── scoring.rs         # compute_match, MatchWeights, MatchConfidence
├── validation/            # boundary validators + address normalisation
├── privacy/               # masking + GDPR export
├── api/                   # REST + gRPC (stub)
```

### 8.2 Layering rules

- `api/*` depends on `models`, `matching`, `validation`, `privacy`.
- `matching` MUST NOT depend on `api`.
- `models` are leaves.

### 8.3 Trait-based abstraction

| Trait | Implementations |
|---|---|
| (No `Matcher` trait yet — `compute_match` is a free function) | — |
| `EventProducer` | `InMemoryEventPublisher` (legacy, memory transport) |
| `relay::EventSink` | `LoggingSink` (default, no-broker) · `FluvioSink` (real broker, `fluvio` Cargo feature, off by default — BUS-3, T-12c) |

