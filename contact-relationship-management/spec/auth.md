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

## Subject rights & retention (CRM-R20, the code side of CRM-G2)

`GET /api/contacts/{pid}/subject-access` (one audited export across
every table keyed to the contact, exclusions named in the payload),
`POST /api/contacts/{pid}/erase` (anonymise; refused `422` while the
contact holds an open deal, an open ticket, or an active nurture
enrolment — CRM-D14), and `GET /api/retention` /
`POST /api/retention/sweep` (the floored-horizon report and sweep,
`CRM_RETENTION_DAYS`, default 365, floor 30). `/erase` and `/sweep`
join [`DESTRUCTIVE_POST_SUFFIXES`](../contact-relationship-management-service-with-rust/src/auth.rs)
(⇒ `access=admin` or `svc=true` under enforcement — no persona below
admin/svc can erase or sweep, including a manager or the record's own
owner). Subject access is refused (`403`) to any caller whose decision
carries the `mask` obligation: a masked export would contradict its
own purpose.

## Activation runbook (CRM-G1)

The shipped default is **wide open** (family posture,
`agents/share/security.md` §4). Activation is a release gate, not a
config tweak:

1. **Mount a policy.** Start from the shipped reference,
   [`config/abac-policy.reference.json`](../contact-relationship-management-service-with-rust/config/abac-policy.reference.json)
   — svc/admin do everything; `resource.owner = $sub` reads and writes
   the caller's own record unmasked (record-level — see the engine
   limit below); `manager=true` writes and reads unmasked (team-wide
   pipeline/forecast oversight); `rep=true` writes (coarse: CRM has no
   ownership-enforcing write handler yet, so this is a plain write
   grant, not a scoped one); `marketing=true` / `support=true` write
   and read **masked**; every other authenticated caller gets the
   masked-read fallback. Point `CRM_ABAC_POLICY_FILE` at your copy (it
   hot-reloads on change) or inline it via `CRM_ABAC_POLICY`.
2. **Point at the keys.** `CRM_PASETO_KEYS_URL` (boot-fetched +
   refreshed) or `CRM_PASETO_KEYS`; set `CRM_TOKEN_ISSUER` /
   `CRM_TOKEN_AUDIENCE` if they differ from the defaults.
3. **Flip the flag.** `CRM_REQUIRE_AUTH=1` (read once at boot —
   restart to change).
4. **Verify.** `cargo test --test enforcement -- --ignored` runs the
   activation matrix against the reference policy shape: public paths
   open; 401 without a token; a reader can GET but not POST; a writer
   (`access=write`) completes a real consent-gated flow end to end.
   Extend it with the persona matrix above (manager unmasked vs
   marketing/support masked, `/erase` and `/sweep` admin/svc-only)
   before relying on it in a real deployment — the shipped suite
   proves the mechanism, not every persona row.

**Known engine limits a deployment must plan around** (stated, not
hidden — CRM's wiring is younger than WPM's, so this list is shorter
in the wrong direction: less is actually enforced today):

- **`resource.owner = $sub` ownership currently reaches only two
  handlers**: `GET /api/contacts/{pid}/subject-access` and
  `POST /api/contacts/{pid}/erase` (both new, CRM-T21). Every other
  handler in this crate — contacts, accounts, deals, leads, campaigns,
  tickets, articles — is gated **only** by the coarse blanket guard
  (method + destructive-suffix ⇒ action; no record loaded, no
  `resource.*` match). `auth::deal_resource_attrs` /
  `auth::contact_resource_attrs` exist and are unit-tested, but until
  more handlers call `auth::authorize_record` with them, a rep's
  "own contacts/leads/deals" scoping (spec `auth.md` persona table) is
  a **policy aspiration**, not yet an enforced boundary — any caller
  who clears the coarse write/read gate can act on any record. Plan
  deployments accordingly until that wiring lands.
- **The `mask` obligation is not yet applied by any read handler.**
  `auth::mask_json` exists and is unit-tested (nulls named keys,
  never fakes a zero), and `subject_access` treats *any* `mask`
  obligation as an outright refusal — but no list/get handler in this
  crate currently calls `mask_json` to redact deal amounts, forecast
  values, campaign ROI, or contact channel details on an ordinary
  read. A `marketing=true` or `support=true` caller today receives the
  **unredacted** response from every endpoint except subject-access.
  The obligation is real infrastructure with one real consumer so
  far; treat the sensitivity-map masking above as a policy contract to
  finish wiring, not a control already in force.
- **`rep=true` is a coarse write grant**, not a scoped one (see the
  first bullet) — a deployment relying on true per-rep ownership needs
  the record-level wiring above before this persona means what the
  spec table implies.
