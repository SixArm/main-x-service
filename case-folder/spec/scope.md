# Scope

> Part of the [Case Tracking specification](index.md). Each subproject
> narrows this further: see
> [loco scope](../case-folder-service-with-rust/spec/scope.md) and
> [svelte scope](../case-folder-front-end-with-svelte/spec/scope.md).

## In scope

- Tracking **physical** paper case-note folders by **NHS Number** across
  a hierarchy of buildings → rooms → cabinets.
- Folder lifecycle: register, search, show, per-folder move history.
- Recording **moves** as an append-only audit log, updating each
  folder's current location.
- Managing the place hierarchy (buildings, rooms, cabinets).
- Server-side and client-side **NHS Number validation** (Modulus 11).
- Aggregate views: cabinet utilisation, folders in transit, recent
  activity, KPIs.
- A JSON API (Loco edition) and a reference browser UI (Svelte edition).
- **Passwordless authentication** via email magic link (stateless signed
  tokens) — see [auth.md](auth.md). Identity comes from a configured
  allowlist; CIS2/OIDC + ABAC remain production gates.

## Out of scope

- **Clinical content.** The system stores no medical notes — only the
  location of the paper that holds them.
- **ABAC, SSO, smartcard / CIS2.** Authentication exists (magic link)
  but attribute-based authorization and federated identity are production
  gates (see [regulatory.md](regulatory.md)).
- **Patient registration as a first-class flow.** Patients are owned by
  the Main Patient Service; registration is a side effect of creating a
  folder, not a standalone endpoint/page.
- **Local persistence of domain data.** All domain data lives in the
  five upstream services (see [domain-model.md](domain-model.md)); the
  tracker owns no tables and the UI keeps no `localStorage`/IndexedDB.
- **Real-time websockets / SSE, multi-tenancy, background workers,
  mailers, queues.**
- **Anything that would push the system toward SaMD classification.**

## Boundary rationale

The deliberate narrowness keeps the system a **pure location tracker and
aggregator**. Every entity of clinical or organisational significance is
owned by an upstream service; the tracker only records *where the paper
is* and *who moved it when*. This boundary is what keeps the regulatory
surface small — see [regulatory.md](regulatory.md).
