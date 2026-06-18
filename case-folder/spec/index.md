# Case Tracking — Specification

Living, **single source of truth** for the Case Tracking project. This
root `spec/` directory holds the **cross-cutting** specification shared
by both editions; each subproject's `spec/` adds stack-specific detail
and links back here for anything domain-level.

> ⚠️ **Demo software.** Not a regulated medical record. The
> [regulatory considerations](regulatory.md) apply before any live use.

## What this project is

A medical case-file tracking system: a digital platform that monitors,
manages, and logs the lifecycle of **physical** paper case-note folders
in a UK NHS hospital setting. It answers one question fast — _"Where is
the paper folder for NHS Number `XXX XXX XXXX` right now?"_ — and keeps
an immutable audit log of every move.

Physical folders are tagged (barcode / QR / RFID). When a folder moves
between departments its tag is scanned, creating a digital trail of its
exact physical location. This cuts the time staff spend hunting for
missing records and prevents delays in patient care.

## Two editions

| Subproject                                                | Role      | Stack                        |
| --------------------------------------------------------- | --------- | ---------------------------- |
| [`case-folder-service-with-rust`](../case-folder-service-with-rust/spec/index.md)     | Back-end  | Loco / Axum / Rust JSON API  |
| [`case-folder-front-end-with-svelte`](../case-folder-front-end-with-svelte/spec/index.md) | Front-end | SvelteKit / TypeScript client |

Both editions serve the **same domain** (see [domain-model.md](domain-model.md))
with the same use cases, NHS Number rules, and invariants. The Loco
edition exposes a JSON API; the Svelte edition is a reference UI client.

## Specification (topic files)

| File                               | Covers                                                       |
| ---------------------------------- | ------------------------------------------------------------ |
| [purpose.md](purpose.md)           | Why this exists, the problem, who it serves                  |
| [scope.md](scope.md)               | In scope / out of scope for the project as a whole           |
| [domain-model.md](domain-model.md) | Entities, the five upstream services, invariants             |
| [places.md](places.md)             | Physical place hierarchy: campus → building → floor → room → cabinet/shelf |
| [volume.md](volume.md)             | What a volume is — a movable bundle of one patient's folders |
| [batch.md](batch.md)               | What a batch is — a transient bulk grouping across patients (proposed) |
| [tag-it.md](tag-it.md)             | Declare an interest in a folder, with desired dates (🚧 TODO / draft) |
| [receive-it.md](receive-it.md)     | Confirm receiving a case folder (🚧 TODO / draft)            |
| [scanners.md](scanners.md)         | Scanner codes & tags on folders — optical (barcode / QR) and wireless (RFID / NFC / BLE) |
| [auth.md](auth.md)                 | Email magic-link authentication (stateless signed tokens)    |
| [nhs-number.md](nhs-number.md)     | NHS Number Modulus 11 rules + worked examples                |
| [architecture.md](architecture.md) | How the two editions fit together                            |
| [testing.md](testing.md)           | Cross-cutting testing strategy + CI gates                    |
| [regulatory.md](regulatory.md)     | DCB0129/0160, DSPT, UK GDPR, Caldicott, WCAG, security gates  |
| [roadmap.md](roadmap.md)           | Combined roadmap across both editions                        |
| [glossary.md](glossary.md)         | Shared vocabulary                                            |

## Production-gate sketches (P0 — design only, not implemented)

Design sketches for the gates that must close before any live use. They are
**not built** in the demo; each says where it plugs into the existing code.

| File                                       | Covers                                                    |
| ------------------------------------------ | --------------------------------------------------------- |
| [rbac.md](rbac.md)                         | NHS CIS2/OIDC identity + role-based authorization (T-G1)  |
| [audit-integrity.md](audit-integrity.md)   | Append-only, hash-chained, signed audit log (T-G2)        |
| [deployment.md](deployment.md)             | Same-origin deployment, SSR re-enable, TLS/HSTS/CSP (T-G3) |

## Specification-driven delivery (SDD)

These three files drive delivery and are kept in lock-step with the
topic files above:

| File                               | Role                                                         |
| ---------------------------------- | ------------------------------------------------------------ |
| [requirements.md](requirements.md) | Numbered requirements + user stories + acceptance criteria   |
| [design.md](design.md)             | System design decisions that satisfy the requirements        |
| [tasks.md](tasks.md)               | Delivery checklist tracing tasks back to requirements        |

**Workflow:** a change starts in `requirements.md` (what + why), is
shaped in `design.md` (how), broken into `tasks.md` (delivery), then the
relevant topic files and the subproject specs are updated to match. No
code lands without the spec describing it.

## References

- [UK NHS Number — Wikipedia](https://en.wikipedia.org/wiki/NHS_number)
- [NHS Synthetic Data Service](https://digital.nhs.uk/services/synthetic-data)
- [GOV.UK Design System](https://design-system.service.gov.uk/)
- [Loco](https://loco.rs) · [SvelteKit](https://svelte.dev/docs/kit)
- The five upstream Main-X-Services live under `~/git/sixarm/main-x-service/`.
