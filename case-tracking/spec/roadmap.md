# Roadmap (combined)

> Part of the [Case Tracking specification](index.md). Per-edition
> detail: [loco roadmap](../case-tracker-service-with-rust/spec/roadmap.md),
> [svelte roadmap](../case-tracker-front-end-with-svelte/spec/roadmap.md).

Priorities are shared across both editions unless noted.

| Priority | Item                                                                       | Edition       |
| -------- | -------------------------------------------------------------------------- | ------------- |
| P0       | Auth (CIS2 smartcard or OIDC) + RBAC, threaded through API + client        | both          |
| P0       | Append-only audit storage with chained signatures                          | loco          |
| P0       | Same-origin deployment + re-enable SSR                                     | svelte        |
| P1       | OpenAPI / JSON Schema document for every endpoint; codegen client types     | both          |
| P1       | Server-Sent Events on `/api/moves` so clients see new moves live           | both          |
| P1       | Barcode / RFID scan capture for moves                                      | svelte        |
| P2       | Soft-delete cabinets (refuse while occupied)                               | loco          |
| P2       | Per-cabinet QR code that opens the move workflow pre-filled                | svelte        |
| P2       | CSV / FHIR export of the audit log                                         | loco          |
| P3       | NHS Spine PDS lookup on folder registration                                | loco          |
| P3       | Service-worker offline mode for porters in basement archives               | svelte        |

See [requirements.md](requirements.md) for the requirements these items
would satisfy and [tasks.md](tasks.md) for active delivery.
