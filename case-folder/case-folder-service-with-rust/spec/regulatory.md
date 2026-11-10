# Regulatory considerations & security gates

> Part of the [Axum edition specification](index.md). Shared frameworks
> + the full pre-production checklist: [root regulatory](../../spec/regulatory.md).

## Regulatory considerations

Identical to the Svelte sibling — DCB0129/0160, DSPT, UK GDPR,
Caldicott, WCAG 2.2 AA (where applicable to a back-end — input
validation, error messaging, audit completeness). The Axum edition adds
two operational concerns:

- **Database backups + PITR** for the audit log under NHS retention
  rules. The audit log itself lives in the Main Event Service, but PITR
  there is a deployment concern.
- **TLS termination** at the ingress (nginx / Azure App Service / AWS
  ALB). axa serves plain HTTP — terminate TLS upstream.

## Security & privacy (production gates)

Same checklist as the root spec, plus axa-specifics:

- [ ] Set `AUTH_SECRET` from a secrets manager (the service has no database, so no `DB_URL`).
- [ ] Use the `--release` profile.
- [ ] Run behind a reverse proxy that enforces HTTPS, HSTS, and rate limits.
- [ ] Wire CIS2 / OIDC via axa's auth layer (or external middleware).
- [ ] Audit `move_events` writes to immutable chained storage on a worker.
