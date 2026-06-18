## 8. Architecture

### 8.1 Module layout

```
src/
├── lib.rs                 # Library root
├── models/
│   ├── thing.rs           # Thing
│   ├── identifier.rs      # ThingIdentifier + IdentifierType
│   └── consent.rs         # Consent
├── matching/
│   ├── name.rs            # Jaro-Winkler
│   ├── description.rs     # Jaro-Winkler
│   ├── url.rs             # scheme/case-normalized comparison
│   ├── identifier.rs      # PropertyValue exact match + deterministic detection
│   ├── phonetic.rs        # Soundex
│   └── scoring.rs         # compute_match, MatchWeights, MatchConfidence
├── validation/            # boundary validators + normalisers
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
| `EventProducer` | `InMemoryEventPublisher` (Fluvio planned) |

A `ThingMatcher` trait is an open question (OQ-2).

