# Authentication & authorization

The family stack unchanged
([authentication-sessions](../../agents/share/authentication-sessions.md),
[authorization-attributes](../../agents/share/authorization-attributes.md)):
cookie sessions + BFF for humans, offline PASETO v4.public for
services, blanket guard `CRM_REQUIRE_AUTH` (default **off** — the
family activation gate; activate before any real exposure), shared
ABAC engine.

## Personas as policy (not code)

One API; four typical policy personas expressed over `attrs`:

| Persona | Typical rules |
|---|---|
| **rep** | own contacts/leads/deals/activities (`resource.owner = $sub` ownership template); read team accounts |
| **sales manager** | team-wide pipeline + forecast + reassign, scoped by `resource.owner` / team attrs |
| **marketing** | campaigns, segments, nurture, consent views; read-only pipeline |
| **support** | tickets, KB, SLA views; read contacts/accounts |

## Record-level attributes

Handlers derive `resource.owner` (the owning worker URN, enabling
`$sub` self-scope), `resource.status`, and `resource.tier` for the
second ABAC pass. The `mask` obligation redacts **deal amounts,
forecast values, campaign costs/ROI, and contact channel details**
while leaving structure (stages, counts, statuses) visible.

## Sensitivity map

| Data | Tier |
|---|---|
| consent history, unsubscribe records | high — compliance evidence, append-only, reads audited |
| deal amounts / forecasts / campaign ROI | high — commercial; masked under `mask` |
| contact channel details & activity content | medium — personal data; masked under `mask` |
| pipelines, stages, SLA policies, published KB | low |
