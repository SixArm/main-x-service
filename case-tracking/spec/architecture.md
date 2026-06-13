# Architecture (project-level)

> Part of the [Case Tracking specification](index.md). Edition-specific
> internals live in
> [loco architecture](../case-tracker-service-with-rust/spec/architecture.md)
> and [svelte architecture](../case-tracker-front-end-with-svelte/spec/architecture.md).

## Two editions, one domain

```
┌───────────────────────────┐        HTTP / JSON        ┌────────────────────────────┐
│  case-tracker-front-end-with-svelte │  ───────────────────────> │  case-tracker-service-with-rust    │
│  SvelteKit browser client  │   GET/POST /api/*         │  Loco/Axum JSON API         │
│  (owns no data)            │ <───────────────────────  │  (owns no domain tables)    │
└───────────────────────────┘                           └─────────────┬──────────────┘
                                                                       │ proxies + snapshots
                                                                       ▼
                                   ┌───────────────────────────────────────────────────┐
                                   │  Five upstream "Main-X-Service" HTTP services        │
                                   │  Patient · Place · Worker · Thing · Event            │
                                   └───────────────────────────────────────────────────┘
```

- The **Loco edition** is the contract. It defines the JSON API, the
  validation, and the upstream proxying. It is back-end only — no UI.
- The **Svelte edition** is a reference client. It owns no data; every
  page fetches from `/api/*`, hydrates a reactive cache, and renders.
- The **five upstream services** own every domain entity (see
  [domain-model.md](domain-model.md)). The tracker reads them and writes
  only folders (Thing) and move events (Event), with denormalised
  snapshots so audit data survives upstream outages.

## Source-of-truth direction

Changes flow **API-first**:

1. A new capability is specified at the root (this `spec/`) if it is
   cross-cutting, then in the **Loco** subproject spec (the wire
   contract).
2. The Loco edition implements and tests it.
3. The Svelte edition consumes it via its typed API client.

The Svelte edition never invents domain data or shapes the API can't
provide; if it needs new API surface, that surface is added to the Loco
spec first.

## Coupling rules

- Cross-service references are **opaque UUIDs**; no service enforces
  referential integrity against another.
- The only durable reconciliation between services is the **snapshots**
  the tracker writes at action time.
- The two editions communicate **only** over HTTP/JSON. They share no
  code, no database, no in-process state.

## Deployment shape (target)

For production, both editions sit behind **one ingress / same origin**
so cross-site cookies and CORS aren't a vector, and TLS terminates at
the proxy. See [regulatory.md](regulatory.md) for the full gate list.
