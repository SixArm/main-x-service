# Audit integrity — production gate sketch

> Part of the [Case Tracking specification](index.md). **Design sketch for
> a P0 production gate — not yet implemented.** Drives roadmap item **T-G2**
> (append-only audit storage with chained signatures).

The move-event audit log is the system's evidentiary record (chain of custody
for paper case notes). Today it lives in the **Main Event Service** and is
*append-only by convention* ([invariant 3](domain-model.md)) — there is no
cryptographic guarantee that a recorded move wasn't later altered or deleted,
and no proof of completeness. For live use under NHS retention and
information-governance expectations, the audit trail must be **tamper-evident**.

## Threat model

- An actor with database/API access **edits** a past move (e.g. to hide a
  mis-file) or **deletes** one to break the chain of custody.
- A move is **back-dated** or inserted out of order.
- We need to *detect* any of the above after the fact, and ideally prove
  completeness (no silent gaps).

## Design — a hash-chained, signed event log

Each move event gains three fields, computed at write time and never updated:

| Field        | Meaning                                                              |
| ------------ | ------------------------------------------------------------------- |
| `seq`        | Monotonic per-log sequence number (gaps are detectable).            |
| `prev_hash`  | Hash of the previous event's `hash` (genesis = all-zero).           |
| `hash`       | `H(seq ‖ prev_hash ‖ canonical(payload) ‖ recorded_at)`.            |
| `signature`  | Detached signature over `hash` by a service signing key.            |

```
event[n].hash = H( seq[n] ‖ event[n-1].hash ‖ canonical(payload[n]) ‖ ts[n] )
event[n].signature = Sign(sk, event[n].hash)
```

- **Append-only:** the writer only ever appends; `UPDATE`/`DELETE` on the log
  table are revoked at the database-role level. A nightly job copies the log
  to **write-once (WORM) / object-lock** storage.
- **Verification:** a verifier walks the log, recomputing each `hash` from the
  previous one and checking `signature`. Any edit, deletion, or reordering
  breaks the chain at that point; a missing `seq` proves a gap.
- **Signing key:** an asymmetric key in an HSM / cloud KMS; the public key is
  published so auditors can verify independently. Keys are rotated; rotations
  are themselves logged.
- **Anchoring (optional):** periodically publish the latest `hash` to an
  external notary / timestamping authority so even a full-log rewrite is
  detectable.

## Where it plugs in

- It belongs in the **Main Event Service** (the system of record), not the
  tracker — the tracker stays a pure aggregator ([D-1](design.md)). The
  tracker's `record()` call is unchanged; the chaining happens server-side on
  write.
- A tracker-side **verification endpoint** (e.g. `GET /api/audit/verify`)
  could surface the chain status (`ok` / first broken `seq`) for an IG
  dashboard, derived like the other read views ([D-10](design.md)).

## Operational concerns

- **Backups + PITR** for the audit store under NHS retention rules; restores
  must preserve the chain.
- **Retention**: case-note audit retention can be decades — size the WORM tier
  accordingly; never prune within retention.
- **Clock**: `recorded_at` from a trusted, monotonic source; don't trust
  client clocks.

## Deliberately deferred

- Choice of signature scheme + KMS/HSM vendor.
- WORM/object-lock storage product and retention tiers.
- External anchoring/notary integration.

## Acceptance (when implemented)

- Tampering with any stored event (edit/delete/reorder) makes
  `verify` report the first broken `seq`.
- The public key verifies every `signature` end-to-end.
- `UPDATE`/`DELETE` on the audit table are denied to the application role.
