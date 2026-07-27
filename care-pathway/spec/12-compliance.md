## 12. Compliance

The Main X Index targets worldwide public governmental health
systems, so the full healthcare-compliance posture applies to this
entity. Frameworks:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
and
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

### 12.1 Data classification

Care-pathway records are **clinical artefacts, not patient data**: a
pathway definition (name, condition codes, interventions,
identifiers) contains no patient identifiers and is usually published
material. Two caveats keep the healthcare posture in force:

- **Audit trails and linked usage are personal data.** Who created,
  edited, reviewed, or merged a record — and, in downstream systems,
  which patients were placed on which pathway — is personal data
  about operators and patients respectively. The audit log (roadmap)
  MUST be treated as personal data under GDPR / UK DPA 2018.
- **Free-text fields can leak.** `keywords`, `interventions`, and
  `alternate_names` MUST never carry patient-level detail; this is a
  shared invariant (§5.5) and an operator-training point.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| US HIPAA | Pathway definitions are not PHI; HIPAA-grade audit trails (who/what/when) still required for registry operations once audit logging lands (§15); soft delete preserves history |
| UK DPA 2018 / UK GDPR / EU GDPR | Applies to operator identities in audit data and to any personal data that strays into free text; lawful-basis and accountability documentation deployment-side; soft delete supports retention policy |
| UK Common Law Duty of Confidentiality | Engaged only if confidential patient information ever enters the registry — by design it MUST NOT (§5.5) |
| UK NHS Act 2006 s251 | Not engaged for pathway definitions (no confidential patient information); becomes relevant only for downstream linkage analyses performed outside this entity |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, backups, logging |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants) |

### 12.3 Information-governance posture

- **Minimisation by design.** The domain model has no fields for
  patient data; there is nothing to mask, so the family's privacy
  controls (masking, GDPR export, consent) are deferred rather than
  required — re-assess the moment any restricted field appears
  (service spec §13).
- **Auditability.** MVP ships soft delete with timestamps; the
  compliance bar for production in a national health system is the
  full audit log + event stream of
  [`agents/share/auditability.md`](../../agents/share/auditability.md)
  — tracked as §13 T-3 and roadmap §15.
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, PASETO v4 public token verification
  per [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  — roadmap) and TLS; the registry is read-mostly and writes are
  restricted to registry operators.
- **Explainability for accountability.** Per-component match
  breakdowns give auditors a replayable rationale for every
  duplicate decision — keep this property (NFR-8).

### 12.4 Extended frameworks

Four frameworks impose obligations beyond the §12.2 table, and this
entity is the family's **reference implementation** of all four (see
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 for the regime detail and
[`spec/compliance` §8](../../spec/compliance/index.md) for the
repository-wide status). The service-side design and task breakdown
live in the service spec
[§12 / §13 T-11–T-14](../care-pathway-service-with-loco/spec/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — audit controls §164.312(b), integrity §164.312(c), transmission security §164.312(e), accounting of disclosures §164.528 | Pathway *definitions* are not PHI, but **who consulted which pathway, when, and why** is system activity over a clinical artefact, and the **instance layer** (a named patient on a pathway) is squarely ePHI. | **Read-auditing** — an audit row for reads / searches / exports / FHIR fetches, carrying purpose-of-use and a disclosure flag — and **tamper-evident history**: a SHA-256 chain over `audit_logs` so the trail can prove it was not rewritten, plus a per-record accounting of disclosures. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Fully engaged for audit-trail personal data (operator identities) and for the instance layer. EHDS's **primary vs. secondary use** split matters directly: pathway data is exactly the registry material its Ch. IV routes to research and policy use. | An **erasure path that survives the immutable chain** (redact the content, keep the linkage); a declared **data residency** and **lawful basis** stamped on every audit row; an `X-Purpose-Of-Use` marker separating care delivery from secondary use; and an export crossing the declared region recorded as a **transfer** event. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | Partial by construction: the service serves FHIR **R5** `PlanDefinition`, which has **no US Core profile**, so certification itself is out of reach (§12.5). The conformance machinery is not. | **Profile and terminology validation** — a declared `meta.profile`, must-support and cardinality checks, and condition codes validated against the value set their element is *bound* to (ICD-10 / ICD-11 / SNOMED CT) rather than merely being well-formed — plus `$validate`, SMART discovery, and Bulk Data `$export`. |
| **IEC 62304 / SaMD** (with ISO 14971, EU MDR Rule 11) | The template registry alone is not a device; the **instance layer crosses the line**, because tracking an individual patient's progress through a pathway informs care decisions. This entity therefore declares a safety classification and carries the evidence artefacts. | A **SOUP register + CycloneDX SBOM** from the real dependency graph; **machine-checked requirement→test traceability**; **signed, reproducible builds** with recorded provenance; and a runtime software-identification surface stating version, build, classification, and which controls are live. |

### 12.4z Hashing reference

Everything this service hashes, how each digest is built, and — the part
that matters when a report says something is wrong — what each one does
and does not prove. All digests are **SHA-256**, rendered lowercase hex.

#### The digests

| Digest | Covers | Detects | Blind to |
|---|---|---|---|
| **Audit chain** (`audit_logs.hash` / `prev_hash`) | Every audit row, linked to its predecessor | Edits, insertions, deletions and reordering **in the trail** | Changes to care-pathway rows that write no audit row |
| **Record content hash** (`care_pathways.content_hash`) | One pathway row's content and lifecycle state | Out-of-band SQL edits **to a record** | Deletion of a whole row; timestamp-only edits |

They are **complementary, and neither subsumes the other.** The chain
covers the *trail*; it cannot see an edit to an entity row, because a
change made without writing an audit row leaves the chain intact. The record hash covers the *rows*; it cannot see a row deleted outright in SQL, because a legitimate delete writes an audit row and an illegitimate one breaks the chain — which is the chain's job.

#### How a pre-image is built

Every digest is the SHA-256 of a **field-separated pre-image**, built the
same way:

```text
SHA256( version ␟ field₁ ␟ field₂ ␟ … ␟ fieldₙ ␟ )
```

- **`␟` is ASCII 31 (unit separator)**, appended after *every* field
  including the last. A separator that cannot occur in the data is what
  stops two different records producing the same pre-image: without it,
  `("ab", "c")` and `("a", "bc")` would concatenate identically.
- **The version tag is the first field.** It is bound *into* the digest,
  not stored beside it, so changing the hashed field set later cannot be
  mistaken for tampering — every old row simply fails against the new
  format, loudly, instead of silently comparing equal.
- **Absent values hash as the empty string**, so `None` and `Some("")`
  are indistinguishable. This is a deliberate, small loss: it keeps the
  pre-image a plain string join rather than a length-prefixed encoding.
- **Booleans hash as `"1"` / `"0"`.**

#### Reproducibility — the two rules that make it survive Postgres

A digest is worthless if the value it was computed over is not the value
that comes back out of the database. Two normalisations exist for exactly
that reason, and both have caused real failures elsewhere:

1. **Time is hashed as epoch microseconds, truncated before storing.**
   Postgres `timestamptz` holds microsecond precision; Rust's clock offers
   nanoseconds. Hashing an untruncated instant produces a digest over a
   value the database will never return. Writers call `trunc_micros`
   before storing, so the hashed value and the stored value are the same
   instant.
2. **JSON is hashed as `serde_json`'s serialization of the parsed value**,
   never as the caller's raw text. `serde_json`'s object representation is
   a `BTreeMap`, so keys come out sorted — which is what a JSONB round
   trip also returns. `{"b":2,"a":1}` and `{"a":1,"b":2}` therefore hash
   identically, as they must, since Postgres will return whichever it
   likes.

Both rules are pinned by DB-gated tests that write through the real
repository and verify after a round trip. Those tests are the only thing
that can catch a normalisation this code did not anticipate, which is why
they are `--ignored`-gated rather than absent: they need a real Postgres,
and a unit test cannot stand in for one.

#### Adoption on a populated table

A row with **no stored digest is reported as `unhashed`, never as a
mismatch, and never as verified.** Two consequences worth being explicit
about:

- Turning the control on does not produce a wall of false positives on
  rows that predate it.
- `verified: true` alongside a non-zero `unhashed` count means "nothing
  detected", not "everything is intact". The counts are reported
  separately so the difference stays visible.

Existing rows are **not back-filled**. Computing a digest from current
content asserts that the current content is authentic — precisely the
claim the digest exists to test — so a back-fill would certify whatever
an attacker had already changed. Rows are hashed on their next write.

#### The audit chain

Each audit row binds its predecessor's digest, so the rows form a chain:

```text
row₁.hash = H(version, "",         row₁ fields…)
row₂.hash = H(version, row₁.hash,  row₂ fields…)
row₃.hash = H(version, row₂.hash,  row₃ fields…)
```

Fields bound, in order: **`version, prev_hash, entity_pid, action, actor, created_at (µs), snapshot (JSON), context (JSON), disclosure`**.

Verification (`GET /api/compliance/audit/verify`) walks the trailing window and reports
two distinct break kinds, because they mean different things:

- **`content`** — the row's stored digest does not match a recomputation.
  The row was *edited*.
- **`linkage`** — the row's `prev_hash` does not match its predecessor's
  digest. A row was *inserted, deleted, or reordered*.

This is what an append-only convention alone cannot give you: deleting a
row is invisible unless something downstream depends on it.

**Concurrency.** Reading the head and appending must be atomic, or two
writers claim the same predecessor and fork the chain — which surfaces as
a `linkage` break that looks like tampering but is not. A Postgres
advisory lock (`pg_advisory_xact_lock(0x6D78_695F_6175_6469)`) serialises the pair,
held to the end of the enclosing transaction. On a pooled connection with
no surrounding transaction each statement is its own transaction, so a
fork remains possible in principle; verification reports it rather than
hiding it.

**Redaction (GDPR Art. 17).** An erased row keeps its `hash` and
`prev_hash` and loses its content, with `redacted_at` stamped.
Verification then **skips the content check and still enforces linkage**,
so erasure and integrity hold simultaneously rather than trading off. A
test pins that redaction cannot be used to detach the following row.

**Version tag: `v1`.**

#### The record content hash

Each `care_pathways` row carries `content_hash`, recomputed on **every** write.

Fields bound, in order: **`version, pid, name, data (JSON), active, deleted_at (µs)`**.

The whole payload is one JSONB `data` column, so hashing "the record" is hashing one field. The relational services (person, worker) must assemble theirs first — see their specs.

**Excluded: `created_at` / `updated_at`.** The ORM and the database set
them, so binding them would make the digest depend on values the writer
does not control — producing mismatches on rows nobody touched. The cost
is stated plainly: an attacker who alters *only* a timestamp is not caught
here. Anything that changes what the record says is.

**The failure mode is a false accusation, not a missed one.** A write path
that forgets to rehash flags an *untouched* record as tampered, which is
worse than having no control at all. That shapes how it is tested: rather
than testing that tampering is caught (necessary but easy), the suite
drives every write path — create, update, merge, delete, erase — and
asserts every record still verifies afterwards. Those tests were each
confirmed to fail when a rehash is removed, so they are known to be load-
bearing rather than decorative.

**Erasure clears the digest rather than recomputing one.** After an Art. 17
erasure the child rows are gone, so there is no assembled record left to
hash; a recomputation would certify a half-destroyed state. `NULL` puts
the row in the `unhashed` bucket, so an erased record reads as a gap, not
as tampering. The chained `erased` audit row is the stronger evidence.

**Version tag: `r1`.**

Verified by `GET /api/compliance/records/verify`. Both verification
endpoints on this service sit under `/api/compliance` alongside the SBOM
and posture endpoints, not under `/api/care-pathways` — unlike case,
person and worker, whose chain endpoints hang off their entity or root
prefix.

### 12.5 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; this service is **R5** and `PlanDefinition` has no
  US Core profile. The service implements profile / terminology
  validation, `$validate`, SMART *discovery*, and Bulk Data because each
  is independently worth having — it does **not** claim certification,
  and the SMART discovery document advertises the deployment's actual
  authorisation server rather than implying the family's PASETO
  credential is SMART OAuth.
- **Not a registered medical device.** Declaring an IEC 62304 safety
  classification and shipping the lifecycle evidence is preparation, not
  conformity assessment. A deployment that surfaces pathway steps as
  clinical decision support must complete the qualification (MDCG
  2019-11) and the ISO 14971 risk file itself; those are organisational
  artefacts (§12.3, [`spec/compliance` §6.3](../../spec/compliance/index.md)).
- **Chain scope.** The hash chain proves the **audit trail** was not
  rewritten. It does not prove the `care_pathways` rows were not —
  row-level integrity hashing over the entity table is not built.
- **Read-auditing is default-off.** `CARE_PATHWAY_AUDIT_READS` defaults
  off so adopting the change is behaviour-neutral; a HIPAA-facing
  deployment MUST turn it on, alongside `CARE_PATHWAY_REQUIRE_AUTH`
  (§12.3, and the activation gate in
  [`agents/share/security.md`](../../agents/share/security.md) §4).
- **EHDS data permits and secure processing environments** are not
  built — they are an operating-organisation and infrastructure
  concern. The service contributes the purpose-of-use marking and the
  export audit a permit regime consumes.
