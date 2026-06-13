## 1. Purpose and Vision

### 1.1 Purpose

The Person Service is a general-purpose centralised registry of
**person identities**. It sits alongside the more domain-specific
[Worker](../../../worker/worker-service-rust-crate/) index and gives callers one
canonical record per real-world person regardless of how many source
systems hold a shard of that identity. It carries the structured
fields (tax ID, identity documents, emergency contacts, multi-country
national identifiers) needed to stand in as a domain-specific identity
registry where a dedicated index is not warranted.

### 1.2 Vision

A single person identity surface that:

- Carries the structured fields (tax ID, identity documents,
  emergency contacts, multi-country national identifiers) needed to
  stand in as a domain-specific identity registry where a dedicated
  index is not warranted.
- Matches probabilistically and deterministically against arbitrary
  input, returning ranked candidates with per-component score breakdowns.
- Detects duplicates in real time on create *and* in batch on demand,
  routing them through a review queue with auto-merge for high-confidence
  matches.
- Emits audit logs and event-streaming records for every CRUD / merge
  / link operation, suitable for HIPAA-grade trails where applicable.

### 1.3 Non-goals

- **Not** a system of record for domain-specific records (e.g.
  encounters, observations, transactions, conditions) — link out
  to the dedicated domain index.
- **Not** a workforce credentialing system — use the Worker Service.
- **Not** an authentication / authorisation provider — JWT middleware
  is planned (§15) but identity proofing is out of scope.

