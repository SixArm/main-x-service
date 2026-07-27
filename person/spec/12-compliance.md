## 12. Compliance

A worldwide public governmental deployment makes compliance a
first-class requirement, not a checklist. Frameworks:
[agents/share/compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md)
and
[agents/share/compliance-for-technology.md](../../agents/share/compliance-for-technology.md).

### 12.1 Framework → mechanism

| Standard | Mechanism (owner) |
|---|---|
| EU / UK GDPR, UK DPA 2018 | Data-subject rights mapping (§12.2); consent model; per-field masking (service) |
| HIPAA (where healthcare-relevant) | HIPAA-style audit trail — old/new JSON, user ID, IP, user agent, timestamp on every mutation; access tracking; soft delete (service) |
| UK NHS Act 2006 s.251 / Common Law Duty of Confidentiality | Audit + consent records support confidentiality governance (service); NHS-number handling is scheme-local in the matcher |
| ISO/IEC 27001 | Operational ISMS controls — deployment-side: TLS at edge, non-root containers, env-based secrets, health checks |
| ISO/IEC 42001:2023 | AIMS controls where matcher tuning becomes ML-driven (roadmap); today the matcher is deterministic and fully explainable, which is the strongest control |
| HL7 FHIR R5 | Person resource bidirectional conversion (service; partial — bundles and capability statement queued) |

### 12.2 Data-subject rights mapping (GDPR / UK DPA)

| Right | Implementation | Status |
|---|---|---|
| Access / portability (Art. 15 / 20) | `GET /api/persons/{id}/export` — full structured export | Service ✔; front-end download button queued (front-end §13 T-20) |
| Erasure (Art. 17) | Soft delete (`active = false`) + consent revocation; audit trail retained for legal-obligation carve-out | Service ✔ |
| Rectification (Art. 16) | `PUT /api/persons/{id}` with validation + audit row | Service ✔, front-end edit ✔ |
| Restriction / objection (Art. 18 / 21) | Consent model (`DataProcessing`, `DataSharing`, `Marketing`, `Research`) with `Active` / `Revoked` / `Expired`; query-layer enforcement is an open question (service §16 OQ-3) | Partial |
| Transparency (Art. 12–14) | Explainable match breakdowns; audit history per person | ✔ |

### 12.3 Privacy engineering

- Masking of sensitive fields (tax ID, document numbers, telecom,
  addresses) on demand: `mask_sensitive` search flag and
  `GET /api/persons/{id}/masked`.
- The matcher never logs and never performs IO — person data cannot
  leak from the scoring path.
- No PII in test fixtures, in any subproject.
- Front-end keeps no durable person data client-side (§10.3).

### 12.4 Gaps (tracked)

- No authentication on the API today — every endpoint is open until
  JWT verification lands (§13 E-1). This is the single largest
  compliance gap for a governmental deployment.
- Consent is recorded but not enforced in the query layer (service
  §16 OQ-3).
- Data-residency / cross-border transfer policy for a multi-region
  deployment is undecided (§16 EOQ-4).

### 12.4b Tamper-evident audit history (delivered 2026-07-26)

The audit chain from [`spec/compliance` §8.5](../../spec/compliance/index.md)
step 3, ported from the care-pathway reference implementation. Migration
`m20260726_000001_audit_chain` adds `seq` / `prev_hash` / `hash` /
`context` / `disclosure` / `redacted_at` to `audit_log`; every write
through `AuditLogRepository::log_action_on` — the single choke point all
audit writes already funnel through — is chained under
`pg_advisory_xact_lock`; `GET /api/audit/verify` reports linkage and
content breaks (HIPAA §164.312(c)).

**Ported, not copied.** Person's `audit_log` predates the loco-style
services, so two things differ from the reference:

- **Order comes from a new `seq BIGSERIAL`, not the primary key.** The PK
  is an application-assigned UUID, which carries no insertion order, and
  a chain needs a total order to mean anything. `timestamp` alone is not
  enough — two rows can share a microsecond and the tie-break would be
  arbitrary.
- **The digest binds request provenance** (`user_id`, `ip_address`,
  `user_agent`) alongside the old/new value pair, so an attacker cannot
  rewrite *who* acted while leaving *what* they did intact.

**Read/disclosure auditing (delivered).** `PERSON_AUDIT_READS`
(**default off**) audits `get` / `masked` / `search` / `export`. The
caller declares context in `X-Purpose-Of-Use` and
`X-Disclosure-Recipient` (normalised against a closed vocabulary, never
echoed), and it is persisted in the row's `context` with the §164.528
`disclosure` flag. On `GET /api/persons/{id}` the row is written **after**
the record-level authorization decision, so a denied request — which
disclosed nothing — never enters the accounting. A **masked** read is
still audited: §164.312(b) records activity, not just full disclosure.
An Art. 15 **export** is always audited, whatever the caller declared,
because it hands over the whole record. `search` is recorded against the
nil id, since it disclosed many records rather than one.
`PERSON_AUDIT_FAIL_CLOSED` (**default off**) decides whether a failed
audit write refuses the read with `503` or is logged.

**§164.528 accounting of disclosures (delivered 2026-07-26).**
`GET /api/persons/{{id}}/audit/disclosures` returns every audit row for
one record flagged as an outward **disclosure** rather than an internal
access, newest first. Gated by the same record-level authorization as
reading the record: learning who a record was disclosed to reveals that
the record exists, so the accounting cannot be more open than the record
it describes. An unknown id is `404`, not an empty accounting — an empty
list would tell a prober that the id is valid but never disclosed.

The response carries `read_auditing_enabled` and a `caveat`. This is not
decoration. `PERSON_AUDIT_READS` defaults off across the family, so an
empty accounting means "reads are not being recorded", not "this record
was never disclosed" — and §164.528 is a question a data subject is
entitled to a truthful answer to. Returning `[]` without saying so would
be a false answer. A test pins that the caveat names the switch.

**Defect found and fixed while building it: the entity-type vocabulary
had split.** Mutation rows have always been written with
`entity_type = "Person"`, but the read-auditing path added with the
audit chain wrote `"person"`. Every per-entity audit query filters on
one spelling, so it silently returned none of the other's rows: the
accounting would have read as empty while disclosures were being
recorded all along, and the existing `GET /api/persons/{{id}}/audit`
endpoint has been missing read rows since read-auditing landed. New rows
use `"Person"` throughout; the queries accept both spellings via `IN`
so rows already written are not orphaned, and `IN` keeps the
`(entity_type, entity_id)` index usable where a case-insensitive
comparison would not. (See the entity-type resolution above for the full spelling story.)

**GDPR Art. 17 erasure (delivered 2026-07-26).**
`POST /api/persons/{{id}}/erase` destroys the record's personal data
and appends a chained `erased` accountability row. It is a **destructive**
action under ABAC (`DESTRUCTIVE_POST_SUFFIXES`), so it requires
`access=admin` — and it is **not** the soft delete: `DELETE /{{id}}`
retires a record and keeps its data, this destroys the data and is
irreversible. The response says `irreversible: true` so a caller cannot
confuse the two.

The collision this resolves is real: honouring Art. 17 by deleting audit
rows would destroy the §164.312(c) integrity the chain exists to provide,
and refusing the erasure to protect the chain would breach Art. 17.
**Redaction** satisfies both — each audit row's snapshot is destroyed and
`redacted_at` stamped while its `hash` and `prev_hash` are left intact, so
verification still checks linkage across it and the chain as a whole keeps
verifying. What survives is the *fact* that a record existed and was
erased, by whom and when: the controller's own accountability record under
the Art. 17(3)(b) carve-out, holding nothing about the subject.

Erasing an unknown or already-erased id is answered, not refused. A
subject's right does not lapse once the record is soft-deleted — the audit
content held about it is still personal data — and a `404` would confirm
to a prober which ids are unknown.

**How this differs from the care-pathway reference.** care-pathway and
case store their whole payload as one JSONB column, so erasure there is a
single `UPDATE` replacing it with a tombstone. A person is
**relational** — names, identifiers, addresses, contacts, documents,
emergency contacts (and their telecom rows), photos, links, and match scores each live
in their own table, and the parent row itself carries `gender`,
`birth_date`, `tax_id`, `deceased_datetime`, and `marital_status`. Tombstoning one column would leave the
actual personal data untouched in ten others. So the child rows are
**deleted** outright and the parent row's own personal fields
**scrubbed**; deletion is right for the children because nothing hashes or
links them, so their absence breaks no integrity property, while a
retained-but-blanked row would still leak how many addresses or
identifiers a subject had. One tombstone name row is written back, because
the read paths assume a person has at least one name and a record with
none would be a landmine rather than a clean degradation. The whole
sequence runs in one transaction: a failure between the child deletes and
the parent scrub would leave a record with no names and un-scrubbed
demographics — worse than either outcome alone.

The test asserts the destruction **in SQL, not through the API**. Erasure
soft-deletes the record, so a later `GET` returns `404`, and an assertion
guarded by "if the read succeeded" would pass without checking anything.
A `404` proves the record is unreachable, not that the data is gone.

A DB-gated test pins the load-bearing property: after an erasure the
chain still verifies and the redactions are *counted*, not hidden. If it
ever fails, the two obligations have stopped being simultaneously
satisfiable and the design is broken, not the test.

**Still open.** Nothing from the original list. (The §164.528 accounting
endpoint, Art. 17 erasure, and row-level record integrity were all open
when this paragraph was first written; all three landed on 2026-07-26 —
see the sections below.)

**Row-level record integrity (delivered 2026-07-26).**
`persons.content_hash` carries a SHA-256 over the record, recomputed on
every write, and `GET /api/records/verify` recomputes and reports
mismatches. This is the **complement** to `/api/audit/verify`, not a
duplicate: the chain proves the *trail* was not rewritten, this proves the
*records* were not edited out of band. An attacker with SQL access who
edits a stored identifier and writes no audit row defeats the first
control and is caught by this one. It is also the gap the dropped database
triggers gestured at without ever closing.

**The digest covers the assembled record, not the `persons` row.** This is
the one substantive difference from the care-pathway reference, which
stores its whole payload in a single JSONB column. A person is
relational, and the data worth tampering with — a surname, a national
identifier, a home address — lives in the child tables. Hashing only the
parent row would have repeated the exact narrowness that made the triggers
worthless. `created_at` / `updated_at` are excluded because the ORM and
the database set them, so binding them would produce false mismatches; the
honest cost is that an attacker who alters only a timestamp is not caught
here.

**Existing rows are not back-filled.** Computing a hash from the current
content would assert that the current content is authentic — precisely the
claim the hash exists to test — so a back-fill would certify whatever an
attacker had already changed. Unhashed rows report as `unhashed`, never as
verified, and are hashed on their next write.

**The failure mode this feature has is a false accusation**, not a missed
one: a write path that forgets to rehash produces a mismatch on an
untouched record, which is worse than having no control. Only `create`
gets compiler help (its initializer names every column); `update`,
`merge`, and `delete` build their `ActiveModel` with
`..Default::default()` and would compile happily with a stale digest. A
DB-gated test therefore exercises create / update / merge / delete /
erase and asserts every record still verifies — **verified to fail** when
the rehash is removed from the update path, and guarded against passing
vacuously (if nothing were hashed, everything would count as `unhashed`
and the report would still read as verified). Erasure clears the hash
rather than recomputing one, because the child rows are gone by then and
there is no longer a record to hash; the chained `erased` audit row is the
stronger evidence anyway.

**Resolved (2026-07-26): the database audit triggers are dropped.**
`m20260726_000003_drop_audit_triggers` removes `audit_patients_changes` and
`audit_organizations_changes`. They appended rows to `audit_log` from the
database, where the application's hashing and advisory lock are
unreachable, so those rows carried a NULL `hash` and verification skipped
them — roughly half the trail. Four reasons they went rather than stayed:

1. **They were a log, not evidence.** Because verification tolerates an
   unchained row, one could be *inserted* without registering as a break,
   and *deleted* without breaking linkage either. A trigger row was as
   forgeable as the edit it claimed to witness.
2. **Worse provenance than the application's own row.** The trigger set
   `user_id` from the row's `created_by` / `updated_by` column rather than
   the authenticated caller, and could not populate `ip_address` or
   `user_agent` at all. The repository writes all three, and binds them
   into the digest.
3. **Pure duplication.** Every event a trigger caught, the repository
   already audits — and chains — in the same transaction.
4. **Narrower than they looked.** This corrects an earlier claim in this
   section. The triggers existed only on the parent `patients` and
   `organizations` tables, **not** on the child tables where most personal
   data lives (names, identifiers, addresses, contacts, documents). They
   never covered a change to any of those, so "they cover row-level
   changes the application does not audit separately" was too generous.

The genuine gap a trigger gestures at — detecting a **raw-SQL edit to an
entity row**, which no application-level audit can see — is properly
served by **row-level record integrity**: a per-row content hash, as in
the care-pathway service's `src/compliance/record_integrity.rs`. That
remains open for this crate (below), and an unchained trigger row was
never a substitute for it.

Rows already written by the triggers are **left in place**. Deleting them
would destroy audit history, and rewriting their `entity_type` would
achieve nothing since they carry no digest to invalidate. The verification
report keeps reporting `unchained` so the historical gap stays visible
rather than being quietly rounded away, and a DB-gated test pins that on a
fresh database a full create/update/delete cycle now leaves **zero**
unchained rows — verified to fail when the triggers are restored.

**Resolved (2026-07-26): the `entity_type` vocabulary.** Mutations wrote
`"Person"` while the read-auditing path wrote `"person"`, and the
triggers wrote `"patient"`. Every per-entity audit query filtered on a
single spelling, so it silently dropped the others' rows: the per-entity
audit endpoint omitted every read, and an accounting of disclosures built
on it would have looked empty while disclosures were being recorded. A
short audit answer is worse than an error, because nothing in the response
says it is incomplete.

All writers now use `"Person"`. Reads go through one shared list
(`ENTITY_TYPE_SPELLINGS` / `entity_type_spellings`) applied to **both**
`get_logs_for_entity` and `disclosures_for_entity`, so the two cannot
drift apart again; only the canonical name expands, so an unrelated type
such as `"PersonBulkExport"` is not silently widened. The `IN` keeps the
`(entity_type, entity_id)` index usable, which a case-insensitive
comparison would not.

Historical rows are **not** rewritten to the canonical spelling.
`entity_type` is bound into the chain's row digest, so an `UPDATE`
normalising it would make every affected chained row fail verification —
the chain would correctly report that someone had edited the audit trail,
because someone had. Tolerating the spelling on read is the only option
that keeps both the history and its integrity. A test pins that the
endpoint returns every row the database holds for a record, and it fails
if the filter narrows.

**Blocker cleared (2026-07-26).** Person is now enrolled in CI's DB
suites ([`ci/db-suites.txt`](../../ci/db-suites.txt)): its whole
`--ignored` suite runs against Postgres on every CI run, so the audit
chain is verified end to end rather than in isolation. Four pre-existing
defects had to be fixed first, none of them from the compliance work:

- `2024122800000005` created the `pg_trgm` extension *after* the indexes
  that use `gin_trgm_ops`, and applied that operator class to
  `patient_names.given`, which is `text[]` and which `gin_trgm_ops` does
  not accept. Fixed in place — the block could never have applied, so no
  deployment can have run past it. `given` now takes the default GIN
  `array_ops` index (containment and overlap); an expression index over
  `array_to_string(given, ' ')` is impossible because Postgres requires
  index expressions to be IMMUTABLE and that function is not. No fuzzy
  matching is lost: it happens in `person-matcher` and Tantivy.
- The rename-to-`person` migration renamed the tables but not their
  `patient_id` foreign-key columns, which the SeaORM entities declare as
  `person_id`. Fixed by a **new forward migration**
  (`m20260726_000002_rename_patient_id_columns`), not by editing the
  original — that rename *can* have applied successfully, so its history
  must not be rewritten. The new migration is idempotent, guarded by
  `to_regclass` and `information_schema.columns`.
- The bulk-import advisory-lock key (SEC-B3) embedded a literal NUL as
  its field separator. Postgres `text` cannot hold one, so **every**
  identifier-keyed import row failed with `invalid byte sequence for
  encoding "UTF8": 0x00` — the lock had never worked for the stable-key
  case it exists to serialise. The boundary is now made unambiguous by
  length-prefixing, which is injective and uses no special bytes, and a
  test pins that the key is valid Postgres text.
- `tests/common/mod.rs` opened a Tantivy index at a path no fixture
  created, so every integration test panicked before its first
  assertion.

### 12.4z Hashing reference

Everything this service hashes, how each digest is built, and — the part
that matters when a report says something is wrong — what each one does
and does not prove. All digests are **SHA-256**, rendered lowercase hex.

#### The digests

| Digest | Covers | Detects | Blind to |
|---|---|---|---|
| **Audit chain** (`audit_log.hash` / `prev_hash`) | Every audit row, linked to its predecessor | Edits, insertions, deletions and reordering **in the trail** | Changes to person rows that write no audit row |
| **Record content hash** (`persons.content_hash`) | One person's **assembled** record — parent row *and* child tables | Out-of-band SQL edits **to a record**, including its names, identifiers and addresses | Deletion of a whole row; timestamp-only edits |

They are **complementary, and neither subsumes the other.** The chain
covers the *trail*; it cannot see an edit to an entity row, because a
change made without writing an audit row leaves the chain intact. The record hash covers the *records*; it cannot see a row deleted outright in SQL, because a legitimate delete writes an audit row and an illegitimate one breaks the chain — which is the chain's job.

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

Fields bound, in order: **`version, prev_hash, id, timestamp (µs), user_id, action, entity_type, entity_id, old_values (JSON), new_values (JSON), ip_address, user_agent, context (JSON), disclosure`**.

Verification (`GET /api/audit/verify`) walks the trailing window and reports
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
advisory lock (`pg_advisory_xact_lock(0x6D78_695F_7072_736E)`) serialises the pair,
held to the end of the enclosing transaction. On a pooled connection with
no surrounding transaction each statement is its own transaction, so a
fork remains possible in principle; verification reports it rather than
hiding it.

**Redaction (GDPR Art. 17).** An erased row keeps its `hash` and
`prev_hash` and loses its content, with `redacted_at` stamped.
Verification then **skips the content check and still enforces linkage**,
so erasure and integrity hold simultaneously rather than trading off. A
test pins that redaction cannot be used to detach the following row.

**Version tag: `p1`.** Shared with the worker service, whose chain format is byte-identical — the tag names the *format*, not the crate. Renaming it would invalidate every stored worker digest, so it stays.

#### The record content hash

Each `persons` row carries `content_hash`, recomputed on **every** write.

Fields bound, in order: **`version, id, the assembled record (JSON, minus `created_at`/`updated_at`), deleted_at (µs)`**.

**The digest covers the assembled record, not the `persons` row.** This is
the substantive difference from the care-pathway reference, which stores
its whole payload in one JSONB column. A person is relational — names,
identifiers, addresses, contacts, documents and links live in their own
tables — and that is exactly where the data worth tampering with lives: a
surname, a national identifier, a home address. A parent-row digest would
have covered the shell and missed the contents, repeating the precise
narrowness that made the dropped database audit triggers worthless.

`deleted_at` is bound **separately** from the assembled record, because
the domain model does not carry it and because on this entity a soft
delete stamps `deleted_at` *without* clearing `active` — so a digest over
`active` alone could not tell a live record from a deleted one, and
un-deleting a record in SQL would go unnoticed.

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

**Version tag: `p-r1`.**

Verified by `GET /api/records/verify`.

### 12.5 Extended frameworks

Four frameworks impose obligations beyond §12.1. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Person is the
**highest-stakes adopter** in the family — it is the identity spine, so
every one of these lands harder here than anywhere else.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.312(e) transmission security, §164.528 accounting of disclosures | **Fully engaged.** Person records are personal / special-category data and, in a health deployment, ePHI. Recording mutations only does not satisfy §164.312(b): a lookup *is* the activity that matters for an identity registry. | **Read-auditing** across `get` / `list` / `search` / `check-duplicates` / `export` / FHIR reads — the same paths SEC-G3 already flags for masking — each row carrying purpose-of-use and a disclosure flag; and **tamper-evident history**: a SHA-256 chain over `audit_log` so the trail can prove it was not rewritten, plus a per-subject accounting of disclosures answering §164.528. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | **Fully engaged.** The Art. 17 ↔ audit-retention tension noted in §12.2 is the sharpest case in the family: erasing a person must not silently destroy the accountability trail. EHDS matters because a person identity is the **linkage key** its Ch. IV secondary-use pipeline depends on. | An **erasure path that survives immutable history** — redact content, preserve chain linkage and the fact an event occurred — superseding "soft delete is erasure"; a declared **data residency** (resolving §16 EOQ-4's shape, if not a deployment's choice of region); **lawful basis + Art. 9 condition** recorded rather than assumed; and bulk export leaving the region recorded as a **Ch. V transfer** event ([`bulk-import-export.md`](../../agents/share/bulk-import-export.md) §8). |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **The most meaningful adopter.** Person maps to **`Patient`**, which — unlike most of the family's resources — *does* have a US Core profile. Conformance is genuinely achievable here, modulo the family's R5-vs-R4 gap (§12.6). | **Profile and terminology validation**: US Core Patient must-support elements (`identifier`, `name`, `gender`, `birthDate`) and cardinalities checked, with `gender` bound to `administrative-gender` and identifier systems validated rather than merely non-blank; `$validate`; SMART discovery; and Bulk Data `$export` for population-level access (which the existing bulk machinery already backs). |
| **IEC 62304 / SaMD** (with ISO 14971) | The registry is not itself a device, but a **false merge is a patient-safety hazard** — attaching one person's clinical history to another is exactly the ISO 14971 harm the standard exists to control. That hazard, not device status, is why the evidence artefacts belong here. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability** (so the deterministic short-circuit rules and merge guards cannot lose their verification silently); **signed, reproducible builds**; and a hazard-to-control trace tying the false-match controls — scheme-local short-circuits (FR-7), the review queue, reversible merge snapshots (FR-14) — to the harm they mitigate. |

### 12.6 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**, and person's FHIR surface
  is a prototype that is not yet mounted
  ([`agents/share/fhir.md`](../../agents/share/fhir.md) §10). Profile and
  terminology validation are worth building on their own merits;
  certification is not claimed.
- **No hazard analysis exists.** The ISO 14971 risk file and the MDCG
  2019-11 qualification are organisational artefacts the operating
  organisation produces; the repository supplies the controls they cite.
- **Every extended control is unimplemented in this service today** —
  the reference implementation lives in the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/)
  and person is step 3 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)). Until it
  lands, read-auditing, chain integrity, and redaction-based erasure are
  an honest gap alongside §12.4's.
