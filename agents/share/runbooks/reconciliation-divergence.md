# Runbook: link-graph reconciliation divergence

The OPS-1 slice for link-graph-service's periodic integrity check —
distinct from [`event-bus-outage-replay.md`](event-bus-outage-replay.md),
which covers the *event-consumption* path that keeps the graph fresh in
real time. Reconciliation is the check that catches what the event
stream missed. See
[`cross-service-linking.md`](../cross-service-linking.md) §5, §5.1, §8
for the design.

## What a reconciliation pass actually does

Once per `LINK_GRAPH_RECONCILE_SECS` (default `300`s; a non-numeric or
`0` value silently falls back to the default), **one independent worker
per configured entity** (person, case, and care-pathway — the only
three with a live bulk `/links` endpoint today):

1. Pulls that entity's authoritative edge set: `GET
   LINK_GRAPH_RECONCILE_URL_<ENTITY>`, bearer `LINK_GRAPH_RECONCILE_TOKEN`
   if set.
2. Reads the local read-model, **scoped to that entity's `from_ref`
   prefix only** (the SEC-B1 fix — an earlier version diffed the whole
   graph against one entity's edges and each pass deleted the others').
3. Diffs the two sets **by `edge_id` only** — an edge whose kind or
   endpoints changed but keeps the same `edge_id` is not detected as
   divergent.
4. Sets the `link_graph_reconciliation_divergence` gauge to
   `missing.len() + extra.len()`.
5. Repairs: applies each `missing` edge (after validating it actually
   belongs to that entity and that its kind permits that endpoint pair —
   SEC-B7's second half; an edge that fails this check is skipped and
   **stays counted as divergence**, on purpose), and **hard-deletes**
   each `extra` edge from the read-model.

None of this is transactional — each apply/delete is its own statement,
and a mid-repair error leaves a partially-repaired graph.

## The gauge had two sharp edges — both closed (T-34/T-35)

`link_graph_reconciliation_divergence` was a **single, unlabelled
gauge**. Both entity workers wrote the same metric name, so its value
was "whatever the *most recently completed* pass of *either* entity
found" — not a sum, not per-entity. A converged `case` pass could
overwrite a diverging `person` pass's `47` with `0` a moment later, with
no way to tell from the metric alone. **T-34 fixed this**: the gauge is
now an `IntGaugeVec` labelled `entity`, so `link_graph_reconciliation_
divergence{entity="person"}` and `{entity="case"}` are independent
series — a converged pass for one entity can no longer mask another's
real divergence.

It was also **only updated on a successful fetch, with no separate
per-pass signal**. A pass that failed (timeout, non-2xx, malformed
JSON) left the divergence gauge exactly where it was — a genuine `0`
and a "hasn't run since boot" `0` looked identical, and the only
per-pass signal at all was the log line. **T-35 fixed this**: a new
`link_graph_reconciliation_last_success_unixtime` gauge, also labelled
`entity`, is set on every *successful* pass and left untouched by a
failed one — so `now() − last_success` tells you how stale a `0` is
without cross-referencing logs.

## Checks

| Check | Where | What it tells you |
|---|---|---|
| Current divergence value, per entity | `GET /metrics.prom` → `link_graph_reconciliation_divergence{entity="…"}` (public even under the guard) | Last successful pass's count **for that entity specifically** (T-34) |
| Time since last successful pass, per entity | `GET /metrics.prom` → `link_graph_reconciliation_last_success_unixtime{entity="…"}` | `now() − this` = staleness; `0`/absent = never run since boot (T-35) |
| Per-status edge counts | `GET /metrics.prom` → `link_graph_edges{status="verified\|unverified\|dangling"}` | Refreshed at scrape time from the DB; the `dangling` count is a leading indicator worth watching independent of divergence |
| Event-consumption lag (a *different* concept) | `GET /api/health/freshness` | Bus lag, not reconciliation lag — a pass can be perfectly converged while this is stale, or vice versa |
| Whether a pass ran and what happened | logs, `reconciliation pass complete` (info, carries `divergence=`) / `reconciliation pass failed` (warn, carries the error) | Still the only place a failed pass's *error detail* is visible — the gauges tell you *that* it failed (staleness), not *why* |
| Whether a source is even configured | boot log — a worker is spawned only when its `LINK_GRAPH_RECONCILE_URL_<ENTITY>` is set and passes the SEC-B7 check below | Silence from an entity can mean "converged" or "never configured" — check the env, not just the metric |

**Forcing a pass on demand (T-36, 2026-09-05).** `POST
/api/admin/reconcile/{entity}` runs one reconciliation pass for
`entity` immediately, calling the exact same `reconcile()` the periodic
worker calls — so the two paths cannot drift — and updates the same
`reconciliation_divergence` / `reconciliation_last_success_unixtime`
gauges. It is `Action::Destructive`-gated (the built-in default policy
admits only `svc=true` or `access=admin`, matching case-service's bulk
`subject_of` dump), and answers `404` when `entity` has no
`LINK_GRAPH_RECONCILE_URL_<ENTITY>` configured — there is nothing to
force. This closes the gap the rest of this section still describes for
context: before it, the only lever an operator had was restarting the
process (which waits out the full `LINK_GRAPH_RECONCILE_SECS` again
before the first pass, since the initial tick is deliberately skipped so
boot isn't blocked) or restarting with a smaller interval temporarily —
there was no endpoint, task, or admin route to force a pass on demand,
list the last-run time per entity, or see a pass/fail counter.

## Symptoms → checks → actions

**"Divergence never reaches zero, and the same warning repeats every
pass: `reconcile: rejecting an ill-typed or foreign-origin authoritative
edge`."**
This is SEC-B7's per-edge validation working as designed, not a bug: the
source is returning an edge that either doesn't originate from that
entity, or whose kind doesn't permit that endpoint-type pair (the closed
§9 registry). It will never repair itself — the rejected edge is
excluded from `missing` on every pass, forever. Fix the source data (or
the registry, if the pairing should be permitted), not the aggregator.

**"Divergence oscillates, or edges from other entities keep
disappearing after each pass."**
This is the historical SEC-B1 bug shape (fixed, but worth recognising if
it recurs): reconciliation must be scoped to its own entity's edges. If
you see this, something has regressed the scoping in
`edges::Model::edge_ids_from_entity` — check that the fix is still in
place before assuming it's a data problem.

**"A source stopped reconciling — no warning, no error, nothing."**
Check whether `LINK_GRAPH_RECONCILE_URL_<ENTITY>` names a non-loopback
host with no `LINK_GRAPH_RECONCILE_TOKEN` set (SEC-B7's *source*
half, distinct from the per-edge validation above): an unauthenticated
pull from a remote host is refused at startup, logged once as
`refusing an unauthenticated remote reconcile source: set
LINK_GRAPH_RECONCILE_TOKEN (only a loopback URL may be token-less)`,
and **no worker is spawned for that entity at all** — not a failing
pass, an entity that was never configured to reconcile in the first
place. `127.0.0.1`, `::1`, and `localhost` (case-insensitive) are the
only URLs exempt from needing a token; anything else, including
`127.0.0.2` or a hostname that happens to resolve to loopback, needs
one.

**"I set `LINK_GRAPH_RECONCILE_TOKEN`, but reconciliation still isn't
happening against a real `<ENTITY>_REQUIRE_AUTH`-enforced peer."**
The token has to be a **real PASETO** the target service's own ABAC
policy grants `Action::Destructive` to (`access=admin` or `svc=true`
in practice) — its bulk `/links` endpoint is gated as a privileged
governed read (SEC-G1), not just any valid token. A shared secret that
isn't a real, currently-valid, sufficiently-privileged token will fail
the peer's *own* auth guard, which surfaces to link-graph as an ordinary
`401`/`403` — collapsed into the same generic `reconciliation pass
failed` warn as a network timeout. If in doubt, `curl` the peer's bulk
endpoint yourself with the same token and see which HTTP status you
actually get.

**"A pass is failing and I can't tell why — the warn only says
`reconciliation pass failed` with an opaque error."**
Reproduce it directly: `curl -H "Authorization: Bearer $LINK_GRAPH_RECONCILE_TOKEN"
<the configured URL>` and read the real status code and body. link-graph
collapses a 401, a 404, a connection refusal, a timeout, and a malformed
JSON response into the identical log line — there is no discriminating
field, so the peer's own response is the fastest way to find out which
one you're in.

**"I need to force reconciliation right now, not wait for the timer."**
`POST /api/admin/reconcile/{entity}` as an `svc=true`/`access=admin`
caller (T-36) — no restart needed. Before T-36 the only lever was
restarting the process (accepting the first-tick skip, so the next real
pass was still `LINK_GRAPH_RECONCILE_SECS` away), or temporarily
lowering `LINK_GRAPH_RECONCILE_SECS` and restarting.

## A worked example, if you want to see the mechanism before trusting it

`link/link-graph-service-with-loco/tests/reconcile.rs`'s
`reconcile_adds_missing_and_removes_extra` (DB-gated, `--ignored`) seeds
the read-model with one edge, points a mock source at a different edge,
asserts the initial divergence count, applies the repair via `GET
/api/edges`, then re-runs and asserts the divergence is back to `0` —
exactly the "converged — no divergence on re-run" check worth running
by hand against a real pair of services if you're debugging a
persistent, non-converging divergence and want to rule out the
aggregator's own repair logic before looking at the data.

## What this runbook cannot help you do

- **See per-entity divergence.** The gauge is global across all
  configured entities; if you need to know which entity is diverging,
  you currently have to correlate against the `reconciliation pass
  complete divergence=<n>` log lines' entity field yourself, pass by
  pass.
- **Force a pass, or see when one last ran.** No such control exists.
- **Trust a `0` reading as proof reconciliation is healthy.** It proves
  the *last completed* pass, for *some* entity, found nothing — check
  the logs for actual pass activity before treating it as a clean bill
  of health.

These are real gaps, not just missing documentation — flag them as
follow-up work if reconciliation ever becomes an operational concern
rather than a background integrity check.
