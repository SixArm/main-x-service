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
| Authentication (peer verification) | Offline PASETO v4.public (Ed25519) bearer verification via `authentication-verifier` 0.3; `AuthUser` extractor + `GET /api/whoami`; env-configured key set (T-4, verification part) |
| Authentication (blanket enforcement, default-off) | Env-gated `THING_REQUIRE_AUTH` middleware (`auth::enforce` + `require_auth_mw`) on every `/api/*` route; public allow-list `/api/health`; wired on both router surfaces; DB-free enforce-matrix + flag-parser tests (T-4, enforcement part) |
| Authentication (boot-time key fetch) | `THING_PASETO_KEYS_URL` set ⇒ key set fetched over HTTP once at boot (`state::boot_verifier` in `after_routes`, before shared-store insert / middleware capture; fetched set wins; failure warn-logs and falls back to `THING_PASETO_KEYS`/empty — always boots); no refresh loop (rotation re-fetch is roadmap) (T-4, fetch part) |
| Authorization (ABAC) | Inside the blanket guard: action derived from method + destructive named POSTs (`/merge`, `/deduplicate`, `/import`); shared `authentication-verifier` 0.3 engine evaluates `THING_ABAC_POLICY`/`_FILE` (else the built-in default policy) over the token's `attrs` claim; first-match-wins, default allow-read / deny-mutation; `401` vs `403` split with deciding-rule reason; DB-free §7 test matrix (T-4, authorization part) |
| Tests | 141 unit + integration_* + Criterion benchmarks |

### 14.2 Open gaps → tasks

| Gap | Task |
|---|---|
| Fluvio production publisher | T-1 |
| `Matcher` trait abstraction | T-2 |
| gRPC API | T-3 |
| Embedding-based similarity | T-5 |
| Spec-drift CI guard | T-6 |

