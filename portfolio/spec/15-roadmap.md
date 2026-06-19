## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept. The near-term roadmap **is** §13 (the entity is spec-only,
§14); the items below are the longer arc beyond the initial trio
build-out.

- **Collaboration sub-resources.** The plan-family lineage carried
  posts, comments, and membership sub-resources; this entity ships
  goals / tasks / issues only (§2.3). Add posts / comments (Markdown
  update threads on a work item / task / issue) and members (a user's
  membership of a work item with a role) when collaboration becomes a
  requirement — each a new sub-resource table keyed by `(parent_kind,
  parent_pid)`, with membership-scoped write authorisation.
- **Family parity — match / search / merge.** Beyond the MVP
  baseline, reach the mature-sibling shape
  ([`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md)):
  Tantivy full-text + fuzzy search over the JSONB payload (per
  collection), search-blocked duplicate candidates (replacing the
  in-memory scan, OQ-2), batch deduplicate scan, and a front-end merge
  action.
- **Auditability — durable event bus.** The MVP ships an in-memory
  `WorkItemEvent` stream (T-5); replace it with the durable event bus
  ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)) so
  peer registries, analytics, and the cross-service link aggregator
  can subscribe across replicas. Work-item events are high-volume
  (every task write), so batched outbox emission matters.
- **Cross-service link aggregator.** Stand up (or join) the
  `link-graph-service`
  ([`agents/share/cross-service-linking.md` §4.3](../../agents/share/cross-service-linking.md))
  so a work item's `EntityRef`s and `entity_links` become a traversable
  graph (a work item's people → their orgs, related work items across
  departments). The portfolio trio ships only the write-side (T-7); the
  aggregator is a separate service.
- **Sub-resource bulk + linking.** Extend bulk import/export and the
  cross-service link write-side to the sub-resources (bulk-load tasks
  from a source PM tool; link a task to a `case` or a `thing`).
- **Security.** Blanket auth enforcement on `/api/*` (PASETO v4 public
  token / cookie session per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  superseding the RS256-JWT model); role-based write authorisation
  over sub-resources; role split between read-side integrators and
  work-item operators; rate limiting.
- **PM-tool sync.** Two-way sync with Jira / Asana / MS Project /
  Linear / GitHub Projects keyed on the deterministic external-id
  identifiers (R-0 schemes), so a registered project stays in step with
  its source-tool twin without becoming a full PM replacement (§8.7).
- **Localization.** Operator UI in the
  [`agents/share/locales.md`](../../agents/share/locales.md) locale
  set; multilingual work-item names via `alternate_names` +
  `in_language`; cross-language duplicate linkage through deterministic
  identifiers / `same_as`.
- **Scale-out and operations.** Multi-replica deployment, PostgreSQL
  replication, JSONB GIN + sub-resource indexing (OQ-3), OTLP
  observability pipeline, Prometheus metrics, backup / DR runbooks,
  container hardening.
- **gRPC.** Tonic stub once a high-throughput consumer exists. (The
  OpenAPI 3 doc + Swagger UI for the REST surface ship under T-9.)
