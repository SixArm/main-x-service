## 14. Implementation Status

| Area | Status |
|---|---|
| Skeleton (compiles, binary runs end-to-end) | ✅ |
| loco.rs conversion (Hooks boot, native controllers, config/*.yaml, Postgres queue, loco Migrator) | ✅ family reference |
| SeaORM entities | ✅ 15 modules (providers, courses, identifiers, links, instances, syllabus_sections, audit_log, course_match_scores, course_merge_records + the normalized collection child tables) |
| Repository CRUD | ✅ courses + identifiers + links + instances + merge records; syllabus_sections still UI-only |
| Search engine | ✅ index / fuzzy / exact / blocking-query / delete |
| Validation | ✅ FR-21..FR-28 |
| Matching adapter | ✅ drives `course_matcher::MatchingEngine` (with T-6 Soundex bonus) |
| REST handlers | ✅ FR-1..FR-9 + FR-14..FR-18 (+ OpenAPI/Swagger UI) |
| Audit / streaming | ✅ in-memory MVP (T-9) **plus** the durable transactional-outbox bus (`course_outbox`, T-21), its relay + retention (T-22), and a real-broker `FluvioSink` behind the `fluvio` cargo feature (T-23, off by default) |
| Privacy | ✅ mask + GDPR export (FR-15, FR-16) |
| Row-level integrity | ✅ record + audit-log digests + keyed MAC, default off (T-24) |
| FHIR | ✅ deliberately non-standard `Basic` surface (T-20) |
| API versioning | ✅ `Accepts-version` header negotiation (T-25) |
| gRPC | – not built, not even a stub (§2.2) |
| OpenTelemetry export | – `OTLP_*` config parses but is unused; no exporter (§2.2) |
| Bulk import / export | – designed (§9.2) but not built (T-19) |
| Metrics | ✅ Prometheus `GET /metrics.prom` (T-16) — process-wide registry; `course_{created,updated,deleted,merged}_total` counters + labelled `http_requests_total{path,status}`, observed on the live request path by a `route_layer` middleware (T-18) |
| Tests | ✅ 123 unit + 2 DB-gated `#[ignore]` unit (`db::outbox_atomicity_tests`) + 14 bridge + 12 `#[ignore]` integration + 1 `#[ignore]` auth-activation (`tests/enforcement.rs`) + 1 feature-gated `#[ignore]` Fluvio round-trip (`tests/fluvio_relay.rs`) + 3 criterion benches — run `cargo test --lib` for the live count |

