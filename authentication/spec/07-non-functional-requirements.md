## 7. Non-Functional Requirements

The auth entity is the **availability-critical hub** of a worldwide
governmental system: every other entity's sign-in depends on it, so
its targets sit above the family baseline
([`agents/share/availability.md`](../../agents/share/availability.md)).
Items marked *(roadmap)* are targets, not yet implemented — see §14 /
§15.

| # | Attribute | Target |
|---|---|---|
| NFR-1 | Scale | Millions of users (operators + citizens); sign-in bursts at population scale *(roadmap: load-tested)* |
| NFR-2 | Availability | Highest in the family — multi-instance, stateless app tier, PostgreSQL replication, multi-region *(roadmap)*. Mitigation built in: peers verify offline, so an auth outage blocks **new sign-ins only**, never in-flight request verification |
| NFR-3 | Token-validation latency at peers | Zero network — offline verification against the cached published key set (post-pivot: PASETO v4.public / Ed25519 at `/.well-known/paseto-keys`); sub-millisecond CPU-bound check per request |
| NFR-4 | Token lifetime | Short by design (post-pivot: ~5-min PASETO; was 3600 s JWT) to bound the revocation-staleness window of offline verification |
| NFR-5 | Issuance latency | Magic-link request ≤ 50 ms p50 excluding SMTP (email delivery is async best-effort; the console log is authoritative in dev) |
| NFR-6 | Key-rotation operability | `kid` stamped in every token header and JWK; verifier selects by `kid`, so old + new keys can serve side by side. Multi-key JWKS + grace window *(roadmap — §13 T-5)* |
| NFR-7 | Key custody | Asymmetric only — no shared secret ever leaves this service. Post-pivot: an **Ed25519** PASETO keypair (was RS256); the private key signs `POST /token`, only public keys are published. Production keys injected via files / `*_PEM` env; the committed `config/keys/*_dev.pem` pair is dev-only |
| NFR-8 | Abuse resistance | Anti-enumeration always-`200` is implemented. Rate limiting on magic-link issuance (per email / per IP) *(roadmap — §13 T-6)* |
| NFR-9 | Internationalisation | User-facing emails and UI localised per [`agents/share/locales.md`](../../agents/share/locales.md). **Implemented (T-7):** magic-link email (`src/i18n.rs`) + front-end UI (`src/lib/i18n.svelte.ts`) ship **English (`en`)** + **Welsh (`cy`)**, each a dependency-light per-locale catalog with `en` fallback; locale chosen per request (email: optional `locale` body field; UI: persisted switcher that also sets the request field). More locales add by extending the catalogs |
| NFR-10 | Observability | loco tracing with structured fields (email, magic-link issuance); OTLP traces / metrics aligned with the family stack *(roadmap: auth-specific metrics — issuance rate, redemption rate, verification failures)* |
| NFR-11 | Determinism in dev | Committed dev keypair keeps the JWKS stable across restarts; no SMTP needed (links logged) |
| NFR-12 | Verifier footprint | Dependency-light (post-pivot: a PASETO v4.public crate e.g. `rusty_paseto` in place of `jsonwebtoken`; `serde`, `thiserror`; `reqwest` optional behind `fetch`), `#![forbid(unsafe_code)]`, allocation-light `verify` safe to share behind an `Arc` |
