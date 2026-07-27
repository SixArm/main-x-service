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
and does not prove. Every digest is computed under **both SHA-256 and SHA-3** over the
same pre-image, rendered lowercase hex; see "Two
algorithms" below for why both are kept.

#### The digests

> **Everything in this section is default-off.** A deployment that sets
> nothing writes the digests and runs none of the rest: no auth on the
> audit endpoints, no read auditing, no keyed MAC, no external witness.
> Turning it on — in an order that matters — is
> [`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md).


| Digest | Covers | Detects | Blind to |
|---|---|---|---|
| **Audit chain** (`audit_log.hash` / `prev_hash`) | Every audit row, linked to its predecessor | Edits, insertions, deletions and reordering **in the trail** | Changes to person rows that write no audit row |
| **Record content hash** (`persons.content_hash`) | One person's **assembled** record — parent row *and* child tables | Out-of-band SQL edits **to a record**, including its names, identifiers and addresses | Deletion of a whole row; timestamp-only edits |

They are **complementary, and neither subsumes the other.** The chain
covers the *trail*; it cannot see an edit to an entity row, because a
change made without writing an audit row leaves the chain intact. The record hash covers the *records*; it cannot see a row deleted outright in SQL, because a legitimate delete writes an audit row and an illegitimate one breaks the chain — which is the chain's job.

#### Two algorithms: SHA-256 and SHA-3

Every digest above is computed **twice**, over a byte-identical
pre-image, and both results are stored:

| | Why it is kept |
|---|---|
| **SHA-256** | FIPS 180-4, NIST-approved, two decades of cryptanalysis, and the digest a compliance reviewer expects to see named. |
| **SHA-3** (SHA3-256) | FIPS 202, so it carries the same NIST standing — but where SHA-256 is Merkle–Damgård with a Davies–Meyer compression function, SHA-3 is a **sponge**. NIST standardised it precisely so an approved alternative would exist sharing no design lineage with SHA-2. |

**Both are FIPS/NIST approved**, which is the deciding property. A digest
that cannot be named in a control document contributes nothing an auditor
may rely on, whatever its other merits — so this pair, and only this pair,
is kept.

> **BLAKE3 was here, and was removed (2026-07-27).** It was added for
> speed and for a third design family, and dropped once the FIPS question
> was put plainly: it is **not** NIST-approved, so it could never be the
> control of record in these services. It cost a column and a hash pass on
> every write while being unusable for the purpose the digests exist to
> serve. The columns were dropped rather than left unmaintained — a digest
> column nothing updates reads as coverage that does not exist.
>
> Losing it costs less than it appears. The **structural-diversity**
> argument survives intact, because Merkle–Damgård against sponge is
> exactly the pairing that argument wants: two unrelated constructions, so
> the kind of cryptanalytic advance that took MD5 and SHA-1 (both
> Merkle–Damgård) cannot take both. BLAKE3's ARX tree was a third family,
> but a third family you may not cite is not worth a column.
>
> What is genuinely lost is speed — BLAKE3 is much the fastest of the
> three — and the extendable-output property that would have made a
> 512-bit digest cheap if quantum margin were ever wanted. Neither is a
> compliance property; if either becomes the binding constraint, the
> decision is worth revisiting on its own terms.

**Why more than one at all, and the cost.** Holding two independent
digests is **algorithm agility**, and it has a deadline. A digest attests
only to the content hashed *at the time of writing*. If SHA-256 were
weakened in five years, re-hashing the existing history under a
replacement would prove nothing: it would compute digests from whatever
the rows contain *then*, certifying content that may already have been
altered — the same argument that forbids back-filling (above). The second
digest therefore has to be written **now**, or the option is gone forever.
That is why this is not deferred until a weakness appears.

The cost is honest: two passes over the pre-image on every write, and two
columns per hashed table. SHA-3 is the slower of the two in software,
lacking the dedicated CPU instructions SHA-256 enjoys.

Because both digests cover the same pre-image, they are built by one
`preimage()` function and hashed separately. Adding a third algorithm
changes only how the bytes are digested, never what is covered.

##### Quantum resistance

**These digests are quantum-resistant, and that is a property of the
choice to build integrity on hashes at all.** It is worth stating
positively, because it is the strongest post-quantum claim this service
can make and it holds today:

- **There is no Shor-style break for hash functions.** Shor's algorithm
  destroys the hardness assumptions behind RSA, Diffie-Hellman and
  elliptic curves — factoring and discrete logs — and nothing analogous
  applies to a well-constructed hash. A hash-based integrity control does
  not become *forgeable* on the day a cryptographically relevant quantum
  computer exists; it only loses margin.
- **Grover's algorithm costs a square root, not the whole thing.** It
  reduces preimage search on an *n*-bit digest from 2ⁿ to about 2^(n/2),
  so a 256-bit digest retains roughly **128-bit preimage security**
  against a quantum adversary. That is the level NIST's post-quantum
  category 1 is defined against (the cost of an exhaustive AES-128 key
  search), so both digests here remain in the range treated as adequate.
  Grover also parallelises poorly, which erodes the attack further in
  practice.
- **Collision resistance under quantum search is not the weak link
  either.** The BHT algorithm's ~2^(n/3) bound needs quantum-accessible
  memory at a scale nobody credible projects, and the consensus estimate
  for realistic models is far closer to the classical birthday bound.
- **Both inherit this equally.** SHA-256 and SHA3-256 are both 256-bit,
  so Grover treats them identically — the effect depends on output
  length, not internal design. Neither is more quantum-resistant than the
  other, and this spec does not claim otherwise; what it claims is that
  **both are quantum-resistant**, which is the useful statement.
  (Structural diversity buys resistance to *classical* cryptanalysis, not
  to Grover.)

**The 512-bit path, if it is ever wanted.** Neither of these is an
extendable-output function, so raising the digest length means adopting
SHA-512 (FIPS 180-4) or SHA3-512 (FIPS 202) — both approved, both a new
column and a new format version rather than a parameter change. That is a
real cost of dropping BLAKE3, whose XOF would have made it free; it is
also not a cost anyone should pay before a risk assessment asks for it.

> **Where the real post-quantum exposure in this system actually is.**
> Not here. The integrity controls described above are hash-based and
> therefore already post-quantum in the sense that matters. The
> **authentication** path is not: cross-service tokens are PASETO
> v4.public, signed with **Ed25519**, and Shor's algorithm breaks
> elliptic-curve signatures outright rather than merely halving their
> strength. A quantum adversary able to run Shor could forge tokens; it
> could not forge these digests. Any post-quantum programme for this
> family should therefore start at
> [`authentication-sessions.md`](../../agents/share/authentication-sessions.md),
> not at the audit chain. Recording that here is the point of writing
> this section down — a reader who takes "our digests are
> quantum-resistant" as their post-quantum answer has been pointed at
> the wrong subsystem.

The summary: **SHA-256 for conservatism and auditor familiarity, SHA-3
for FIPS standing without SHA-2's design lineage — two approved,
structurally unrelated constructions, so no single cryptanalytic result
takes both.**

##### What verification reports

Each report carries per-algorithm counters (`intact` / `sha3_intact`,
`unchained` / `sha3_unhashed`), and a tampered row is reported **once**,
naming which digests disagreed. That naming is diagnostic rather than
cosmetic:

- **both disagree** — the row's content changed.
- **exactly one disagrees** — the content is intact and a *digest column*
  was edited, or a write path stamped one digest and not the other. The
  second reading is the likelier one, and is why every write takes both
  digests from a single call that cannot express stamping only one.

Rows written before the second algorithm was adopted carry no SHA-3
digest. They are counted as `sha3_unhashed` — never as a mismatch, and
never as verified — exactly as a missing SHA-256 digest is treated.


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

#### The keyed MAC: a key the database never holds

The two digests above are **unkeyed**, and their pre-image format is
published in this section. An adversary who can write SQL can therefore
defeat them: edit the row, recompute both digests, update both columns.
What the digests actually detect is *careless or unaware* modification —
a bug, a manual fix, a restore from the wrong backup, an attacker who
does not know the columns exist.

An **HMAC-SHA256** (FIPS 198-1) over the same pre-image raises that bar to
a secret. The key lives in the service environment
(`<ENTITY>_INTEGRITY_MAC_KEY`) and is **never written to the database**,
so an adversary holding only the data — a stolen backup, a read replica, a
SQL-injection foothold, a DBA without application-server access — cannot
forge one. A unit test states the property directly: the unkeyed digests
are reproducible by anyone, the MAC is not.

**What it does not defend against, stated plainly.** An adversary holding
*both* the database and the service environment has the key and can forge
freely. This is defence against **database-only** compromise — the common
case, and worth having — not against full host compromise, which nothing
stored beside the data could resist. It is also void if the key is put
anywhere the database can reach: a config table, a connection string, a
`pgcrypto` call. **The separation is the control.** `pgcrypto` offers
`hmac()`, and using it would place the key exactly where the adversary
already is.

**Key identity and rotation.** A stored MAC is prefixed with its key id —
`k1:9f86d0…`. Without that, rotating the key would invalidate every
historical row at once, which is indistinguishable from mass tampering and
is the same trap as silently changing a hash format. Retired keys stay
available for verification (`<ENTITY>_INTEGRITY_MAC_KEYS_RETIRED`), so
rotation is additive rather than a flag day.

**Absent or unknown key.** No key configured means no MAC is written, and
those rows report `mac_absent`. A row naming a key this service does not
hold reports `mac_unverifiable` — **not** a mismatch, because "I cannot
check this" and "this is wrong" lead to different investigations, and
reporting a key-distribution problem as tampering would waste an incident
response. Only a MAC that recomputes to a *different* value is a finding.

**Operational notes.** The key must be at least 32 bytes; a shorter one is
refused rather than used, since a placeholder that reaches production
would produce MACs an attacker could reproduce by guessing. Verification
is constant-time (`Mac::verify_slice`) — a timing oracle would let an
attacker with write access recover a valid tag byte by byte without ever
holding the key. The key is never logged, only its length on rejection.
A missing or malformed key disables MAC writing and logs it rather than
blocking boot, matching the ABAC-policy and PASETO-key loaders; the
consequence is visible, because `mac_absent` then climbs in every report.

#### The external witness: checkpoints kept off-box

The MAC stops a row being **forged**. It says nothing about a row that is
simply **gone**, and neither does the chain: deletion from the *middle* of
a run breaks the successor's linkage, but deleting the **tail** leaves no
successor to break, so the shortened chain verifies perfectly. Delete
everything and it verifies vacuously. A DB-gated test states this
plainly — it empties `audit_logs`, confirms `/api/audit/verify` still reports
`verified: true`, and then catches the deletion by other means.

Truncation is invisible from inside the data. Detecting it needs
something the attacker cannot reach.

`GET /api/audit/checkpoint` returns a **checkpoint**: "at position
*N* the chain head was *H*, and *C* rows stood at or before *N*", MAC'd so
it cannot be rewritten by someone holding only the database. The operator
takes one periodically and stores it **outside this database**.

**Checkpoints are also emitted as `INFO` log lines on an
`audit_checkpoint` target, so a deployment already shipping logs has a
witness for free.** This is worth calling out separately because it is
the cheapest correct deployment of the control: no scheduler, no object
store, no second system to build or operate. If logs leave the host — to
a log aggregator, a SIEM, a retention bucket — then an off-box record of
the chain's state already exists, and honouring a checkpoint later is a
matter of pulling one line back out. A deployment that ships logs and
does nothing further still gets the deletion detection; one that stores
checkpoints only in this database gets none of it.

`POST /api/audit/checkpoint/verify` takes one back and answers
whether the chain still honours it:

| Verdict | Meaning |
|---|---|
| `honoured` | the anchor is present, unchanged, with at least as much history behind it |
| `anchor_missing` | **rows were deleted** — the witnessed row is gone |
| `head_changed` | the anchor survived but its content changed |
| `rows_deleted` | the anchor survived but history *behind* it shrank |
| `checkpoint_not_authentic` | the **witness** was altered; nothing is concluded about the chain |
| `checkpoint_unverifiable` | the checkpoint names a key this service cannot check |

The row count is carried for a specific reason: without it, an attacker
could delete history freely as long as they left the newest row alone.

**The storage is the control, not this code.** A checkpoint kept in this
database is worthless — an attacker who can delete audit rows can delete
the checkpoint in the same transaction, and its MAC prevents forgery, not
deletion, which is the entire problem. This service only makes the value
cheap to produce, cheap to compare, and unforgeable in transit. Where it
is kept is a deployment decision and the one that determines whether any
of this works.

**A tampered witness accuses itself.** The checkpoint's own MAC is checked
before the chain is consulted, and a failure reports
`checkpoint_not_authentic` rather than blaming the data. Without that
distinction an altered checkpoint would manufacture an apparent tampering
incident and send an investigation to the wrong subsystem.

#### Where the digests are computed: Rust, never the database

**Decision (2026-07-27): every digest is computed in the service, in
Rust. None is computed by Postgres.** Recorded here because the opposite
is an obvious-looking idea that would quietly destroy the control.

**Not for lack of database support.** Postgres can do both of these: a
core `sha256()` needs no extension at all, and `pgcrypto`'s
`digest(data, 'sha3-256')` works on PG 18 (OpenSSL-backed) — both
verified against this project's own database, not assumed. `pgcrypto` is
already installed in several of these schemas for `gen_random_uuid()`.
So this is a deliberate choice made *against* an available capability,
not a workaround for a missing one. Anyone who rediscovers that Postgres
can hash should read on rather than conclude the decision rested on a
false premise.

**The decisive reason: it would turn the database into a forgery oracle.**
These digests are **unkeyed**, and the pre-image format is published in
this very section. If Postgres computed them — in a trigger, or a
`GENERATED ALWAYS AS ... STORED` column — then *every* write would produce
a correct digest as a side effect, including a raw `UPDATE` from an
attacker with SQL access. Tamper detection would not merely weaken; it
would invert, because the mechanism meant to witness the change would be
driven by the change itself.

This is the same reasoning that removed the database audit triggers
(`m20260726_000003_drop_audit_triggers`): a witness that fires on the
attacker's own write attests to nothing. Having reached that conclusion
once, it would be strange to reintroduce the same defect one layer down.

Three further costs, in descending order:

1. **Byte-exactness across two implementations.** The pre-image would have
   to be rebuilt in SQL — `concat_ws(chr(31), …)` with identical
   NULL-versus-empty handling and identical JSON canonicalisation. It
   would not match: Postgres renders `jsonb::text` with spacing after
   `:` and `,` that `serde_json` does not emit. The divergence is silent
   and total, and the golden vectors exist precisely because this class
   of mismatch is invisible until something fails to verify.
2. **It would pin the digests to a Postgres version.** Any change to
   `jsonb` rendering or numeric formatting across a major upgrade would
   invalidate every stored digest — and the no-back-fill rule (above)
   means there would be no legitimate way to repair them.
3. **It would end offline verification.** The digest functions are pure
   Rust over a documented format, so a third party can recompute one
   without this service — the SHA-256 and SHA-3 golden vectors in
   `audit_chain.rs` were each cross-checked against an independent Python
   implementation. Computing in the database makes independent
   verification require a Postgres instance.

**What this decision does *not* fix, stated plainly.** An attacker with
SQL write access can still defeat the record hash today, because the
format is public and unkeyed: edit the row, recompute both digests,
update both columns. What the control actually buys is detection of
**careless or unaware** modification — a bug, a manual fix, a restore
from the wrong backup, an attacker who does not know the digest columns
exist. Two things would raise that bar, and neither involves moving
computation into the database:

- **An external witness** — **built, 2026-07-27**; see "The external
  witness" above.
- **A keyed digest (HMAC)** with a key the database does not hold —
  **built, 2026-07-27**; see "The keyed MAC" above. It is the *opposite*
  direction from database-side hashing, which is why that idea and this
  one cannot both be right.

**Where the database would genuinely help** is *verification* rather than
computation — a SQL-side bulk check would avoid loading every row into the
service. That remains open, and it inherits cost (1) in full: the SQL
would have to rebuild the pre-image byte-for-byte, so it would need the
same golden vectors before anyone could trust it.

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
