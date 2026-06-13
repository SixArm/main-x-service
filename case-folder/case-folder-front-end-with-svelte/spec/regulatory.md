# Regulatory considerations & security

> Part of the [Svelte edition specification](index.md). Shared
> frameworks + the full pre-production checklist:
> [root regulatory](../../spec/regulatory.md).

## Regulatory considerations

Same as the API sibling (see
[Loco regulatory](../../case-folder-service-with-rust/spec/regulatory.md)) plus
a client-side angle:

- **DCB0129 / DCB0160** clinical risk management.
- **WCAG 2.2 AA** accessibility (Public Sector Bodies Accessibility
  Regulations 2018). The client is the surface a clinician touches — axe
  scans in CI are TODO (see [accessibility.md](accessibility.md)).
- **No PII in the browser cache beyond session lifetime.** No
  `localStorage` of NHS Numbers, no IndexedDB. The reactive cache
  evaporates on reload.

## Security & privacy

Before deploying anywhere reachable from patient data:

- [ ] Same-origin deployment (front-end and API behind one ingress) so
      CORS / cross-site cookies aren't a vector.
- [ ] Re-enable SSR once same-origin is the case (delete `ssr = false`
      from `+layout.ts`).
- [ ] SSO via NHS CIS2 or Azure AD; pass auth headers through the `api.*`
      client.
- [ ] CSP that disallows inline scripts and restricts font sources
      (SVAR Grid's `cdn.svar.dev` font preconnect needs IG review).
- [ ] Per-user attribution on `movedBy` from the auth context, not a
      free-text input.
- [ ] Audit log retention / chained signatures handled by the API side.
