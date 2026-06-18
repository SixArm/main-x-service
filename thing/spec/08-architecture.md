## 8. Architecture

### 8.1 The trio

```
+--------------------------------------------------------------+
|                     Operators (browser)                       |
+------------------------------+-------------------------------+
                               |
+------------------------------v-------------------------------+
|  thing-front-end-with-svelte                                  |
|  SvelteKit 2 SPA · Svelte 5 runes · SVAR DataGrid · Lily      |
|  Own copy of types / ApiClient / ThingRepository              |
+------------------------------+-------------------------------+
                               |  REST (JSON envelope)
+------------------------------v-------------------------------+
|  thing-service-with-loco            loco.rs 0.16 / Axum 0.8  |
|  +----------------+ +----------------+ +-------------------+  |
|  | REST handlers  | | Validation +   | | Privacy + masking |  |
|  | 15 endpoints   | | normalisation  | | + GDPR export     |  |
|  +----------------+ +----------------+ +-------------------+  |
|  +----------------+ +----------------+ +-------------------+  |
|  | Search         | | Matching       | | Audit + events    |  |
|  | (Tantivy)      | | in-service     | | (in-memory bus)   |  |
|  +----------------+ | scorer         | +-------------------+  |
|                     |   +            |                        |
|                     | adapter.rs ----+--> thing-matcher 0.6.1 |
|                     +----------------+    (embedded library)  |
+------------------------------+-------------------------------+
                               |  SeaORM
+------------------------------v-------------------------------+
|  PostgreSQL — things + child tables + audit_log (§10)         |
+--------------------------------------------------------------+
```

### 8.2 Dependency direction

Strictly one-way; no cycles:

```
front-end ──REST──> service ──Cargo dep──> matcher
```

- The **matcher** depends on nothing in the trio (pure library: no
  IO, no async runtime, `#![forbid(unsafe_code)]`).
- The **service** embeds the matcher via
  [`src/matching/adapter.rs`](../thing-service-with-loco/src/matching/adapter.rs)
  (DTO contract, §5.3) and never the reverse.
- The **front-end** knows only the REST surface; it never links Rust
  code.

### 8.3 SSO integration (roadmap)

The central [authentication entity](../../authentication/) is the
single sign-on provider for the whole Main X Index: passwordless
magic-link, RS256 JWT issuance, JWKS for offline verification.
Planned wiring (service spec §13 T-4, front-end spec §15 v0.3):

- Front-end obtains a JWT from the authentication front-end / service.
- Thing service verifies tokens offline against the JWKS — no
  per-request call to the auth service.
- Roles: editor / read-only / service.

### 8.4 Deployment topology

**Today (single region):** one or more stateless service instances
behind a load balancer; PostgreSQL primary + replica; Tantivy index
local to each instance; front-end served as static SPA assets.

**Roadmap (worldwide governmental scale — see §15):** multi-region
active/active with PostgreSQL replication; durable event bus
replacing the in-memory publisher; search externalised so instances
share one index; per-region locale packs. These are aspirational
until the corresponding §13 / §15 items land — do not document them
as current behaviour.
