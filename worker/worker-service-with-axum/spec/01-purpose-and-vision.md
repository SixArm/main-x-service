## 1. Purpose and Vision

### 1.1 Purpose

The Worker Service is a centralised registry of **workforce and
professional identities**: operators, contractors, drivers, site
staff, field engineers — anyone whose role + credentials matter to
the caller.

### 1.2 Vision

One trustworthy record per worker, regardless of how many HR,
scheduling, credentialing, and payroll systems hold shards of that
identity:

- Carry credential / licence / professional-identifier fields (NPI,
  DEA, board licence, employee number) alongside the same
  fields the person index uses.
- Match probabilistically and deterministically against arbitrary
  input (typed name, partial NPI, credential number, …) and return
  ranked candidates with per-component score breakdowns.
- Detect duplicates in real time on create *and* in batch on demand.
- Emit HIPAA-grade audit logs and event-streaming records for every
  CRUD / merge / link operation.

### 1.3 Non-goals

- **Not** a credentialing / licensing source — link to the issuing
  authority; we record the credential, we do not validate it.
- **Not** a payroll system.
- **Not** an authentication / authorisation provider — the central
  authentication-service owns identity; this service only verifies its
  PASETO v4.public tokens offline (blanket enforcement is planned,
  §15); identity proofing is out of scope.

