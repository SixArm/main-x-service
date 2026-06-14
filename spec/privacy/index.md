# Privacy & data protection

Monorepo-wide specification for privacy and data protection across the
Main X Index family of crates. This is the cross-cutting source of truth;
per-crate `spec.md §1–§18` and the brief
[`agents/share/privacy.md`](../../agents/share/privacy.md) defer to it.

It describes the privacy controls that are **actually implemented** in
the repository today, names the modules and endpoints that back them,
and is explicit about what is implemented versus deferred per service.

## Contents

1. [Data masking](#1-data-masking)
2. [GDPR data export (right of access, Art. 15)](#2-gdpr-data-export-right-of-access-art-15)
3. [GDPR erasure (right to erasure, Art. 17)](#3-gdpr-erasure-right-to-erasure-art-17)
4. [Consent management](#4-consent-management)
5. [Anti-enumeration as a privacy control](#5-anti-enumeration-as-a-privacy-control)
6. [What is and isn't personal data, per entity](#6-what-is-and-isnt-personal-data-per-entity)
7. [Implemented vs deferred](#7-implemented-vs-deferred)
8. [Compliance linkage](#8-compliance-linkage)

---

## 1. Data masking

The MPI ("master person index") services — **person**, **worker**,
**place** — ship a `src/privacy/mod.rs` module that returns a copy of a
record with sensitive field values redacted for display. Masking is a
**view-time** transformation: the stored record is never altered, and
the unmasked record remains available to authorized callers.

### 1.1 Masked-view endpoint

```
GET /<plural>/{id}/masked
```

Returns the record with sensitive values masked. Examples:
`GET /persons/{id}/masked`, `GET /workers/{id}/masked`,
`GET /places/{id}/masked`. Search results can also be masked inline via
the `mask_sensitive=true` query parameter on `GET /<plural>/search`.

### 1.2 Masked fields

Masking targets the values an operator does not need in full to
disambiguate a record. Names, the primary email local part, and
record IDs are left intact.

| Category | Masked field(s) | Notes |
|---|---|---|
| Tax / national ID | `tax_id`, identifiers of type `SSN` / `TAX` / `PPN` (passport) / `DL` (driver licence) | last 4 visible |
| Documents | every `IdentityDocument.number` | last 4 visible |
| Telecom | contact points with system `Phone` / `Sms` / `Fax` | last 4 visible |
| Email | masked by `mask_value` where applied | tail kept |
| Geo | coordinates (place / worker geo) | per `agents/share/privacy.md` |
| Postal address | address lines | per `agents/share/privacy.md` |

The implemented person masker is
[`mask_person`](../../person/person-service-rust-crate/src/privacy/mod.rs);
worker and place ship the analogous `mask_worker` / `mask_place`.

### 1.3 `mask_value` semantics

The core primitive is `mask_value(value, visible_chars)`. It keeps the
**last `visible_chars` characters** visible and rewrites the hidden
prefix:

- **Alphanumeric** characters in the hidden prefix become `*`.
- **Separators / punctuation** (e.g. hyphens, dots, `+`, `@`) pass
  through unchanged, so the value stays readable.
- Values no longer than `visible_chars` are returned **unchanged**.

| Input | `visible_chars` | Output |
|---|---|---|
| `123-45-6789` | 4 | `***-**-6789` |
| `AB12345` | 4 | `***2345` |
| `+1-555-123-4567` | 4 | `**-***-***-4567` (tail `4567`) |
| `short` | 10 | `short` (unchanged) |

**Char-based, not byte-based.** `mask_value` counts Unicode scalar
values (`char`s), never bytes, and never slices across a UTF-8 char
boundary. This was a real fix: the earlier byte-indexed implementation
panicked on multibyte input such as `"1é345"` (the naive `len - 4` byte
cut landed *inside* the two-byte `é`). The regression is pinned by
`test_mask_value_multibyte_does_not_panic`
([source](../../person/person-service-rust-crate/src/privacy/mod.rs)),
covering `"1é345"`, `"naïve12"`, `"café"`, and `"Müller-9981"`. The
practical consequence: accented names and non-Latin identifiers are
masked correctly and the endpoint cannot be crashed by exotic input.

---

## 2. GDPR data export (right of access, Art. 15)

Two export surfaces exist, both returning the data subject's own data
as JSON.

### 2.1 MPI entity export

```
GET /<plural>/{id}/export
```

Backed by `export_<entity>_data` (e.g.
[`export_person_data`](../../person/person-service-rust-crate/src/privacy/mod.rs)),
which serializes the full, **unmasked** record to JSON for the
data-subject right of access. The export is the complete record
(identifiers, names, addresses, contacts, documents, dates) — masking
is deliberately *not* applied here, because access is the subject's own
data.

### 2.2 Authentication account export

```
GET /api/auth/account/export   (bearer token required)
```

Implemented in
[`authentication/.../src/controllers/auth.rs`](../../authentication/authentication-service-rust-crate/src/controllers/auth.rs)
(`export_account` → `AccountExport`). It bundles three datasets for the
authenticated subject:

| Dataset | Contents |
|---|---|
| `users` | the account row |
| `sessions` | issuance / expiry / revocation timestamps + user agent |
| `auth_events` | the subject's audit trail, matched by `pid` **or** email |

**Never exported:** password hash, `api_key`, magic-link token, or any
key material. The passwordless `password` column (a hash of an
unguessable random value, present only to satisfy `NOT NULL`) is
excluded by construction. A GDPR-erased account is treated as gone and
returns `401` rather than the tombstoned record (see §3).

See [spec/authentication](../authentication/index.md) for the full
account lifecycle.

---

## 3. GDPR erasure (right to erasure, Art. 17)

Two distinct deletion semantics coexist; keeping them separate is the
point of this section.

### 3.1 Soft delete — reversible tombstone

Every MPI service uses **soft delete** for ordinary record deletion and
for the losing side of a merge: the record is marked inactive
(`active = false` / `deleted_at` stamped) but its data is retained. This
is a reversible tombstone — it preserves the audit trail and the merge
link graph, and the record can be restored. `DELETE /<plural>/{id}`
performs a soft delete, not a hard delete.

### 3.2 Anonymisation — irreversible

```
DELETE /api/auth/account   (bearer token required)
```

The authentication service implements true Art. 17 erasure
(`erase_account` in
[`auth.rs`](../../authentication/authentication-service-rust-crate/src/controllers/auth.rs),
`Model::erase` in
[`models/users.rs`](../../authentication/authentication-service-rust-crate/src/models/users.rs)).
It is a **soft-delete + anonymise + revoke + audit** sequence:

1. **Soft-delete** — stamp `users.deleted_at`.
2. **Anonymise** — overwrite `email` and `name` with a non-reversible
   placeholder (the row is kept so foreign-key / audit integrity holds,
   but the PII is destroyed).
3. **Revoke** — revoke all of the subject's sessions.
4. **Audit** — record an `account_erased` audit row.

After erasure the bearer token still *verifies cryptographically* until
its `exp` (stateless RS256; see
[spec/authentication](../authentication/index.md)), but `/me`, the
account export, and the audit routes all refuse it with `401` — a
deleted account is treated as gone. Re-erasing an already-erased account
is a no-op `200`.

| Aspect | Soft delete | Anonymisation |
|---|---|---|
| Reversible? | Yes (tombstone) | No |
| PII retained? | Yes | No (overwritten) |
| Row retained? | Yes | Yes (FK/audit integrity) |
| Where | every MPI service `DELETE` + merge loser | authentication `DELETE /api/auth/account` |

---

## 4. Consent management

The MPI services model GDPR/DPA consent as a first-class record
([`models/consent.rs`](../../person/person-service-rust-crate/src/models/consent.rs)).

### 4.1 Consent model

| Field | Type | Description |
|---|---|---|
| `id` | Uuid | consent record ID |
| `<entity>_id` | Uuid | subject ID (e.g. `person_id`) |
| `consent_type` | `ConsentType` | purpose category (below) |
| `status` | `ConsentStatus` | `Active` / `Revoked` / `Expired` |
| `granted_date` | Date | when granted |
| `expiry_date` | `Option<Date>` | when it expires (None = no expiry) |
| `revoked_date` | `Option<Date>` | when revoked |
| `purpose` | `Option<String>` | free-text purpose |
| `method` | `Option<String>` | how obtained (written / electronic) |

**`ConsentType`** — `DataProcessing`, `DataSharing`, `Marketing`,
`Research`, `EmergencyAccess`.

**`ConsentStatus`** — `Active`, `Revoked`, `Expired`.

### 4.2 `has_active_consent` checking utility

[`has_active_consent(consents, consent_type) -> bool`](../../person/person-service-rust-crate/src/privacy/mod.rs)
answers whether the subject has granted a given consent type that is
currently in force. A consent counts when, and only when:

- its `consent_type` matches the requested type, **and**
- its `status` is `Active`, **and**
- its `expiry_date` is absent **or** not in the past (compared against
  today in UTC).

Pinned by `test_consent_active_check` (active, far-future expiry →
`true`) and `test_consent_expired_check` (status `Active` but a
past `expiry_date` → `false`).

---

## 5. Anti-enumeration as a privacy control

Account enumeration is a privacy leak: confirming whether an email is
registered exposes membership of a dataset. The authentication service
treats anti-enumeration as a deliberate privacy control on its
passwordless surfaces:

- `POST /api/auth/signup` and `POST /api/auth/magic-link` **always
  return `200`**, whether or not the email belongs to an existing
  account. The unknown-email path is audited (as `unknown_email`) for
  security review, but the HTTP response is byte-identical to the
  known-account path.
- Magic-link consumption maps unknown / expired / already-consumed
  tokens all to the same `401`, and never logs the token itself.
- Rate limiting returns `429` keyed on request volume (not on account
  existence), so it does not perturb the always-`200` shape.

Full treatment lives in [spec/authentication](../authentication/index.md).

---

## 6. What is and isn't personal data, per entity

Privacy controls are scoped to where personal data actually lives. Not
every entity in the index is a data subject.

| Entity | Personal data? | Notes |
|---|---|---|
| **Person** | Yes — core | Names, DOB, tax/national IDs, documents, contacts, addresses. Full masking + export + consent. |
| **Worker** | Yes — core | Workforce/professional identities; same controls as person. |
| **Case** | Yes — subjects | Governmental case data *is* personal data (benefits, legal, social-services, complaints). |
| **Care pathway** | Mixed | Clinical pathway templates are largely reference data; linkage to patients is personal. |
| **Organization** | Largely public | Legal name, jurisdiction, registry IDs (LEI/DUNS/ROR) are public. **Contact fields** (telecom, contact address) are personal data. |
| **Place** | Largely public | Geographic places are public; contact / occupant fields are personal. |
| **Thing / Event / Course** | Largely non-personal | Asset / time-window / course-template registries; contact fields, where present, are personal. |
| **User (authentication)** | Yes | Email + name; minimal by design (passwordless). |

**case-folder note.** The `case-folder` app (NHS paper case-note folder
location tracking) deliberately stores **subjects as opaque IDs**, not
embedded PII. It records *where the folder for NHS Number X is right
now*, referencing the person / place / worker services by ID rather than
duplicating demographic data — the move audit trail therefore carries no
embedded personal data of its own.

---

## 7. Implemented vs deferred

Privacy maturity varies by service generation. The MPI services
(person / worker / place) carry the full privacy stack; the newer
loco.rs services defer parts of it. This is intentional and recorded
per-crate in `spec.md`.

| Service | Per-field masking | GDPR export | Consent model | Erasure (anonymise) |
|---|---|---|---|---|
| person | ✅ `mask_person` + `/masked` | ✅ `/export` | ✅ `Consent` + `has_active_consent` | soft delete |
| worker | ✅ `mask_worker` + `/masked` | ✅ `/export` | ✅ | soft delete |
| place | ✅ `mask_place` + `/masked` | ✅ `/export` | ✅ | soft delete |
| authentication | n/a (minimal data) | ✅ `/account/export` | n/a | ✅ true anonymise |
| organization | ⏳ **deferred** | ⏳ **deferred** | — | soft delete |
| care-pathway | ⏳ **deferred** | ⏳ **deferred** | — | soft delete |
| case | ⏳ **deferred** (per-field masking + GDPR export) | ⏳ **deferred** | — | soft delete |

The organization, care-pathway, and case `spec.md` overviews explicitly
list per-field privacy masking and GDPR export as **deferred** — they
ship CRUD + matching + audit + event streaming first. Case data is
personal data, so its masking/export gap is the most consequential of the
three and is flagged in its spec. When these are implemented they MUST
follow the `mask_value` / `/<plural>/{id}/masked` / `/<plural>/{id}/export`
shape defined in §1–§2 so the behaviour stays uniform across the index.

---

## 8. Compliance linkage

Privacy controls map onto the regulatory regimes the index targets:

- **GDPR / UK GDPR / DPA 2018** — Art. 15 right of access (§2), Art. 17
  right to erasure (§3), consent as a lawful basis (§4), data
  minimisation (§6), anti-enumeration (§5).
- **HIPAA** — masking of sensitive identifiers (§1) and the audit trail
  behind every access / change.

The audit trail that records *who accessed / changed what, when* — the
other half of compliance — is specified in
[`agents/share/auditability.md`](../../agents/share/auditability.md).
The regulatory regime lists are in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
(HIPAA, UK NHS Act s251, DPA, GDPR) and
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
(ISO/IEC 27001, ISO/IEC 42001, GDPR, DPA).

### See also

- [`agents/share/privacy.md`](../../agents/share/privacy.md) — the brief this spec expands
- [spec/authentication](../authentication/index.md) — account lifecycle, anti-enumeration, RS256/JWKS
- [spec/restful](../restful/index.md) — endpoint conventions (status codes, JSON envelope)
- [spec/postgresql](../postgresql/index.md) — persistence, soft-delete columns
- [`agents/share/auditability.md`](../../agents/share/auditability.md) — audit logging & event streaming
