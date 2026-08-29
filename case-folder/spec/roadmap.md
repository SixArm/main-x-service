# Roadmap (combined)

> Part of the [Case Tracking specification](index.md). Per-edition
> detail: [loco roadmap](../case-folder-service-with-rust/spec/roadmap.md),
> [svelte roadmap](../case-folder-front-end-with-svelte/spec/roadmap.md).

Priorities are shared across both editions unless noted.

| Priority | Item                                                                       | Edition       |
| -------- | -------------------------------------------------------------------------- | ------------- |
| P0       | Auth (CIS2 smartcard or OIDC) + ABAC, threaded through API + client        | both          |
| P0       | Append-only audit storage with chained signatures                          | loco          |
| P0       | Same-origin deployment + re-enable SSR                                     | svelte        |
| P1       | OpenAPI / JSON Schema document for every endpoint; codegen client types     | both          |
| P1       | Server-Sent Events on `/api/moves` so clients see new moves live           | both          |
| P1       | Barcode / QR / RFID / NFC scan capture for moves ([scanners.md](scanners.md)) | svelte        |
| P2       | Soft-delete cabinets (refuse while occupied)                               | loco          |
| P2       | Per-cabinet QR code that opens the move workflow pre-filled                | svelte        |
| P2       | CSV / FHIR export of the audit log                                         | loco          |
| P3       | NHS Spine PDS lookup on folder registration                                | loco          |
| P3       | Service-worker offline mode for porters in basement archives               | svelte        |
| P3       | BLE / RFID proximity & bulk reads — zone presence + batch reads ([scanners.md](scanners.md)) | both |

See [requirements.md](requirements.md) for the requirements these items
would satisfy and [tasks.md](tasks.md) for active delivery.

## Idea stage — not designed, not queued

These three topic files sketch a possible capability each, but none has
a `requirements.md` entry, a `design.md` decision, or a `tasks.md` row —
so, unlike the table above, they are **not** committed roadmap items.
Each file's own "Open questions" section is the reason: the data model
and lifecycle are still undecided. They stay linked from
[index.md](index.md) as parked ideas, not silently dropped, but picking
one up starts at `requirements.md` per the SDD workflow above, not at
its sketch file.

| File | Idea |
| --- | --- |
| [tag-it.md](tag-it.md) | Declare an interest in a folder, with desired dates |
| [receive-it.md](receive-it.md) | Confirm receiving a case folder, closing the loop on a move |
| [batch.md](batch.md) | Transient multi-patient bulk grouping for a single physical handling action |
