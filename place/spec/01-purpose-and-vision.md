## 1. Purpose and Vision

### 1.1 Purpose

The **Place entity** is the geographic place registry of the Main X
Index — a federated identity index serving a worldwide public
governmental system with millions of users. It maintains one
canonical record per real-world place modelled by
[schema.org/Place](https://schema.org/Place): government facilities,
service-delivery sites, civic structures, administrative areas,
business locations, and addresses.

The entity is delivered as a trio of subprojects:

| Subproject | Role |
|---|---|
| [place-service-rust-crate](../place-service-rust-crate/) | System of record — REST API, PostgreSQL persistence, Tantivy search, duplicate detection, merge, audit, privacy |
| [place-matcher-rust-crate](../place-matcher-rust-crate/) | Canonical pairwise-comparison algorithm — pure, deterministic, dependency-light library embedded by the service |
| [place-front-end-with-svelte](../place-front-end-with-svelte/) | Operator UI — SvelteKit SPA for CRUD / search / match / merge / audit |

### 1.2 Vision

One canonical place record at national and international scale,
regardless of how many source systems (national gazetteers, GLN
registries, OSM imports, agency CRMs, GIS feeds) hold a shard:

- A government clerk registering a new service-delivery site is warned
  in real time when the site already exists (409 + candidate matches).
- An auditor can trace every change to every place record — who, what,
  when, old and new values.
- A downstream agency resolves a place by GLN, FIPS, GNIS, or OSM ID
  and gets the same canonical record every peer gets.
- A resident's home address, where place data is personal data, is
  masked, exportable, and consent-tracked per GDPR.
- Operators in any of the project's supported locales
  ([`agents/share/locales.md`](../../agents/share/locales.md)) can run
  the registry; matching is diacritic-correct by construction.

### 1.3 Non-goals

- **Not a full GIS** — no tile serving, no routing, no map rendering.
  Places carry coordinates; cartography is a consumer concern.
- **Not a map-rendering service** — the front-end shows records and
  scores, not maps (map embeds are a roadmap idea, not a goal).
- **Not a geocoder** — reverse-geocoding is a service-crate roadmap
  item ([service spec §13 T-6](../place-service-rust-crate/spec/13-tasks.md)),
  not an entity commitment.
- **Not an address-validation authority** — the registry stores and
  normalises addresses; postal-reference lookups belong to integrated
  gazetteer authorities (roadmap, [§15](15-roadmap.md)).
