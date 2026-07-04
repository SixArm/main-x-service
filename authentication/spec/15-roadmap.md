## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept. Ordered roughly by the path from today's MVP to a
worldwide governmental deployment with millions of users.

- **Session + PASETO pivot (§13 T-12 — core landed).** The human
  session moved off JWT to a server-side `__Host-mxi_session` cookie
  session, and the cross-service credential to **PASETO v4.public**
  (Ed25519) minted by `POST /token` and published at
  `/.well-known/paseto-keys`; front-ends moved to the BFF
  pattern; RS256 + `/.well-known/jwks.json` decommissioned. Per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
  This **supersedes** the previous JWT model and reframes the
  enforcement rollout below. Remaining: full double-submit CSRF + the
  sessions-table reshape (§13 T-12a/T-12c).
- **PASETO enforcement rollout across peer services.** Embed the
  PASETO-mode `authentication-verifier` in every entity service (the
  loco conversion's next step), reject unauthenticated `/api/*`
  requests (valid session or valid PASETO per
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md)),
  and have sibling front-ends call entity services server-side (BFF)
  with a minted PASETO. The verifier crate exists precisely to make
  this a per-crate one-liner.
- **Key rotation automation.** Multi-key published key set
  (`paseto-keys`) with a grace window
  (§13 T-5 — delivered), scheduled rotation, secrets-manager
  integration, and a
  documented emergency-revocation runbook (key compromise = rotate +
  wait out the TTL).
- **Rate limiting and abuse resistance.** Per-email / per-IP issuance
  limits (§13 T-6), bot resistance on the public endpoints, anomaly
  alerting on issuance spikes.
- **OIDC compliance.** Publish `/.well-known/openid-configuration`,
  support the authorization-code flow with PKCE, and standard
  `id_token` claims — so commercial and governmental relying parties
  can integrate without the bespoke client.
- **WebAuthn / passkeys.** A second passwordless factor alongside
  magic links — phishing-resistant and offline-friendly for
  operator-grade assurance levels.
- **Localisation.** Magic-link / welcome emails and the front-end UI
  across the family's locales (§13 T-7,
  [`agents/share/locales.md`](../../agents/share/locales.md)),
  including RTL scripts (ar, fa, ur).
- **Multi-region deployment.** Active-active stateless app tier,
  PostgreSQL replication across regions, the `paseto-keys` key set
  served from a CDN edge
  (it is public, cacheable, and tiny), regional SMTP relays.
- **Auditability at governmental grade.** Auth event audit log +
  event streaming (§13 T-10), retention policy, auditor query API —
  aligning this entity with the family's
  [`auditability`](../../agents/share/auditability.md) baseline.
- **Account lifecycle.** GDPR export / erasure (§13 T-9), account
  recovery for lost email access, inactive-account policy.
- **Refresh tokens / session continuity.** Resolve §16 OQ-1; if
  adopted, rotate refresh tokens server-side while keeping access
  tokens short and offline-verifiable.
