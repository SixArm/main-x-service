## 1. Purpose and Vision

### 1.1 Purpose

The **thing entity** is the generic thing / asset registry of the Main
X Index — a federated identity index serving a worldwide public
governmental system with millions of users. The entity is a trio of
subprojects that compose into one capability:

| Subproject | Role |
|---|---|
| [thing-service-with-loco](../thing-service-with-loco/) | System of record — REST CRUD, search, dedup, merge, audit, privacy |
| [thing-matcher-rust-crate](../thing-matcher-rust-crate/) | Canonical pairwise-comparison algorithm, embedded by the service |
| [thing-front-end-with-svelte](../thing-front-end-with-svelte/) | Operator UI over the service's REST API |

The domain model is faithful to
[schema.org/Thing](https://schema.org/Thing) with typed
[schema.org/PropertyValue](https://schema.org/PropertyValue)
identifiers (DOI, ISBN, ISSN, GTIN, SKU, MPN, SerialNumber, URI,
UUID, Custom). It is the most general entity in the family: public
assets, registered items, catalogued objects, publications, devices,
software — anything discrete that a government registry must identify
and deduplicate.

### 1.2 Vision

One canonical record per real-world asset, wherever no dedicated
entity index exists:

- A stable, cross-system Thing ID that agencies and downstream
  systems converge on, at population scale (millions of records,
  thousands of data sources).
- Deterministic convergence on globally-unique identifiers (DOI /
  ISBN / ISSN / GTIN / MPN / SerialNumber / UUID) and probabilistic
  convergence on names, descriptions, URLs, and `sameAs`
  cross-references.
- Every read and write auditable end-to-end (who / what / when) to
  the standard a public-sector deployment demands.
- Privacy-by-default for things linked to individuals — masking,
  consent, GDPR export — under GDPR, UK DPA 2018, and ISO/IEC
  27001 / 42001 controls.
- Operable in any locale the system serves (see
  [`agents/share/locales.md`](../../agents/share/locales.md)).

### 1.3 Non-goals

- **Not a substitute for a dedicated entity.** When a more
  opinionated sibling exists — person, worker, place, event, course,
  organization, care pathway — use it. The thing entity is the
  fallback for everything else.
- **Not an inventory or asset-management system** — identity, not
  stock, location, or custody chain.
- **Not a catalogue manager** — canonical properties, not marketing
  copy or pricing.
- **Not a blob store** — `image[]` holds URLs, not bytes (service
  spec §2.2, OQ-1).
