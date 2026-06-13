# Roadmap

> Part of the [Loco edition specification](index.md). Combined
> cross-edition roadmap: [root roadmap](../../spec/roadmap.md).

| Priority | Item                                                                |
| -------- | ------------------------------------------------------------------- |
| P0       | Auth (CIS2 smartcard or OIDC) + RBAC                                |
| P0       | Append-only audit storage with chained signatures                   |
| P0       | API versioning via `Accept` header mediatype (`application/vnd...`) |
| P1       | OpenAPI / JSON Schema document for every endpoint                   |
| P1       | Server-Sent Events on `/api/moves` so clients see new moves live    |
| P2       | Soft-delete cabinets (refuse while occupied)                        |
| P2       | CSV / FHIR export of the audit log                                  |
| P3       | NHS Spine PDS lookup on folder registration                         |
