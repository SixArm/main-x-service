<svelte:head><title>Architecture · Main X Index</title></svelte:head>

<h1>Architecture</h1>

<p>
    Every service boots on <strong>loco.rs</strong> (Axum + SeaORM +
    PostgreSQL); the differences are which internal shape a crate uses and
    which capabilities it carries.
</p>

<h2>The family at a glance</h2>

<ul>
    <li>
        <strong>Ten entity-registry services</strong> — person, worker,
        place, thing, event, course, organization, care-pathway, case,
        portfolio — each a CRUD + matching registry for one domain entity,
        embedding a sibling matcher crate.
    </li>
    <li>
        <strong>authentication-service</strong> — the central SSO provider
        (not a registry): passwordless magic-link login, Postgres cookie
        sessions, and PASETO v4.public token issuance with a published key
        set.
    </li>
    <li>
        <strong>link-graph-service</strong> — the read-model
        <strong>aggregator</strong> for cross-service links (read-only to
        the world; writes are event-driven).
    </li>
    <li>
        <strong>Library crates</strong> — the matcher crates (pairwise
        comparison), <code>authentication-verifier</code> (offline PASETO
        verification + the shared ABAC engine), and <code>entity-ref</code>
        (the cross-service <code>EntityRef</code> URN + edge-kind
        registry).
    </li>
    <li>
        <strong>SvelteKit front-ends</strong> — one operator SPA per
        entity.
    </li>
</ul>

<h2>Layered request flow</h2>

<pre>{`+------------------------------------------------------------------+
|  Clients (operator SPAs, peer services, EHR/analytics)           |
+---------------------------------+--------------------------------+
                                  |
+---------------------------------v--------------------------------+
|  API layer (Axum, mounted by loco)                               |
|   REST + OpenAPI/Swagger  ·  FHIR R5 (8 crates)  ·  gRPC (3)      |
|   blanket ABAC guard (<ENTITY>_REQUIRE_AUTH, default-off)         |
+---------------------------------+--------------------------------+
                                  |
+---------------------------------v--------------------------------+
|  Domain logic                                                    |
|   matching (embeds *-matcher)  ·  validation -> 422               |
|   duplicate detection + record merge  ·  privacy masking         |
|   event emit (CRUD + linked/unlinked)  ·  audit                  |
+---------------------------------+--------------------------------+
                                  |
        +-------------------------+-------------------------+
        |                         |                         |
+-------v--------+   +------------v-----------+  +----------v---------+
| PostgreSQL     |   | Tantivy full-text      |  | Event transport    |
| (SeaORM +      |   | index (all ten         |  | in-memory (default)|
|  migrations)   |   | registries)            |  | or Postgres outbox |
| entity rows,   |   |                        |  | -> relay -> Fluvio |
| audit, outbox, |   +------------------------+  +----------+---------+
| entity_links   |                                          |
+----------------+                          (link/created/... events)
                                                            |
                                          +-----------------v---------+
                                          | link-graph aggregator     |
                                          | (edges read-model,        |
                                          |  neighbors/single-view,   |
                                          |  reconcile)               |
                                          +---------------------------+`}</pre>

<h2>Two internal shapes</h2>

<p>
    Both boot identically through a loco <code>Hooks</code> impl in
    <code>src/app.rs</code>; they differ in how routes and persistence are
    organised.
</p>

<h3>person-style (<code>src/api/rest/</code>)</h3>

<p>
    The older hand-rolled Axum layout, now mounted under loco. Used by
    person, worker, course (and place / thing / event, mid-conversion). A
    rich domain model with per-field tables.
</p>

<h3>loco-style (<code>src/controllers/</code>)</h3>

<p>
    The newer loco-idiomatic layout. Used by organization, care-pathway,
    case, portfolio (and link-graph, authentication). The API DTO
    <strong>is</strong> the matcher type, stored verbatim as JSONB — no
    separate model to drift.
</p>

<h2>Cross-cutting subsystems</h2>

<ul>
    <li>
        <strong>Authentication &amp; authorization.</strong> Short-lived
        PASETO v4.public tokens from cookie sessions, verified offline via
        <code>authentication-verifier</code>; ABAC (attribute-based
        access control) with a default read-allow / mutation-deny policy.
    </li>
    <li>
        <strong>Event bus.</strong> Every CRUD/merge (and linked/unlinked)
        emits a canonical versioned envelope, transported per service by
        <code>&lt;ENTITY&gt;_EVENT_TRANSPORT</code> (default in-memory; a
        durable Postgres outbox + Fluvio relay is available family-wide).
    </li>
    <li>
        <strong>Cross-service linking.</strong> Each originating service
        records outbound edges in its own <code>entity_links</code> table
        and emits linked/unlinked events; the link-graph aggregator
        consumes the stream into a queryable read-model.
    </li>
</ul>

<p>
    For the full detail, see the repo's
    <a href="https://github.com/SixArm/main-x-service/blob/main/agents/share/architecture.md">architecture.md</a>
    and the rest of
    <a href="https://github.com/SixArm/main-x-service/tree/main/agents/share">agents/share/</a>.
</p>
