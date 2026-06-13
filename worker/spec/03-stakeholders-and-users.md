## 3. Stakeholders and Users

The Worker entity targets a worldwide public governmental deployment:
population-scale registries operated by or for government, with
millions of users.

| Stakeholder | Interest | Touches |
|---|---|---|
| **Registry operators** | Day-to-day data stewardship: create / correct records, work the duplicate review queue, perform merges | Front-end UI; service REST API |
| **Licensing / credentialing authorities** | Authoritative source of professional identifiers (NPI, DEA, board licence, ODS); want their issued credentials reflected accurately | Service API (bulk import — roadmap §15); identifier + document model |
| **Government agencies** | Cross-agency workforce identity resolution; one canonical record per professional across HR, scheduling, payroll shards | Service REST / FHIR / gRPC APIs; match + merge workflows |
| **Employers and verifiers** | Verify a professional's credentials and registry status before engagement | Search + match endpoints; credential-verification integrations (roadmap §15) |
| **Workers (data subjects)** | Accuracy of their own record; GDPR rights — access, erasure, consent | GDPR export, soft delete, consent model, masked views |
| **Auditors and regulators** | Who saw / changed what, when; HIPAA-grade trail; GDPR / UK DPA / ISO 27001 evidence | Audit log + audit query API; event stream |
| **Developers and AI agents** | Clear contracts to build against; spec-driven discipline | This spec; per-crate specs and `AGENTS/` docs |

### 3.1 User roles (planned)

JWT-enforced roles are queued in the service spec
([§13 T-1](../worker-service-rust-crate/spec/13-tasks.md)):
HR-admin, credentialing-officer, read-only, and service (machine)
roles, with tokens issued by the
[authentication entity](../../authentication/) and verified offline
via JWKS. Until T-1 lands, all API access is unauthenticated —
acceptable for development, a blocker for governmental deployment
(§15).
