## 14. Implementation Status

### 14.1 Delivered

| Capability | Notes |
|---|---|
| Project chassis | Cargo, modular architecture |
| Domain model | schema.org/Thing canonical properties + PropertyValue identifiers |
| Matching | Probabilistic (name / identifier / description / URL / sameAs) + deterministic (DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID short-circuit) + Soundex bonus |
| Search | Tantivy index on name / alternate_names / description / identifier value / URL / same_as |
| REST API | 15 endpoints + OpenAPI/Swagger + CORS + structured errors |
| Validation | Required `name`, URL formats, per-type identifier formats, normalisation |
| Privacy | Per-field masking (`owner`, identifier `value`), GDPR export, consent model |
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.2; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-4, verification part) |
| Authentication (blanket enforcement, default-off) | Env-gated `THING_REQUIRE_AUTH` middleware (`auth::enforce` + `require_auth_mw`) on every `/api/*` route; public allow-list `/api/health`; wired on both router surfaces; DB-free enforce-matrix + flag-parser tests (T-4, enforcement part) |
| Tests | ~100 unit + integration_* + Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Fluvio production publisher | T-1 |
| `Matcher` trait abstraction | T-2 |
| gRPC API | T-3 |
| Authentication — roles + published-key HTTP fetch (peer PASETO verification + default-off blanket enforcement delivered) | T-4 |
| Embedding-based similarity | T-5 |
| Spec-drift CI guard | T-6 |

