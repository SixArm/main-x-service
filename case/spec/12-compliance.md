## 12. Compliance

The Main X Index targets worldwide public governmental systems, and
**case data is personal data**: a case concerns an identified or
identifiable person or organisation. Privacy and compliance therefore
matter *more* for this entity than for most siblings (a pathway or a
place definition is reference data; a case is about someone). Family
frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
(the latter applies to healthcare / social-care cases).

### 12.1 Data classification

Case records are **personal data**, and some are **special-category**
(health, social-services, immigration, criminal-matter cases). Three
facets:

- **The record itself is personal data.** Title, case type, status,
  agency, opened date, and the involved `subjects` together identify a
  matter about a person — even though `subjects` are opaque references,
  re-identification is trivial for the holding agency. This is GDPR /
  UK DPA personal data; some cases engage GDPR Art. 9 special
  categories.
- **Audit trails are personal data too.** Who created, edited,
  reviewed, or merged a record is personal data about operators; the
  audit log (delivered) MUST be governed accordingly.
- **Free text can leak.** `keywords` and `alternate_titles` MUST never
  carry substantive case content or personal detail; `subjects` carry
  only opaque ids. This is a shared invariant (§5.5) and an
  operator-training point.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| UK DPA 2018 / UK GDPR / EU GDPR | Fully engaged — case records are personal data. Lawful basis (usually public task / legal obligation), data-subject rights (access, rectification, erasure), and accountability documentation are mandatory. Soft delete supports retention policy; a GDPR-erasure path on top of it is required (§13 T-10). |
| US HIPAA | Engaged for healthcare / social-care cases that touch PHI; HIPAA-grade audit trails (delivered) and access controls required; soft delete preserves history. |
| UK Common Law Duty of Confidentiality | Engaged for cases holding confidential personal information (health, social care). |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, encryption at rest, backups, logging. |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants). |

### 12.3 Information-governance posture

- **Minimisation by design.** `subjects` are opaque references, not
  personal detail; the registry holds identity/routing metadata, not
  the case file (§1.3). This shrinks but does **not** remove the
  personal-data footprint — the record is still about a person.
- **Privacy controls are a priority gap.** Per-field masking (for the
  masked-view endpoint) and GDPR data-subject export are **not yet
  built** and are higher-priority here than for any sibling — tracked
  as §13 T-10 and roadmap §15. Until they land, masking/export are an
  honest gap (§14), and deployments must mitigate operationally
  (access control, need-to-know, data-protection impact assessment).
- **Auditability.** Delivered: soft delete + durable `audit_logs` row
  per create/update/delete/merge + in-memory event stream, per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
  The remaining gap is a durable cross-replica event bus (roadmap §15).
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, PASETO v4 public token verification —
  delivered for `whoami` / `actor`; *blanket `/api/*` enforcement is
  roadmap*; see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  supersedes RS256 JWT) and TLS; writes are restricted to caseworkers /
  registry operators.
- **Explainability for accountability.** Per-component match breakdowns
  give auditors a replayable rationale for every duplicate / merge
  decision — keep this property (NFR-9).

### 12.4 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Because a case
**is about a person** (§12.1), these land here about as hard as they do
on person itself — and harder than on any other sibling, given the
`case ↔ person` edge's elevated governance
([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§10).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.528 accounting of disclosures | Engaged for healthcare and social-care cases touching PHI. The decisive point: **learning that a case exists is itself the disclosure**, so a read must be audited, not just a write — and the `subject_of` edge's concealment rule (§10 of the linking doc) is exactly a §164.528 disclosure boundary. | **Read-auditing** on `get` / `list` / `search` / `check-duplicates` / `export` / FHIR reads and on **every traversal that surfaces a `subject_of` edge**, each row carrying purpose-of-use and a disclosure flag; **tamper-evident history** (a SHA-256 chain over `audit_logs`), which matters acutely because a case audit trail is potential legal evidence; and a per-record accounting of disclosures. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | **Fully engaged**, including Art. 9 special category for health, social-services, immigration, and criminal-matter cases. EHDS engages for the health and social-care subset, whose case data is exactly what its Ch. IV secondary-use pipeline would seek. | An **erasure path that survives immutable history** — redact the content, keep the chain linkage — which is the concrete shape of the §13 T-10 GDPR-erasure task, and which must extend to the `entity_links` rows and their `linked`/`unlinked` events, not just the case row; a declared **data residency** and **lawful basis** (usually public task or legal obligation); an `X-Purpose-Of-Use` marker; and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **Weak.** Case maps to `Task` — a best-effort mapping with no US Core profile ([`agents/share/fhir.md`](../../agents/share/fhir.md) §3). There is no certification target here. | The conformance *machinery* only: a declared `meta.profile`, structural validation, **terminology validation** of `status` / `intent` / `priority` against their bound value sets, and `$validate`. Bulk Data `$export` is available but, on this entity, is a **mass disclosure of personal data** and must inherit the §8 masking and audit rules of [`bulk-import-export.md`](../../agents/share/bulk-import-export.md) — plus the `subject` reference concealment. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device. The engagement is **supply-chain and configuration evidence**, plus one real harm: a false merge attaches one person's case history to another — a consequential error in benefits, immigration, or criminal-matter contexts. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, notably over the deterministic short-circuit rules and the record-level ABAC / masking-obligation paths, where a silent regression is a disclosure; and **signed, reproducible builds**. |

### 12.4b GDPR Art. 17 erasure (delivered 2026-07-26)

`POST /api/cases/{pid}/erase` destroys the case's personal data and
appends a chained `erased` accountability row. It is a **destructive**
action under ABAC ([`crate::auth::DESTRUCTIVE_POST_SUFFIXES`]), so it
requires `access=admin` — and it is **not** the soft delete: `DELETE
/{pid}` retires a record and keeps its data, this destroys the data and is
irreversible. The response says `irreversible: true` so a caller cannot
confuse the two.

The collision this resolves is real: honouring Art. 17 by deleting audit
rows would destroy the §164.312(c) integrity the chain exists to provide,
and refusing the erasure to protect the chain would breach Art. 17.
**Redaction** satisfies both — each audit row's snapshot is destroyed and
`redacted_at` stamped while its `hash` and `prev_hash` are left intact, so
verification still checks linkage across it and the chain as a whole keeps
verifying. What survives is the *fact* that a case existed and was erased,
by whom and when: the controller's own accountability record under the
Art. 17(3)(b) carve-out, holding nothing about the subject.

**The cross-service links are withdrawn too**, which is what makes this
meaningful for a case rather than merely correct. A `subject_of` edge
asserts that a named person is the subject of a benefits, legal, or
investigative proceeding — the family's highest-governance link
([`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§10). Tombstoning the payload while leaving that edge standing would erase
the details and keep the accusation, the opposite of what the subject
asked for. The links are **soft**-deleted rather than dropped: the link
aggregator reconciles against this table, and a row that vanishes without
trace is indistinguishable from one that was never written, which would
let a dropped event resurrect the edge.

Erasing an unknown or already-erased pid is answered, not refused. A
subject's right does not lapse once the record is soft-deleted — the audit
content held about it is still personal data — and a `404` would confirm
to a prober which pids are unknown.

DB-gated tests pin the load-bearing property: after an erasure the chain
still verifies and the redactions are *counted*, not hidden. If that ever
fails, the two obligations have stopped being simultaneously satisfiable
and the design is broken, not the test.

### 12.4z Hashing reference

Everything this service hashes, how each digest is built, and — the part
that matters when a report says something is wrong — what each one does
and does not prove. Every digest is computed under **both SHA-256 and
BLAKE3** over the same pre-image, rendered lowercase hex; see "Two
algorithms" below for why both are kept.

#### The digests

| Digest | Covers | Detects | Blind to |
|---|---|---|---|
| **Audit chain** (`audit_logs.hash` / `prev_hash`) | Every audit row, linked to its predecessor | Edits, insertions, deletions and reordering **in the trail** | Changes to case rows that write no audit row |
| **Record content hash** (`cases.content_hash`) | One case row's content and lifecycle state | Out-of-band SQL edits **to a record** | Deletion of a whole row; timestamp-only edits |

They are **complementary, and neither subsumes the other.** The chain
covers the *trail*; it cannot see an edit to an entity row, because a
change made without writing an audit row leaves the chain intact. The record hash covers the *rows*; it cannot see a row deleted outright in SQL, because a legitimate delete writes an audit row and an illegitimate one breaks the chain — which is the chain's job.

#### Two algorithms: SHA-256 and BLAKE3

Every digest above is computed **twice**, over a byte-identical
pre-image, and both results are stored:

| | Why it is kept |
|---|---|
| **SHA-256** | The conservative choice. FIPS 180-4, NIST-approved, two decades of cryptanalysis, and the digest a compliance reviewer expects to see named — some regimes name it explicitly. It is not going anywhere. |
| **BLAKE3** | Several times faster (SIMD, and a parallel tree structure that scales with cores), which matters because every write on a hot path pays for a digest. It is also an extendable-output function, so a longer digest is available later at the same speed. |

**The real reason for keeping both is algorithm agility**, and it has a
deadline. A stored digest can only ever attest to the content that was
hashed *at the time of writing*. If SHA-256 were weakened in five years,
re-hashing the existing history under a replacement would prove nothing:
it would compute digests from whatever the rows contain *then*, certifying
content that may already have been altered — the same argument that
forbids back-filling (above). The second digest therefore has to be
written **now**, alongside the first, or the option is gone forever. That
is why this is not deferred until a weakness appears.

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
- **Both algorithms inherit this equally.** SHA-256 and BLAKE3 are both
  256-bit, so Grover treats them identically — the effect depends on
  output length, not internal design. Neither is more quantum-resistant
  than the other here, and this spec does not claim otherwise; what it
  claims is that **both are quantum-resistant**, which is the useful
  statement.

**BLAKE3 is what makes the next step cheap.** If a future risk assessment
wants a higher NIST category rather than the current margin, BLAKE3 is an
extendable-output function: it emits a **512-bit** digest natively, at the
same speed, restoring ~256-bit preimage security under Grover. SHA-256
cannot be stretched — that path means adopting SHA-512 and a new column
and a new format version. Having BLAKE3 in place already turns a
migration into a parameter change.

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
> this section down — a reader who takes "we hash with BLAKE3" as their
> post-quantum answer has been pointed at the wrong subsystem.

The summary: **SHA-256 for conservatism and auditor familiarity, BLAKE3
for speed, a longer digest on demand, and the agility to survive a future
weakness in either.**

##### What verification reports

Each report carries per-algorithm counters (`intact` / `blake3_intact`,
`unchained` / `blake3_unhashed`), and a tampered row is reported **once**,
naming which digests disagreed. That naming is diagnostic rather than
cosmetic:

- **both disagree** — the row's content changed.
- **exactly one disagrees** — the content is intact and a *digest column*
  was edited, or a write path stamped one digest and not the other. The
  second reading is the likelier one, and is why every write takes both
  digests from a single call that cannot express stamping only one.

Rows written before the second algorithm was adopted carry no BLAKE3
digest. They are counted as `blake3_unhashed` — never as a mismatch, and
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

Verification (`GET /api/cases/audit/verify`) walks the trailing window and reports
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

**Version tag: `v1`.** Shared with the care-pathway service, whose chain format is identical — the tag names the *format*, not the crate.

#### The record content hash

Each `cases` row carries `content_hash`, recomputed on **every** write.

Fields bound, in order: **version, pid, title, data (JSON), active,
deleted_at (µs)**.

The whole payload is one JSONB `data` column, so hashing "the record" is
hashing one field — the relational services (person, worker) must assemble
theirs from child tables first.

**Excluded: `created_at` / `updated_at`.** The ORM and the database set
them, so binding them would make the digest depend on values the writer
does not control, producing mismatches on rows nobody touched. An attacker
who alters *only* a timestamp is not caught here; anything that changes
what the record says is.

**The failure mode is a false accusation, not a missed one.** A write path
that forgets to rehash flags an *untouched* record as tampered, which is
worse than having no control. Only `create` gets compiler help — the other
three (`update_data`, `soft_delete`, and the Art. 17 erasure) build their
`ActiveModel` from `..Default::default()` or an existing row and compile
happily with a stale digest. A DB-gated test therefore drives all four and
asserts every record still verifies, and was **confirmed to fail** when
the rehash is removed from the update path.

**Erasure recomputes the digest over the tombstone rather than clearing
it.** A case's whole payload is one column, so an erased record is still a
*complete* record and can be hashed — it keeps verifying instead of
dropping into the `unhashed` bucket. (person and worker null theirs
instead, because their child rows are deleted by then and no assembled
record remains to hash.)

**Version tag: `c-r1`.**

Verified by `GET /api/cases/records/verify`.

### 12.5 Honest limits

- **Masking and GDPR export are still not built** (§12.3), and that gap
  outranks everything in the table above. Read-auditing tells you a
  disclosure happened; masking is what stops the wrong one. Land §13
  T-10 first.
- **Not a certified health-IT module,** and not a candidate: `Task` is a
  best-effort mapping and the family serves **R5** against a
  certification targeting **R4 + US Core**.
- **Bulk export is the sharpest new risk.** Adding `$export` to an
  entity whose every row is personal data is only safe behind the
  masking profile, the elevated-authorisation gate, and the per-export
  audit that [`bulk-import-export.md`](../../agents/share/bulk-import-export.md)
  §8 requires. Do not ship it before those.
- **The audit half of the extended controls is implemented; the rest is
  not.** Delivered: the tamper-evident chain, read/disclosure auditing,
  `/audit/verify`, the §164.528 accounting, Art. 17 erasure by redaction
  including link withdrawal (§12.4b), and row-level record integrity
  (`cases.content_hash` + `/api/cases/records/verify`, 2026-07-27). Still absent:
  the GDPR residency and lawful-basis declarations, FHIR profile and
  terminology validation, and the SOUP/SBOM evidence bundle. The
  reference implementation remains the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  case is step 3 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
