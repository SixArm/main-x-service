## 4. Glossary

Entity-level terms. Per-crate glossaries:
[service §4](../worker-service-rust-crate/spec/04-glossary.md),
[matcher §4](../worker-matcher-rust-crate/spec/04-glossary.md),
[front-end §4](../worker-front-end-with-svelte/spec/04-glossary.md).

| Term | Meaning |
|---|---|
| **Worker** | A workforce / professional identity record. Two shapes exist: the service's rich FHIR-shaped `Worker` and the matcher's flat builder-shaped `Worker` (see §5). |
| **Trio** | The three subprojects of one entity: service crate + matcher crate + front-end project. |
| **System of record** | The service crate. The front-end is a thin presentation layer; the matcher is a pure function library. |
| **Adapter / bridge** | `src/matching/adapter.rs` in the service: the lossy but well-defined projection `to_matcher_worker()` from the service shape to the matcher shape. |
| **DTO contract** | The field-routing rules of the adapter, pinned by the bridge test suite (`tests/duplicate_detection.rs`). |
| **In-service matcher** | The service's own `src/matching/` implementation (probabilistic + deterministic). Coexists with the embedded canonical matcher. |
| **Canonical matcher** | The `worker-matcher` crate — the reference algorithm with 42 national-identifier parsers, embedded by the service and re-exported as `matcher_lib`. |
| **Identifier scheme** | A national personal-identifier system (UK NHS, US SSN, FR NIR, …). Scheme-local: identifiers MUST never cross-match across schemes. |
| **Passport book** | Matcher type `PassportBook` — ISO 3166-1 country + number + optional dates; a worker may carry several. |
| **Credential document** | Service `IdentityDocument` (passport, licence, permit, …) with expiry tracking. |
| **Blocking** | Using the search index to narrow match candidates before pairwise scoring. |
| **Short-circuit** | A deterministic rule that pins the match score (e.g. tax-ID exact match → 1.0). |
| **Review queue** | Persisted duplicate candidates with status `Pending` / `Confirmed` / `Rejected` / `AutoMerged`. |
| **Merge** | Combining a confirmed duplicate into a surviving main record: transfer, alias, `Replaces` link, soft delete, snapshot, `Merged` event. |
| **Soft delete** | Records are marked inactive, never physically deleted — required by the audit posture. |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): passwordless magic-link, RS256 JWT, JWKS for offline verification. |
| **SDD** | Spec-driven development: spec is canonical; three-part PRs (spec + code + test). |
