# Runbook: activating the integrity controls

**The shipped default is inert.** A deployment that sets nothing gets no
authentication, no read auditing, no keyed MAC, and no external witness —
the audit chain and record digests are written, and nothing else in this
document is running. That is deliberate (it lets the family ship and
integrate before a deployment's policy exists) and it is dangerous
exactly once: on the day the service becomes reachable by anyone you do
not trust.

This runbook covers the **integrity and audit** controls
(`spec/12-compliance.md` §12.4z in each entity spec). Auth activation
itself is [`jwt-enforcement.md`](../jwt-enforcement.md); this is the
partial OPS-1 slice for integrity, and the other OPS-1 runbooks — key
rotation for PASETO, reconciliation divergence, event-bus replay, bulk
recovery — are still unwritten.

Applies to **care-pathway, case, person, worker** — the four services
carrying the chain. Substitute the entity prefix throughout:
`CARE_PATHWAY_`, `CASE_`, `PERSON_`, `WORKER_`.

## What you get if you do nothing

| Control | Default | Consequence |
|---|---|---|
| `<E>_REQUIRE_AUTH` | **off** | `/api/*` and `/fhir/*` are open; audit endpoints included |
| `<E>_AUDIT_READS` | **off** | reads are not recorded; the §164.528 accounting is empty and says so |
| `<E>_AUDIT_FAIL_CLOSED` | **off** | a failed audit write lets the read proceed; the chain shows a gap indistinguishable from a deletion |
| `<E>_INTEGRITY_MAC_KEY` | **unset** | no keyed MAC: anyone with SQL write access can forge the digests, since their format is published |
| checkpoint storage | **undefined** | wholesale deletion of the trail is undetectable |
| `<E>_EVENT_TRANSPORT` | `memory` | events are lost on restart |

The audit chain and record digests **are** written by default. They
detect careless modification. They do not detect a determined actor, and
they never detect deletion of the tail.

## The endpoints, per service

They are **not** at the same paths — care-pathway groups them under
`/api/compliance`, case under its entity prefix, and person/worker at the
API root. Substitute from this table wherever the text says
`<checkpoint endpoint>` or `<verify endpoint>`.

| | care-pathway | case | person / worker |
|---|---|---|---|
| chain verify | `/api/compliance/audit/verify` | `/api/cases/audit/verify` | `/api/audit/verify` |
| record verify | `/api/compliance/records/verify` | `/api/cases/records/verify` | `/api/records/verify` |
| take checkpoint | `/api/compliance/checkpoint` | `/api/cases/checkpoint` | `/api/audit/checkpoint` |
| verify checkpoint | `/api/compliance/checkpoint/verify` | `/api/cases/checkpoint/verify` | `/api/audit/checkpoint/verify` |
| disclosure accounting | — | `/api/cases/{pid}/audit/disclosures` | `/api/<plural>/{id}/audit/disclosures` |

## Activation order, and why it is an order

1. **`<E>_REQUIRE_AUTH=1` + an ABAC policy.** First, because everything
   below writes or exposes audit data, and an open `/audit/*` surface
   hands an attacker the very trail they would want to inspect before
   editing. Policy via `<E>_ABAC_POLICY` (inline JSON) or
   `<E>_ABAC_POLICY_FILE`.
2. **`<E>_INTEGRITY_MAC_KEY`** — 32+ bytes, hex, from your secret
   manager. Set this **before** read auditing, so the rows read auditing
   generates are covered from the first one. Rows written before the key
   exists are permanently unMACed: the digest cannot be added later
   without certifying whatever the row contains at that point, which is
   the claim it exists to test.
3. **`<E>_AUDIT_READS=1`.** Now reads are recorded, and each row is MACed.
4. **`<E>_AUDIT_FAIL_CLOSED=1`** once you are confident the audit write
   path is healthy. This turns a failed audit write into `503` rather
   than a silent gap. Enable it *after* step 3 has run cleanly for a
   while — enabling it first turns any audit hiccup into an outage.
5. **Take and store the first checkpoint** (below). Nothing before this
   step detects deletion.
6. **`<E>_EVENT_TRANSPORT=outbox`** if you need durable events.

Steps 1–4 are per-service environment; each requires a restart, because
all four are read once at boot.

## Verifying each step actually took effect

Do not assume a variable was read. Each control has an observable:

| Step | Check | Expected |
|---|---|---|
| auth | `curl -i /api/<plural>` with no token | `401` |
| MAC key | `GET <verify endpoint>` | `mac_valid` climbing, `mac_absent` flat |
| read auditing | `GET /api/<plural>/{id}` then the accounting endpoint | `read_auditing_enabled: true`, caveat says "complete" |
| fail-closed | (see below) | a forced audit failure returns `503`, not `200` |
| checkpoint | `GET <checkpoint endpoint>` | a body with a non-empty `head` and a non-null `mac` |

**`mac_absent` not falling is the most common misconfiguration** — it
means the key never loaded. Causes, in order of likelihood: not hex; under
32 bytes; set in the wrong process; set after boot. The service logs the
reason at `ERROR` on startup and never logs the key itself.

## Storing checkpoints — the part that is a deployment decision

A checkpoint kept in the service's own database is **worthless**: whoever
can delete audit rows can delete it in the same transaction, and its MAC
prevents forgery, not deletion.

Cheapest correct option: **do nothing extra.** Checkpoints are emitted as
`INFO` log lines on the `audit_checkpoint` target, so if logs already
leave the host — aggregator, SIEM, retention bucket — an off-box record
exists. Confirm your log pipeline retains that target for at least your
audit-retention period.

If you want an explicit copy, poll `GET <checkpoint endpoint>` on a
schedule (daily is ample; the anchor moves with every audit write) and
store the JSON anywhere the service's database credentials cannot reach.

## Symptoms → checks → actions

**"Verification reports a content break."**
Check *which* digests disagree — the report names them. All of them means
the content changed. Exactly one means that digest column was edited, or
a write path stamped some digests and not others; the second is a bug in
this service, not an attack. Check `mac_valid` for the same window: a
break with the MAC still valid is very unlikely to be an outside actor,
because forging it needs the key.

**"`mac_unverifiable` is climbing."**
Rows name a key id this process does not hold. This is a key-distribution
problem, not tampering — do not open an incident on the data. Add the
retired key to `<E>_INTEGRITY_MAC_KEYS_RETIRED` as `id:hex` and restart.

**"A checkpoint is not honoured."**
Read the verdict before acting:
- `anchor_missing` / `rows_deleted` — audit rows were deleted. Preserve
  the database as-is, stop writes if you can, and work from the
  off-box checkpoints to bound when it happened.
- `head_changed` — the anchor row's content changed.
- `checkpoint_not_authentic` — the **witness** was altered, not
  necessarily the data. Investigate wherever checkpoints are stored
  before concluding anything about the chain.
- `checkpoint_unverifiable` — this process cannot check that checkpoint's
  MAC; treat as the key-distribution case above.

**"`unchained` is non-zero."**
Rows predating the chain, or written by something that does not chain.
Expected on a table that existed before the chain landed. If it is
climbing *now*, something is writing audit rows outside the repository —
the database audit triggers were removed for exactly this reason
(`m20260726_000003_drop_audit_triggers`).

## Rotating the MAC key

1. Generate a new key; give it a new id.
2. Move the current key into `<E>_INTEGRITY_MAC_KEYS_RETIRED` as
   `oldid:hex` (comma-separated for several).
3. Set `<E>_INTEGRITY_MAC_KEY` to the new key and
   `<E>_INTEGRITY_MAC_KEY_ID` to its id. Restart.
4. Verify: `mac_valid` still climbing, `mac_unverifiable` flat. Old rows
   verify under the retired key; new rows carry the new id.

Never remove a retired key while rows still name it — that converts
verifiable history into `mac_unverifiable`. There is no way to re-MAC old
rows honestly, for the same reason there is no way to back-fill a digest.
