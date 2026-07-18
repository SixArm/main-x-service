# Module 1 — Sales automation (SFA)

## Contacts & accounts

CRUD wrappers over `person:` / `organization:` URNs
([scope](scope.md) boundary). The contact/account detail view is the
**relationship timeline**: activities, deals, campaign touches, and
tickets merged chronologically. Ownership (`owner_ref`) drives the
manager persona's team scope ([auth](auth.md)).

## Lead management & scoring

Lifecycle `new → contacted → qualified → converted | disqualified`
(pure-core state machine; illegal transitions `422`).

**Scoring is deterministic rules, recomputed on every lead change**
(pure core, explainable — the response carries the per-rule
breakdown, the same posture as the family matchers' score
breakdowns):

| Rule | Points |
|---|---|
| source = referral | +20 |
| source = campaign (attributed) | +10 |
| known contact (`contact_ref` set) | +15 |
| corporate email domain (not a freemail list) | +10 |
| any activity in the last 7 days | +15, decaying to 0 at 30 days |
| ≥3 activities total | +10 |
| campaign click recorded | +10 |
| unsubscribe on record | −30 |

Clamped to 0–100; thresholds `hot ≥ 70`, `warm ≥ 40` label the
work queue. Weights are config-tunable; the rule *set* is fixed in
v1.

**Conversion** (`qualified → converted`) creates or links the
Contact (by `person:` URN) and optionally opens a Deal — one
transaction, audited, `lead_converted` emitted.

## Pipeline management (Kanban)

Pipelines are configurable stage lists with probabilities and
terminal won/lost flags. The board view groups open deals by stage
with `kanban_position` ordering; a stage move validates the target
stage belongs to the deal's pipeline, stamps the move into the
audit + `deal_stage_changed` event, and entering a terminal stage
sets `closed_at`/`won` (lost requires `lost_reason`). Closed deals
are immutable except a reasoned reopen. **Stalled** = no stage move
or activity for N days (config, default 14) — a derived flag on the
board.

## Forecasting

The live forecast is pure arithmetic over open deals:
`Σ amount_minor × probability_percent / 100`, grouped by
`expected_close_on` period and owner, per currency (no FX in v1 —
mixed-currency groups report per-currency lines). Month-end
snapshots persist the roll-up for later comparison
(`ForecastSnapshot`). No hand-edited forecast numbers exist
anywhere.
