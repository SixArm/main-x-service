# Deployment — production gate sketch

> Part of the [Case Tracking specification](index.md). **Design sketch for
> P0 production gates — not yet implemented.** Drives roadmap item **T-G3**
> (same-origin deployment + re-enable SSR) and the TLS/CSP/secrets gates in
> [regulatory.md](regulatory.md).

The demo runs the [Svelte client](../case-tracker-front-end-with-svelte/spec/index.md)
and the [Loco API](../case-tracker-service-with-rust/spec/index.md) as two processes,
stitched together in dev by a Vite proxy ([auth.md](auth.md)). Production needs
a single, hardened, same-origin deployment.

## Target topology — same origin behind one ingress

```
            ┌──────────────── ingress (TLS, HSTS, CSP, rate-limit) ───────────────┐
  client ──►│  /            → SvelteKit (adapter-node, SSR on)                     │
            │  /api/*       → Loco JSON API (:5150, plain HTTP, internal only)     │
            │  /healthz     → Loco                                                 │
            └──────────────────────────────────────────────────────────────────────┘
```

- **Same origin** removes the cross-site-cookie problem entirely: the
  HttpOnly `cts_session` cookie is first-party with no CORS exposure. The
  permissive dev CORS (`allow_origins: "*"`) is **removed** in production.
- **Re-enable SSR**: delete `export const ssr = false` from
  `+layout.ts` and swap `@sveltejs/adapter-auto` → **`@sveltejs/adapter-node`**.
  Loaders run server-side again; the API is reached over the internal network.
  (CSR-only was a deliberate dev simplification — see
  [svelte stack](../case-tracker-front-end-with-svelte/spec/stack.md).)
- **TLS terminates at the ingress** (nginx / Azure App Service / AWS ALB);
  Loco keeps serving plain HTTP internally ([loco regulatory](../case-tracker-service-with-rust/spec/regulatory.md)).
  Set **HSTS**.

## Hardening checklist (production gates)

These mirror the [security checklists](regulatory.md); this is the deployment
view of them.

- [ ] **Secrets** from a secrets manager — `AUTH_SECRET`, `DATABASE_URL`
      credentials, signing keys ([audit-integrity.md](audit-integrity.md)).
      Nothing in committed config.
- [ ] **Auth config** for production: `auth.cookie_secure: true`,
      `auth.expose_magic_link: false`, `auth.require_session: true`, and (when
      built) `auth.rbac.enforce: true` ([rbac.md](rbac.md)).
- [ ] **DB**: `auto_migrate: false`; migrations run from a controlled release
      step; `dangerously_truncate/recreate: false`.
- [ ] **CSP** that disallows inline scripts and restricts font/connect sources.
      Note the SVAR grid's `cdn.svar.dev` font preconnect needs IG review or
      self-hosting.
- [ ] **HTTPS + HSTS** at the ingress; redirect HTTP→HTTPS.
- [ ] **Rate-limiting / WAF** in front of `/api/*` and the auth endpoints.
- [ ] **Observability**: the OpenTelemetry/Prometheus wiring
      ([loco stack](../case-tracker-service-with-rust/spec/stack.md)) exported to the
      trust's monitoring; alert on auth failures and upstream `503`s.

## Build/release outline

1. `case-tracker-service-with-rust`: `cargo build --release`; run behind the ingress;
   migrations as a release step.
2. `case-tracker-front-end-with-svelte`: switch to `adapter-node`, `npm run build`, run
   the Node server behind the same ingress at `/`.
3. CI gates stay green (`cargo` + `npm run check/lint/test:unit/test:e2e`); add
   a deploy smoke check hitting `/healthz` and a signed-in `/api/stats`.

## Deliberately deferred

- Concrete hosting target (Azure App Service vs AKS vs on-prem) + IaC.
- CSP exact directives (pending the SVAR font decision).
- Zero-downtime release / blue-green specifics.
