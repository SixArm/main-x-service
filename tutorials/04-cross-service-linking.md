# Cross-service linking: `subject_of`, `same_identity`, and reconciliation

This tutorial exercises
[`agents/share/cross-service-linking.md`](../agents/share/cross-service-linking.md)
end to end: write two cross-service edges — `subject_of` (case → person,
the highest-governance v1 kind) and `same_identity` (person ↔ worker, the
federation backbone) — from their **originating** services, then query the
**link-graph aggregator**'s read-model (`neighbors`, `single-view`,
`health/freshness`), and finally force the read-model to diverge from the
source of truth and watch periodic **reconciliation** repair it, via the
`link_graph_reconciliation_divergence` Prometheus gauge.

Unlike the previous tutorials, this one needs the **full family**
compose stack (DEP-1b) — twelve services against one shared Postgres —
because the aggregator has nothing to aggregate without at least the
person, worker, and case services running alongside it.

This tutorial does **not** turn on `<ENTITY>_REQUIRE_AUTH` anywhere (TUT-3
already covered ABAC in depth) — every call here runs against the stock,
default-open compose stack, which turns out to matter for how
reconciliation gets authorized (§5 below).

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker) | builds and runs the full compose stack | 6.0.2, with `podman machine` running (12 GB RAM allocated — see DEP-1's own notes on the default 6 GB OOM-killing concurrent release builds) |
| `curl` + `python3` | verifies everything live | whatever your OS ships |
| `psql` (or `podman exec` into the Postgres container) | corrupts a row in link-graph's own read-model for the break-and-reconcile demo | bundled with the `postgres:18-alpine` image used by the stack |

## 1. Build and start the full family

This setup is heavier than every previous tutorial's: twelve service
images, each its own multi-stage release build, against a build context
that also has to pull in each crate's sibling path dependencies from the
repository root. Per
[`examples/compose/README.md`](../examples/compose/README.md), build and
`up` are **two separate commands** — `up -d --build` is documented to hang
indefinitely under this machine's compose provider. That document's
happy path is `podman compose -f examples/compose/full-family.yml build`
as one step; this run found a sharper edge one level below that.

### A real finding: a 12-way parallel `compose build` can take down the whole podman VM, not just hang

Running the documented `build` command as written —

```sh
podman compose -f examples/compose/full-family.yml build
```

— kicks off **all twelve** Dockerfiles' `cargo build --release` steps
**concurrently** (`docker-compose`'s default, confirmed in this run's own
log: all twelve "Building" / "Sending build context" lines interleaved
from the first second). On this machine — `podman machine` at the
documented 8 CPU / 12 GB allocation, **no swap configured** — that is
twelve simultaneous heavy Rust release compiles (LTO, several crates
pulling in `tantivy`/`sea-orm`/`opentelemetry`/`arrow`) competing for 12 GB
with nothing to fall back on. The build log grew normally for the first
few minutes, then went completely silent — no error, no OOM message, no
"Successfully built" — for the better part of ten minutes with the host
process's own CPU time flat (`ps` showed `7:48.47`, then `7:48.47` again,
then `7:48.48` — i.e. essentially idle, not compiling). At that point even
`podman ps -a` and `podman machine ssh ... uptime` stopped responding
(90+ seconds, no reply) — the VM itself, not merely the compose wrapper,
had become unresponsive under memory pressure. `podman machine stop` still
completed cleanly (a graceful shutdown, not a hard kill was needed), and
`podman machine start` brought it back healthy (confirmed via `free -m`:
9.9 GB free, 0 swap used, right after restart).

**If your build appears to hang or silently die the same way** — the log
stops growing with no error for several minutes, and `podman ps`/`podman
machine ssh` stop responding — that is this same resource ceiling, not a
corrupted install. Restarting the podman machine
(`podman machine stop && podman machine start`) recovers it. The reliable
fix is **not** `--parallel 1` on `compose build` (this `docker-compose`
version accepts the flag but it does not change the observed
concurrency) — it is bypassing compose's build orchestration entirely and
building each image **one at a time** with plain `podman build`, reading
each service's exact `image:` name and `dockerfile:` path straight out of
`full-family.yml`:

```sh
podman build -f person/person-service-with-loco/Dockerfile \
  -t mxi-family-person-service .
podman build -f worker/worker-service-with-loco/Dockerfile \
  -t mxi-family-worker-service .
podman build -f place/place-service-with-loco/Dockerfile \
  -t mxi-family-place-service .
podman build -f thing/thing-service-with-loco/Dockerfile \
  -t mxi-family-thing-service .
podman build -f event/event-service-with-loco/Dockerfile \
  -t mxi-family-event-service .
podman build -f course/course-service-with-loco/Dockerfile \
  -t mxi-family-course-service .
podman build -f organization/organization-service-with-loco/Dockerfile \
  -t mxi-family-organization-service .
podman build -f care-pathway/care-pathway-service-with-loco/Dockerfile \
  -t mxi-family-care-pathway-service .
podman build -f project-portfolio-management/project-portfolio-management-service-with-loco/Dockerfile \
  -t mxi-family-portfolio-service .
podman build -f authentication/authentication-service-with-loco/Dockerfile \
  -t mxi-family-authentication-service .
podman build -f link/link-graph-service-with-loco/Dockerfile \
  -t mxi-family-link-graph-service .
# case-service was already built by an earlier tutorial/DEP-1 run in this
# environment (image reused, not rebuilt, in this run)
```

Real, live-observed per-image time (sequential, one build finishing before
the next starts, memory never exceeding ~3.4 GB of the 12 GB budget):

| Image | Seconds |
|---|---|
| person | 212 |
| worker | 204 |
| place | 180 |
| thing | 178 |
| event | 191 |
| course | 185 |
| organization | 96 |
| care-pathway | 98 |
| portfolio | 106 |
| authentication | 87 |
| link-graph | 85 |
| **total (11 images; case reused)** | **1622 s ≈ 27 min** |

The four `person`-style crates (person/worker/place/thing/event/course —
the older, richer layout) each took visibly longer (178–212 s) than the
five loco-idiomatic crates + link-graph (85–106 s) — consistent with the
richer domain model and larger dependency graph the architecture doc
describes for that layout. Sequential, single-image builds never came
close to memory pressure (peak observed ~3.4 GB of 12 GB) — the ceiling
that broke the all-at-once build is specifically about *concurrency*, not
about any one image being too large to build on this machine.

Once every image exists under its exact compose-expected name, `up` does
**not** rebuild — it just starts containers from the images already in
local storage:

```sh
podman compose -f examples/compose/full-family.yml up -d
```

```
 Network compose_mxi-family Creating
 Network compose_mxi-family Created
 Volume compose_postgres_data Creating
 Volume compose_postgres_data Created
 Container mxi-family-postgres Creating
 ...
 Container mxi-family-postgres Healthy
 Container compose-case-migrate-1 Starting
 ...
 Container compose-case-migrate-1 Exited
 Container compose-case-service-1 Starting
 ...
 Container compose-care-pathway-service-1 Started
```

(Full log has one migrate → exited → service-starting sequence per
entity, exactly per [`examples/compose/README.md`](../examples/compose/README.md)'s
"migrate-then-start" explanation; trimmed here to the shape.) This step
took well under a minute — the twelve `*-migrate` one-shot containers ran
their `db migrate` and exited, then each long-running service started
against the now-migrated schema.

Verify every health endpoint — the six person-style crates answer on
`/api/health`, the five loco-idiomatic registries plus link-graph on
`/_health`:

```sh
curl -s http://localhost:8081/api/health    # person
curl -s http://localhost:8082/api/health    # worker
curl -s http://localhost:8083/api/health    # place
curl -s http://localhost:8084/api/health    # thing
curl -s http://localhost:8085/api/health    # event
curl -s http://localhost:8086/api/health    # course
curl -s http://localhost:8087/_health       # organization
curl -s http://localhost:8088/_health       # care-pathway
curl -s http://localhost:8089/_health       # case
curl -s http://localhost:8090/_health       # portfolio
curl -s http://localhost:8091/_health       # authentication
curl -s http://localhost:8092/_health       # link-graph
```

```json
{"status":"healthy","service":"person-service","version":"0.5.0"}
{"status":"healthy","service":"worker-service","version":"0.5.0"}
{"success":true,"data":{"status":"healthy","service":"place-service","version":"0.5.0"}}
{"success":true,"data":{"status":"healthy","service":"thing-service","version":"0.5.0"}}
{"status":"healthy","service":"event-service","version":"0.5.0"}
{"success":true,"data":{"status":"healthy","service":"course-service","version":"0.3.0"}}
{"ok":true}
{"ok":true}
{"ok":true}
{"ok":true}
{"ok":true}
{"ok":true}
```

All twelve `200`. Note the three distinct health-body shapes across the
family (bare, `{success,data:{...}}`, and `{"ok":true}`) — a real,
pre-existing inconsistency this tutorial did not introduce, worth knowing
if you're scripting a health check across services rather than eyeballing
it.

## 2. Create the source records: a person, a worker, and a case

Person and worker share the same demographic data — they represent one
real human, Sakura Ito, in her general-registry and workforce-registry
identities respectively — which is exactly the scenario `same_identity`
exists to assert:

```sh
curl -s -i -X POST http://localhost:8081/api/persons \
  -H "Content-Type: application/json" -H "Accepts-version: 1.0" \
  -d '{"name":{"family":"Ito","given":["Sakura"]},"gender":"female","birth_date":"1985-11-02"}'

curl -s -i -X POST http://localhost:8082/api/workers \
  -H "Content-Type: application/json" -H "Accepts-version: 1.0" \
  -d '{"name":{"family":"Ito","given":["Sakura"]},"gender":"female","birth_date":"1985-11-02"}'
```

```
HTTP/1.1 201 Created
...
{"success":true,"data":{"id":"a9718af2-6fe1-4618-b98d-1d1b4249531f","identifiers":[],"active":true,"name":{"use_type":null,"family":"Ito","given":["Sakura"],"prefix":[],"suffix":[]},"...":"..."},"error":null}

HTTP/1.1 201 Created
...
{"success":true,"data":{"id":"498c9c4f-a20c-48ea-a4a5-9c3bbc4373ff","identifiers":[],"active":true,"name":{"use_type":null,"family":"Ito","given":["Sakura"],"prefix":[],"suffix":[]},"worker_type":null,"...":"..."},"error":null}
```

Substitute your own run's ids for the rest of this tutorial:

```sh
PERSON_ID=a9718af2-6fe1-4618-b98d-1d1b4249531f
WORKER_ID=498c9c4f-a20c-48ea-a4a5-9c3bbc4373ff
```

Then a governmental case naming that person as its subject (reusing the
body shape [`examples/api/case.http`](../examples/api/case.http) already
curl-verifies — but with the real `PERSON_ID` above in `subjects`, not the
file's placeholder `person:abc`; the case service is loco-idiomatic, so
its create response is **bare** JSON, not the `{success,data,error}`
envelope person/worker use):

```sh
curl -s -i -X POST http://localhost:8089/api/cases \
  -H "Content-Type: application/json" -H "Accepts-version: 1.0" \
  -d "{
  \"title\": \"Housing benefit appeal\",
  \"agency_id\": \"dwp\",
  \"case_number\": \"HB-2026-0099\",
  \"subjects\": [\"person:${PERSON_ID}\"],
  \"keywords\": [\"housing\", \"benefit\"],
  \"identifiers\": [{ \"scheme\": \"Docket\", \"value\": \"CV-2026-009900\" }]
}"
```

```
HTTP/1.1 200 OK
...
{"pid":"f44c9955-19cc-4201-8a6c-e6d8cc857ff8","title":"Housing benefit appeal"}
```

```sh
CASE_PID=f44c9955-19cc-4201-8a6c-e6d8cc857ff8
```

`subjects: ["person:<uuid>"]` here is the case's own **domain field** —
free-text-ish subject refs stored on the case record itself, unrelated to
`entity_links`. The next step writes the actual governed
[`cross-service-linking.md`](../agents/share/cross-service-linking.md) §9
edge, which is a separate table, a separate event, and (§10) a separately
governed read.

## 3. Write the edges — local, optimistic, no cross-service call

Per design §4.1, each write lands in the **originating** service's own
`entity_links` table and never calls the target service. Person is the
reference originator of `same_identity` (§9); person → worker is the one
direction this run exercises (worker's own `POST
/api/workers/{id}/links` exists too — confirmed live in this crate's
source, `src/api/rest/links.rs` — but design §12's "either side may
assert" is still an open question, and this tutorial only asserts from
one side, matching how the design doc's own worked example reads).

```sh
curl -s -i -X POST http://localhost:8081/api/persons/${PERSON_ID}/links \
  -H "Content-Type: application/json" \
  -d "{\"kind\":\"same_identity\",\"to_ref\":\"worker:${WORKER_ID}\"}"
```

```
HTTP/1.1 200 OK
...
{"success":true,"data":{"id":"b6b8e17d-922e-41e2-93c4-e58d003ab7d0","from_ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","kind":"same_identity","to_ref":"worker:498c9c4f-a20c-48ea-a4a5-9c3bbc4373ff","role":null,"confidence":null,"provenance":"operator","valid_from":null,"valid_to":null},"error":null}
```

Case is the reference originator of `subject_of` (§9, §10 — the
highest-governance v1 kind):

```sh
curl -s -i -X POST http://localhost:8089/api/cases/${CASE_PID}/links \
  -H "Content-Type: application/json" \
  -d "{\"kind\":\"subject_of\",\"to_ref\":\"person:${PERSON_ID}\",\"provenance\":\"operator\"}"
```

```
HTTP/1.1 200 OK
...
{"id":"d9e47aec-564b-4479-80ce-1d7d1dc39a04","from_ref":"case:f44c9955-19cc-4201-8a6c-e6d8cc857ff8","kind":"subject_of","to_ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","role":null,"confidence":null,"provenance":"operator","valid_from":null,"valid_to":null}
```

(Note case's link-write response is bare JSON — same envelope
inconsistency as its `POST /api/cases` above, not new here.) Both writes
are `200`, both are local to the originating service's own database, and
neither call touched link-graph or the other endpoint's service at all.

## 4. Before the aggregator sees anything: a real, documented gap

Query the aggregator right now, before doing anything else:

```sh
curl -s http://localhost:8092/api/health/freshness
curl -s "http://localhost:8092/api/neighbors/person:${PERSON_ID}"
```

```json
{"success":true,"data":{"topics":[],"as_of":null}}
{"success":true,"data":{"ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","edges":[],"as_of":null}}
```

Both empty. This is not lag in the usual sense — it's the gap
[`examples/api/link-graph.http`](../examples/api/link-graph.http)'s own
GOTCHA note documents: `full-family.yml` leaves every
`<ENTITY>_EVENT_TRANSPORT` at its default `memory` (an in-process bus),
and link-graph's own image was built without the `fluvio` Cargo feature
(the default — confirmed in `src/consumer.rs`'s module doc: "a default
build has no consumer at all"). There is, right now, **no mechanism at
all** by which a `linked` event reaches this aggregator — not slow, not
retrying, simply not wired. `freshness.topics` will stay `[]` and every
graph response's `as_of` will stay `null` for the **entire rest of this
tutorial**, even after reconciliation (§5) starts populating real edges —
`as_of` is the bus-consumption watermark specifically
(`consumer_offsets`), a different signal from reconciliation's own
`link_graph_reconciliation_divergence` metric (§7). Worth internalising
before the next step, so a `null` `as_of` next to real edges doesn't read
as a bug.

The **only** path a new edge can reach this read-model in this compose
stack is the one design §8 calls the integrity *check*, not the primary
feed: periodic **reconciliation**, configured next.

## 5. Configure reconciliation + lazy verify-on-read

Both are boot-time env vars on link-graph-service, so this needs a
container **recreate** (a compose override, applied on top of
`full-family.yml` — not a change to any tracked file):

```yaml
# scratch override, NOT staged/committed — points reconciliation at
# person's and case's bulk /links endpoints, and turns on lazy
# verify-on-read so edge status settles without waiting on a bus that
# isn't running.
services:
  link-graph-service:
    environment:
      LINK_GRAPH_RECONCILE_URL_PERSON: http://person-service:8080/api/persons/links
      LINK_GRAPH_RECONCILE_URL_CASE: http://case-service:5150/api/cases/links
      LINK_GRAPH_RECONCILE_TOKEN: dev-tut4-placeholder-token
      LINK_GRAPH_RECONCILE_SECS: "10"
      LINK_GRAPH_LAZY_VERIFY: "true"
      LINK_GRAPH_PROBE_URL_PERSON: http://person-service:8080/api/persons/{id}
      LINK_GRAPH_PROBE_URL_WORKER: http://worker-service:8080/api/workers/{id}
      LINK_GRAPH_PROBE_URL_CASE: http://case-service:5150/api/cases/{id}
```

Deliberately **not** `LINK_GRAPH_RECONCILE_URL_WORKER`: this tutorial
only asserts `same_identity` from the person side (§3), so worker's own
`entity_links` stays empty — a worker reconcile worker would just be a
third always-`0` writer to the shared divergence gauge (§7 explains why
that specifically matters), with no edge of its own to contribute. The
probe URL for worker is still configured, because lazy verify-on-read
needs it to resolve the `same_identity` edge's *other* endpoint.

Two things worth knowing about `LINK_GRAPH_RECONCILE_TOKEN` before using
it: it is a **placeholder string, not a real PASETO**. Link-graph's own
SEC-B7 gate (`src/reconcile.rs::source_auth_ok`) only requires a
*non-empty* bearer before it will pull from a non-loopback URL — the
compose network's service hostnames (`person-service`, `case-service`)
are not loopback, so *some* token is mandatory or the source is silently
refused (logged as a `tracing::warn!`, confirmed by the design doc and
the `reconciliation-divergence.md` runbook). But **what the token
actually contains never matters here**, because `PERSON_REQUIRE_AUTH` /
`CASE_REQUIRE_AUTH` are both off (`full-family.yml`'s default) — and both
services' `authorize_bulk` short-circuits to `Ok(())` before even looking
at the bearer when their own `REQUIRE_AUTH` flag is off (confirmed by
reading `case/case-service-with-loco/src/auth.rs::authorize_record` and
`person/worker`'s `src/api/rest/links.rs::authorize_bulk`). Turning on
enforcement (as TUT-3 does, and as `examples/compose/enforced.yml`
documents in its own header) would change this: the placeholder would
then need to be a real `access=admin`/`svc=true` PASETO, and minting one
needs a live pass through authentication-service — which is exactly why
`enforced.yml` leaves `LINK_GRAPH_RECONCILE_TOKEN` empty and documents
completing it as a manual step, rather than baking in a fake one.

```sh
podman compose -f examples/compose/full-family.yml \
               -f /tmp/tut4-scratch/reconcile-override.yml \
               up -d
```

```
 Container compose-link-graph-service-1 Recreate
 Container compose-link-graph-service-1 Recreated
 ...
 Container compose-link-graph-service-1 Started
```

(Every other container stays `Running` — only link-graph-service is
recreated, since it's the only service the override touches. The twelve
`*-migrate` one-shot containers also re-run on this `up`, harmlessly —
sea-orm-migration tracks applied migrations, so a second `db migrate`
against an already-migrated schema is a no-op.)

## 6. Query the aggregator: neighbors, single-view, and status settling to `verified`

`LINK_GRAPH_RECONCILE_SECS=10` and `tokio::time::interval`'s fixed
schedule (ticks at `t=0` — consumed deliberately so boot isn't blocked —
then `t=10`, `t=20`, …) means the first real reconciliation pass lands
about 10 seconds after the container starts. A query at `t+15s` already
shows both edges:

```sh
curl -s "http://localhost:8092/api/neighbors/person:${PERSON_ID}"
```

```json
{"success":true,"data":{"ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","edges":[
  {"edge_id":"b6b8e17d-922e-41e2-93c4-e58d003ab7d0","from_ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","to_ref":"worker:498c9c4f-a20c-48ea-a4a5-9c3bbc4373ff","kind":"same_identity","directed":false,"role":null,"confidence":null,"provenance":"operator","valid_from":null,"valid_to":null,"status":"verified","observed_at":"2026-08-04T10:11:56.623905Z","source_event_id":"b6b8e17d-922e-41e2-93c4-e58d003ab7d0"},
  {"edge_id":"d9e47aec-564b-4479-80ce-1d7d1dc39a04","from_ref":"case:f44c9955-19cc-4201-8a6c-e6d8cc857ff8","to_ref":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","kind":"subject_of","directed":true,"role":null,"confidence":null,"provenance":"operator","valid_from":null,"valid_to":null,"status":"verified","observed_at":"2026-08-04T10:11:56.619042Z","source_event_id":"d9e47aec-564b-4479-80ce-1d7d1dc39a04"}
],"as_of":null}}
```

Real, worth noticing:

- **One query surfaces edges from both directions and both originating
  services** — `neighbors/person:...` returns the `same_identity` edge
  (person is `from`) *and* the `subject_of` edge (person is `to`,
  originated by case) in the same response. Neither service knew about
  the other's write; the aggregator is what makes them visible together.
- **`same_identity` canonicalised without reordering** here: `from_ref`
  stayed `person:...` because `"person:" < "worker:"` lexicographically
  (`graph::canonical`, design §6 FR-6) — the edge would have been
  reordered had the two URNs sorted the other way, regardless of which
  side actually asserted it.
- **`status` is already `"verified"`, not `"unverified"`** — lazy
  verify-on-read (§5.1) fired synchronously on this very read: an
  endpoint whose presence was unknown got a one-shot `GET` to its owning
  service (`LINK_GRAPH_PROBE_URL_<ENTITY>`), the `200` cached the verdict
  in `entity_presence`, and the response reflects the recomputed status
  in the same round-trip. This has nothing to do with reconciliation or
  the bus — it is a third, independent integrity path (§5.1), and it is
  the only reason `status` isn't stuck at `unverified` forever in this
  compose stack.
- **`as_of` is still `null`** — exactly the §4 finding holding: this
  field tracks bus-consumption freshness, and there is still no consumer.
  Real edges with real `verified` status and a `null` freshness watermark
  side by side is the correct, if slightly odd-looking, state here — not
  a bug.

`single-view` confirms the golden-record walk works across both edge
kinds in one call:

```sh
curl -s "http://localhost:8092/api/single-view/person:${PERSON_ID}"
```

```json
{"success":true,"data":{
  "identity_refs":["person:a9718af2-6fe1-4618-b98d-1d1b4249531f","worker:498c9c4f-a20c-48ea-a4a5-9c3bbc4373ff"],
  "affiliations":[{"from":"case:f44c9955-19cc-4201-8a6c-e6d8cc857ff8","to":"person:a9718af2-6fe1-4618-b98d-1d1b4249531f","kind":"subject_of"}],
  "as_of":null
}}
```

`identity_refs` is the `same_identity`-unified set (person + worker,
sorted); `affiliations` is every non-`same_identity` edge touching that
set — here, the one `subject_of` edge, correctly attributed to `case`,
even though the query started from `person`. And the metrics baseline
this tutorial's next step diffs against:

```sh
curl -s http://localhost:8092/metrics.prom | grep -E "link_graph_edges|link_graph_reconciliation_divergence"
```

```
link_graph_edges{status="dangling"} 0
link_graph_edges{status="unverified"} 0
link_graph_edges{status="verified"} 2
link_graph_reconciliation_divergence 0
```

## 7. Break-and-reconcile: force divergence, watch it repair

The read-model and each service's `entity_links` are **two separate
stores** (design §8) — they can drift on a dropped event or a relay bug.
The cleanest way to *manufacture* that drift for a demo is to bypass the
normal write path entirely and edit link-graph's own `edges` table
directly with `psql`, so `entity_links` in case-service (the source of
truth) and the aggregator's read-model now disagree. The edge id is the
one `POST /api/cases/{pid}/links` returned back in §3:

```sh
SUBJECT_OF_EDGE_ID=d9e47aec-564b-4479-80ce-1d7d1dc39a04

podman exec -i mxi-family-postgres psql -U loco -d link_graph -c \
  "DELETE FROM edges WHERE edge_id='${SUBJECT_OF_EDGE_ID}';"
podman exec -i mxi-family-postgres psql -U loco -d link_graph -c \
  "INSERT INTO edges (edge_id, from_ref, to_ref, kind, directed, role,
     confidence, provenance, valid_from, valid_to, status, observed_at,
     source_event_id)
   VALUES (gen_random_uuid(), 'case:${CASE_PID}',
     'person:88888888-8888-4888-8888-888888888888', 'subject_of', true,
     NULL, NULL, 'operator', NULL, NULL, 'unverified', now(),
     gen_random_uuid());"
```

```
DELETE 1
INSERT 0 1
```

One statement removes the real `subject_of` edge (the **missing** case —
still authoritative in case-service, gone from the read-model); the other
injects a fabricated one pointing at a person id that doesn't exist (the
**extra** case — present in the read-model, absent from case-service).
Both are scoped to the same case's `from_ref`, so the same per-entity
reconcile pass (§8 SEC-B1 scoping) catches both in one tick. Immediately
after, the aggregator visibly disagrees with reality:

```sh
curl -s "http://localhost:8092/api/neighbors/person:${PERSON_ID}"   # subject_of edge is just... gone
curl -s "http://localhost:8092/api/edges?kind=subject_of"           # shows the fabricated one instead
```

```json
{"success":true,"data":{"ref":"person:...","edges":[{"...":"same_identity edge only..."}],"as_of":null}}
{"success":true,"data":{"edges":[{"edge_id":"...","from_ref":"case:...","to_ref":"person:88888888-...","kind":"subject_of","...","status":"dangling","...}]}}
```

A bonus, unplanned confirmation that lazy verify-on-read (§6) is doing
real work: the fabricated edge's `status` comes back `"dangling"`, not
`"unverified"` — the very `GET` above triggered a probe against
`person-service` for `88888888-...`, got a `404`, and
`entity_presence`/`edge_status` correctly concluded the endpoint is
known-gone. The corruption is caught as *implausible* before
reconciliation even gets a chance to fix it.

### Real, empirically-surprising finding: the divergence gauge showed `0` through two full corrupt→repair cycles

Expecting to see `link_graph_reconciliation_divergence` climb to `2`
during the broken window, this tutorial polled it (and the `/api/edges`
read-model, and `entity_links` directly) roughly every second across two
separate corrupt→repair cycles, ~20–30 seconds each. **The gauge read
`0` on every single poll, in both runs** — even while the read-model was
genuinely diverged (confirmed by `/api/edges` and the `entity_links`
table not matching), and even during the tick where the repair
demonstrably happened (the fabricated edge disappearing and the real one
reappearing, confirmed via direct `psql` reads of `edges.observed_at`).

This is not a bug hunted down mid-tutorial — it is
[`agents/share/runbooks/reconciliation-divergence.md`](../agents/share/runbooks/reconciliation-divergence.md)'s
own documented "sharp edge", reproduced live: `link_graph_reconciliation_divergence`
is **one unlabelled gauge shared by every configured entity's reconcile
worker** (`case` and `person` in this tutorial's override, §5). Both
workers tick independently on the same ~10 s schedule; `person`'s own
pass finds `0` divergence on **every** tick (its `entity_links` was never
touched), and whichever of the two workers' `.set()` call happens to run
last within a given tick wins the gauge's value until the next tick. With
two workers and only one of them ever diverging, the runbook's own
phrasing — "a converged pass can overwrite a diverging pass's count a
moment later, and you'd never know from the metric alone" — turned out
to describe *most* ticks in this run, not a rare race: `0` from `person`
kept winning (or `case`'s own `2`→repair→`0` transition landed inside a
window this tutorial's ~1 s polling granularity never caught — the two
are empirically indistinguishable from outside, which is exactly the
runbook's point). The takeaway this tutorial can now personally vouch
for: **do not gate a "did reconciliation catch and fix the divergence"
check on the gauge alone** — query `/api/edges` (or the per-status
`link_graph_edges` gauge, which *is* accurate at scrape time, just not
attributed to a cause) for ground truth, and reserve the divergence gauge
for "is there active divergence *right now*, from *some* configured
entity" — never a specific one, and never a reliable historical record.

### Real observed repair timing

The read-model itself, not the gauge, is what actually shows the repair.
Across the two corrupt→repair cycles run for this tutorial (each timed
from the `psql` corruption statement to the restored edge's new
`observed_at`):

| Run | Corrupted at | Repaired at (`edges.observed_at`) | Latency |
|---|---|---|---|
| 1 | 11:13:30 | 11:13:36.589 | ~6.6 s |
| 2 | 11:14:57.3 | 11:15:06.605 | ~9.3 s |

Both land inside the configured `LINK_GRAPH_RECONCILE_SECS=10` window, as
expected for a fixed-schedule periodic tick — the honest range is
"anywhere from just-after-corruption to just-under-the-full-interval,
depending on how the corruption timing happens to land relative to the
next tick boundary," the same shape TUT-3 found for ABAC policy
hot-reload. Confirming the read-model, not the gauge, converged:

```sh
curl -s http://localhost:8092/metrics.prom | grep -E "link_graph_edges|link_graph_reconciliation_divergence"
curl -s "http://localhost:8092/api/neighbors/person:${PERSON_ID}"
```

```
link_graph_edges{status="dangling"} 0
link_graph_edges{status="unverified"} 0
link_graph_edges{status="verified"} 2
link_graph_reconciliation_divergence 0
```

Both edges are back, both `verified`, `link_graph_edges{status}` (the
*accurate*, cause-blind gauge) confirms `0` dangling / `0` unverified —
the read-model has converged with the source of truth, exactly as design
§8 promises, just not provably from the divergence gauge alone in a
multi-entity deployment.

## 8. Tear down

```sh
podman compose -f examples/compose/full-family.yml down -v
```

```
 Container compose-place-migrate-1 Stopping
 ...
 Container mxi-family-postgres Removed
 Volume compose_postgres_data Removing
 Network compose_mxi-family Removing
 Network compose_mxi-family Removed
 Volume compose_postgres_data Removed
```

`down -v` removes every container, the shared network, and the Postgres
volume — the next `up` starts from twelve genuinely empty databases, same
as `scripts/test-db.sh down`'s tmpfs-backed guarantee for the per-crate
test stacks. Also clean up the scratch override and person/worker/case
ids from your shell:

```sh
rm -rf /tmp/tut4-scratch
```

The twelve `mxi-family-*` images built in §1 are **not** removed by
`down -v` (they're local build artifacts, not compose-managed state) — a
second run of this tutorial can skip straight to `up -d` without
rebuilding, unless the source has changed since.

## What's next

- **TUT-5 — bulk import/export**: fixture import (dry-run, error report),
  idempotent re-import, masked vs. full export.
- **TUT-6 — event bus**: outbox rows, the relay, `/events/recent`.

See [`tasks.md`](../tasks.md) for their current status.
