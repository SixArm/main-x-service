# Regulatory considerations & security gates

> Part of the [Loco edition specification](index.md). Shared frameworks
> + the full pre-production checklist: [root regulatory](../../spec/regulatory.md).

## Regulatory considerations

Identical to the Svelte sibling — DCB0129/0160, DSPT, UK GDPR,
Caldicott, WCAG 2.2 AA (where applicable to a back-end — input
validation, error messaging, audit completeness). The Loco edition adds
two operational concerns:

- **Database backups + PITR** for the audit log under NHS retention
  rules. The audit log itself lives in the Main Event Service, but PITR
  there is a deployment concern.
- **TLS termination** at the ingress (nginx / Azure App Service / AWS
  ALB). Loco serves plain HTTP — terminate TLS upstream.

## Security & privacy (production gates)

Same checklist as the root spec, plus Loco-specifics:

- [ ] Replace the dev `DATABASE_URL` credentials with secrets-manager values.
- [ ] Set `auto_migrate: false` in `config/production.yaml`.
- [ ] Set `dangerously_truncate: false` and `dangerously_recreate: false`.
- [ ] Use the `--release` profile.
- [ ] Run behind a reverse proxy that enforces HTTPS, HSTS, and rate limits.
- [ ] Wire CIS2 / OIDC via Loco's auth layer (or external middleware).
- [ ] Audit `move_events` writes to immutable chained storage on a worker.
