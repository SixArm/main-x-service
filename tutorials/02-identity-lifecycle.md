# Identity lifecycle: create, duplicate, match, merge, audit

This tutorial walks the full duplicate-handling lifecycle on **one
service — person-service** — the family's richest matching
implementation: create a near-duplicate and watch it get blocked at
`409`, check for duplicates without creating anything, score a
probable-not-certain match, run a batch scan into the stored review
queue, decide two pending items, merge a confirmed pair, and read the
resulting audit trail. Then the same review → confirm → merge sequence
again, through the operator front-end.

Person is the right service for this, not an arbitrary choice: three
things landed earlier this session specifically to make this tutorial
possible.

1. [`examples/data/persons.jsonl`](../examples/data/persons.jsonl) — 50
   synthetic persons with **five deliberate duplicate pairs**, one of
   which (Ren vs Kenji Nakamura) is scored *probable*, not *certain*, on
   purpose — see
   [`examples/data/README.md`](../examples/data/README.md#the-duplicate-pairs)
   for the full list and scores.
2. `cargo loco task seed_examples` — loads all 50 persons through the
   **model layer**, bypassing the create endpoint's real-time duplicate
   detection so both halves of every pair actually land (`POST
   /api/persons` would `409` the second half of each pair).
3. [`person-front-end-with-svelte`](../person/person-front-end-with-svelte/)'s
   `/review` screen — a Kanban + table view over the stored review
   queue, a side-by-side comparison panel, and a merge deep-link.

This tutorial does **not** cover authentication (the service runs with
`PERSON_REQUIRE_AUTH` off, its default — see TUT-3) or Podman (TUT-1
already covered the container path; this one runs the service and its
Postgres directly, which is faster to iterate on and just as real).

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker) | only for the throwaway test Postgres | 6.0.2, with `podman machine` running |
| Rust (this repo pins `1.96.1` in [`rust-toolchain.toml`](../rust-toolchain.toml)) | builds and runs person-service directly | `cargo` on `PATH` |
| [Node.js](https://nodejs.org/) 20+ and [pnpm](https://pnpm.io/) | runs the front-end dev server | Node v26.5.1, pnpm 11.0.8 |
| `curl` + `python3` (for `python3 -m json.tool`) | verifies the backend directly | whatever your OS ships |

## 1. Start Postgres and the service

The family ships a per-crate throwaway test Postgres
([`postgresql.md`](../agents/share/postgresql.md)): one `postgres:18-alpine`
container, superuser `loco`/`loco`, on tmpfs so every start is a clean
`initdb`. Run this from the repository root:

```sh
scripts/test-db.sh up person/person-service-with-loco
```

```
 Container mxi-person-test-db Starting
 Container mxi-person-test-db Started
test-db: waiting for mxi-person-test-db...
test-db: mxi-person-test-db ready
  DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test
```

person-service's own [`README.md`](../person/person-service-with-loco/README.md)
"Local Development" section documents `cargo loco start`. **That doesn't
work as written in this environment** — `cargo loco` needs a `cargo-loco`
shim on `PATH`, and only the standalone `loco` binary (the Postgres/Redis
generator CLI) is installed here:

```
$ cargo loco --version
error: no such command: `loco`
```

The README's own parenthetical alternative, `cargo run -- start`, is
what actually works, so that's what this tutorial uses throughout
(`cargo run -- <subcommand>` for everything — `start`, `db migrate`,
`task ...`).

Point `DATABASE_URL` at the test Postgres and apply migrations
(`development.yaml`'s `auto_migrate` would also do this on `start`, but
running it explicitly first makes the empty-database state visible):

```sh
cd person/person-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test
cargo run -- db migrate
```

This applies ~20 migrations (content-hash columns, the integrity MAC
columns, the review-queue `provenance` column, …); the tail looks like:

```
Migration 'm20260802_000001_review_queue_provenance' has been applied
```

Now start the server. `config/development.yaml` listens on **port
5150**, which is also `person-front-end-with-svelte`'s dev-mode default
target — no port juggling needed later.

```sh
cargo run -- start
```

```
environment: development
   database: logging, automigrate
     logger: debug
compilation: debug
      modes: server

listening on http://0.0.0.0:5150
```

Leave this running (background it, or use a second terminal) and verify
it's actually up:

```sh
curl -s http://localhost:5150/api/health
```

```json
{"status":"healthy","service":"person-service","version":"0.5.0"}
```

## 2. Seed the duplicate pairs

```sh
cd person/person-service-with-loco   # if not already there
DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test \
  cargo run -- task seed_examples
```

```
seeded person 1/50: Adaeze Okonkwo (26578248-52f2-45a0-8a4f-ebae2ba81b32)
seeded person 2/50: Pieter Vantongeren (87552410-e62b-4432-93d5-817336668db5)
seeded person 3/50: Lucia Sandoval-Reyes (cc06d132-a162-45af-ba84-94950547ba24)
seeded person 4/50: Adaeze Okonkow (921c48ac-d073-49db-b235-4ad82f8278c3)
...
seeded person 17/50: Ren Nakamura (195503f4-d4d6-4912-a40b-35300a7a48f0)
...
seeded person 19/50: Kenji Nakamura (1e8ebf4f-059c-4341-b2d3-008602952d7e)
...
seeded 50 of 50 persons (existing: 0)
```

(Real UUIDs from the run this tutorial was verified against — yours
will differ. Person **4** ("Okonkow") and person **19** ("Kenji
Nakamura") are the second half of two of the five duplicate pairs, sat
right next to their match at 1/17: exactly the pairs `POST
/api/persons` would `409` on the second half of, which is why the task
loads them through the model layer instead.)

The task's own doc comment
([`src/tasks/seed_examples.rs`](../person/person-service-with-loco/src/tasks/seed_examples.rs))
explains why this bypasses the create endpoint, and also states it
writes **no audit row and publishes no event** — deliberately, so the
tutorials that exercise duplicate detection, audit, and events do so
against records already present, not against the act of seeding.

### A finding this tutorial depends on: seeding also skips the search index

The doc comment above doesn't mention this, but it's true and it
matters for the next three steps: `POST /api/persons`,
`check-duplicates`, and `match` all **block on the Tantivy search
index** before running the matcher (`dataflow.md`: "Duplicate Detection
(search + match against existing)"). The model-layer insert
`seed_examples` uses has no access to the search engine, so seeded
persons are in Postgres but invisible to search — confirmed live:

```sh
curl -s -X POST http://localhost:5150/api/persons/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Okonkwo","given":["Adaeze"]},"gender":"female","birth_date":"1984-03-11"}'
```

```json
{"success":true,"data":{"has_duplicates":false,"potential_matches":[]},"error":null}
```

That's an exact copy of seeded person #1's own name and birth date, and
the service reports no duplicate at all. (`POST
/api/persons/deduplicate`, the **batch** scan used in step 6, is
unaffected — it walks the database directly and never touches the
search index; see that step.)

The fix, for the two pairs this tutorial actually exercises, is a
no-op `PUT` — `update_person` **does** re-index (`state.search_engine
.index_person(...)`), unlike the seed task:

```sh
curl -s -X PUT http://localhost:5150/api/persons/26578248-52f2-45a0-8a4f-ebae2ba81b32 \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Okonkwo","given":["Adaeze"]},"gender":"female","birth_date":"1984-03-11"}'

curl -s -X PUT http://localhost:5150/api/persons/195503f4-d4d6-4912-a40b-35300a7a48f0 \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Nakamura","given":["Ren"]},"gender":"male","birth_date":"1988-07-19"}'

curl -s -X PUT http://localhost:5150/api/persons/1e8ebf4f-059c-4341-b2d3-008602952d7e \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Nakamura","given":["Kenji"]},"gender":"male","birth_date":"1988-07-19"}'
```

Each returns `200` with the unchanged record. Substitute the pids your
own seed run printed for persons 1 ("Okonkwo"), 17 ("Ren Nakamura"), and
19 ("Kenji Nakamura").

## 3. Create → 409 duplicate

Post a near-duplicate of the now-indexed Okonkwo record — close, but not
identical (a further-corrupted family name, birth date a day off):

```sh
curl -s -i -X POST http://localhost:5150/api/persons \
  -H "Content-Type: application/json" \
  -H "Accepts-version: 1.0" \
  -d '{"name":{"family":"Okonkwoh","given":["Adaeze"]},"gender":"female","birth_date":"1984-03-12"}'
```

```
HTTP/1.1 409 Conflict
content-type: application/json
accepts-version: 1.0

{"success":false,"data":null,"error":{"code":"DUPLICATE_DETECTED","message":"Potential duplicate persons found. Review matches before proceeding.","details":{"has_duplicates":true,"potential_matches":[{"detection_method":"duplicate_detection","person":{"...":"...","id":"26578248-52f2-45a0-8a4f-ebae2ba81b32","name":{"family":"Okonkwo","given":["Adaeze"]},"...":"..."},"quality":"certain","score":0.9749999999999999,"score_breakdown":{"address_score":0.0,"birth_date_score":0.95,"document_score":0.0,"gender_score":1.0,"identifier_score":0.0,"name_score":0.9874999999999999,"tax_id_score":0.0}}]}}}
```

(The candidate's full record is trimmed above for readability — the
real response embeds it verbatim, as `examples/api/person.http` and
`agents/share/match-search-merge.md` document.) Nothing was created:
the record never exists under any id, and a search for the near-dup
name comes back empty:

```sh
curl -s "http://localhost:5150/api/persons/search?q=Okonkwoh&limit=10"
```

```json
{"success":true,"data":{"persons":[],"total":0,"query":"Okonkwoh","offset":0,"limit":10}}
```

## 4. `check-duplicates` — same detection, no side effect

Same body, different endpoint: this one only ever reports, whether or
not it finds anything.

```sh
curl -s -X POST http://localhost:5150/api/persons/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Okonkwoh","given":["Adaeze"]},"gender":"female","birth_date":"1984-03-12"}'
```

```json
{"success":true,"data":{"has_duplicates":true,"potential_matches":[{"person":{"id":"26578248-52f2-45a0-8a4f-ebae2ba81b32","name":{"family":"Okonkwo","given":["Adaeze"]},"birth_date":"1984-03-11","...":"..."},"score":0.9749999999999999,"quality":"certain","detection_method":"duplicate_detection","score_breakdown":{"address_score":0.0,"birth_date_score":0.95,"document_score":0.0,"gender_score":1.0,"identifier_score":0.0,"name_score":0.9874999999999999,"tax_id_score":0.0}}]}}
```

Identical score and breakdown to the `409` — same
`check_duplicates_internal` function backs both. The only difference is
the wrapper (`has_duplicates`/`potential_matches` here vs.
`error.details` on a `409`) and that this one is safe to call
speculatively from a create form before submitting.

## 5. `match` — a probable, not certain, score

`POST /api/persons/match` takes one probe record and searches for
candidates the same way create/check-duplicates do (fresh index entries
required — this is why Ren and Kenji were reindexed in step 2). Probe
with Ren Nakamura's own data:

```sh
curl -s -X POST http://localhost:5150/api/persons/match \
  -H "Content-Type: application/json" \
  -d '{"name":{"family":"Nakamura","given":["Ren"]},"gender":"male","birth_date":"1988-07-19"}'
```

```json
{"success":true,"data":{"matches":[
  {"person":{"id":"195503f4-d4d6-4912-a40b-35300a7a48f0","name":{"family":"Nakamura","given":["Ren"]},"...":"..."},"score":1.0,"quality":"certain","detection_method":"probabilistic","score_breakdown":{"birth_date_score":1.0,"gender_score":1.0,"name_score":1.0,"...":"0.0 elsewhere"}},
  {"person":{"id":"1e8ebf4f-059c-4341-b2d3-008602952d7e","name":{"family":"Nakamura","given":["Kenji"]},"...":"..."},"score":0.9425641025641024,"quality":"probable","detection_method":"probabilistic","score_breakdown":{"birth_date_score":1.0,"gender_score":1.0,"name_score":0.8755555555555555,"...":"0.0 elsewhere"}}
],"total":2},"error":null}
```

Two things worth noticing in this real response. First, `match` doesn't
exclude the probe from its own candidates the way `check-duplicates`
does (it excludes by matching `person.id`, but a match probe has no
`id` — it need not be a stored record at all) — since this probe
happens to equal Ren's own stored data, Ren comes back as a `1.0`
self-match ahead of Kenji. Second, Kenji's score, **0.9425641025641024**,
rounds to exactly the **0.9426 "probable"** score
[`examples/data/README.md`](../examples/data/README.md#the-duplicate-pairs)
documents for this pair — the one pair in the fixture deliberately built
to land below the 0.95 "certain" line, so an operator has a real
decision to make rather than an obvious one.

## 6. Populate and read the review queue

Neither `check-duplicates` nor `match` writes anything to the stored
review queue — they're read-only scoring endpoints. The **only** thing
that populates `review_queue` rows is the batch scan,
`POST /api/persons/deduplicate` — confirmed by reading the handler
(`crate::db::review_queue::upsert` is called nowhere else) and by
testing it. Unlike create/check-duplicates/match, this one scans
`list_active()` directly rather than blocking on the search index, so
it finds all five pairs regardless of what's indexed:

```sh
curl -s -X POST http://localhost:5150/api/persons/deduplicate \
  -H "Content-Type: application/json" \
  -d '{"auto_merge_threshold": 1.01}'
```

The request overrides `auto_merge_threshold` (default `0.95`) to just
above the maximum possible score. Without that, four of the five pairs
here (everything except Nakamura) score `≥ 0.95` and would be stored
with status `automerged` — but the handler's own doc comment is explicit
that this label is **not** an action: "Does not itself merge — it only
produces review-queue items." Only `pending` items can be decided
(`POST .../review-queue/{id}/decision` answers `422` on anything else),
so an `automerged`-labelled pair would sit in the queue unreachable.
Pushing the threshold above 1.0 keeps every real duplicate in the
decidable `pending` state instead.

```json
{"success":true,"data":{"persons_scanned":50,"duplicates_found":5,"auto_merged":0,"queued_for_review":5,"review_items":[
  {"id":"42843791-bd11-4fce-9fdc-c55729d42fbe","person_id_a":"26578248-...","person_id_b":"921c48ac-...","match_score":0.9934065934065933,"match_quality":"certain","status":"pending","provenance":"operator","...":"..."},
  {"id":"0678f250-7ce4-44c4-8a9e-8a61a92fadfa","person_id_a":"a0ea036d-...","person_id_b":"aabda370-...","match_score":0.9907692307692307,"match_quality":"certain","status":"pending","...":"..."},
  {"id":"e788888b-93cc-4235-9e58-0691dd5622a7","person_id_a":"64121a81-...","person_id_b":"9c5b66fb-...","match_score":0.9705128205128204,"match_quality":"certain","status":"pending","...":"..."},
  {"id":"8c3d796f-449d-4d90-bbc9-2fcc7a9c8dc5","person_id_a":"03669b2f-...","person_id_b":"1d887df7-...","match_score":0.9994999999999999,"match_quality":"certain","status":"pending","...":"..."},
  {"id":"03513dd5-e821-4f26-870a-6364f2fad9e9","person_id_a":"195503f4-...","person_id_b":"1e8ebf4f-...","match_score":0.9425641025641024,"match_quality":"probable","status":"pending","...":"..."}
]},"error":null}
```

All five documented pairs, all five documented scores
(`0.9934`/`0.9908`/`0.9705`/`0.9995`/`0.9426`), all `pending`. Ids are
**stable** across re-scans (a normalized-pair upsert), so note them down
— they're used below. List just the pending ones the way an operator
would:

```sh
curl -s "http://localhost:5150/api/persons/review-queue?status=pending"
```

## 7. Decide: reject the ambiguous one, confirm a clear one

The Nakamura pair (id `03513dd5-...`) is the fixture's deliberately
ambiguous case — a different given name on an otherwise-matching
record. A real reviewer might reasonably go either way; here we reject
it, to show that path exists and actually works:

```sh
curl -s -X POST http://localhost:5150/api/persons/review-queue/03513dd5-e821-4f26-870a-6364f2fad9e9/decision \
  -H "Content-Type: application/json" \
  -d '{"status":"rejected"}'
```

```json
{"success":true,"data":{"id":"03513dd5-e821-4f26-870a-6364f2fad9e9","status":"rejected","match_score":0.9425641025641024,"match_quality":"probable","reviewed_at":"2026-08-04T07:18:31.632254Z","...":"..."}}
```

And confirm the clear-cut Okonkwo pair (id `42843791-...`), which we'll
merge next:

```sh
curl -s -X POST http://localhost:5150/api/persons/review-queue/42843791-bd11-4fce-9fdc-c55729d42fbe/decision \
  -H "Content-Type: application/json" \
  -d '{"status":"confirmed"}'
```

```json
{"success":true,"data":{"id":"42843791-bd11-4fce-9fdc-c55729d42fbe","status":"confirmed","match_score":0.9934065934065933,"match_quality":"certain","reviewed_at":"2026-08-04T07:18:48.515332Z","...":"..."}}
```

## 8. Merge

```sh
curl -s -X POST http://localhost:5150/api/persons/merge \
  -H "Content-Type: application/json" \
  -d '{"main_person_id":"26578248-52f2-45a0-8a4f-ebae2ba81b32","duplicate_person_id":"921c48ac-d073-49db-b235-4ad82f8278c3","merge_reason":"Confirmed duplicate — family-name transposition, same DOB (review-queue item 42843791)"}'
```

```json
{"success":true,"data":{
  "merge_record":{"id":"db716847-0853-460d-be1a-90de08e11524","main_person_id":"26578248-...","duplicate_person_id":"921c48ac-...","status":"completed","merge_reason":"Confirmed duplicate — family-name transposition, same DOB (review-queue item 42843791)","transferred_data":{},"merged_at":"2026-08-04T08:04:41.282681Z"},
  "main_person":{
    "id":"26578248-52f2-45a0-8a4f-ebae2ba81b32",
    "name":{"family":"Okonkwo","given":["Adaeze"],"use_type":null},
    "additional_names":[{"family":"Okonkow","given":["Adaeze"],"use_type":"old"}],
    "links":[{"other_person_id":"921c48ac-d073-49db-b235-4ad82f8278c3","link_type":"replaces"}],
    "...":"..."
  }
},"error":null}
```

Exactly what [`merge.md`](../agents/share/merge.md) documents: the
duplicate's primary name became an **"old" alias** on the survivor
(`additional_names`, `use_type: "old"`), and a **`Replaces`** link
(`link_type: "replaces"`) was added pointing at the duplicate. Note
`transferred_data` is `{}` here — real, not a bug in this capture: the
merge's transferred-data snapshot only records **identifiers** and
**tax_id**, and neither seeded record has any; the name alias and the
link are applied directly to the returned record but aren't mirrored
into that snapshot map.

### A defect this tutorial found and fixed, not just documented

The first time this merge was attempted, it failed:

```json
{"success":false,"data":null,"error":{"code":"DATABASE_ERROR","message":"Failed to merge persons: Database error: Query Error: error returned from database: new row for relation \"person_names\" violates check constraint \"patient_names_use_type_check\" at line 2076","details":null}}
```

`src/db/repositories.rs` wrote `NameUse`/`IdentifierUse`/
`ContactPointSystem`/`ContactPointUse`/`LinkType` via `format!("{:?}")`
— `"Old"`, `"Phone"`, `"Replaces"` — while their columns' CHECK
constraints accept only lowercase (`'old'`, `'phone'`, `'replaces'`).
This is the **same defect** `examples/data/README.md` already documents
for `telecom`/name `use_type` on the fixture data (tracked as
`PERSON-CONTACT-CASE` in the root `tasks.md`) — but merge hits it a
different, unconditional way: `merge_duplicate_into_main` always sets
the duplicate's aliased name to `NameUse::Old` and always adds a
`LinkType::Replaces` link, so **every** merge of two different persons
failed this way, regardless of which pair. No test caught it because no
existing test posts a name with `use_type` set, and the only merge test
in the suite is the self-merge-rejection guard, which never reaches the
insert.

Since this tutorial's entire premise depends on merge working, and there
was no way to route around an unconditional failure by choosing
different demo data, `src/db/repositories.rs` was fixed as part of this
work: the write side now uses the already-established `enum_to_tag`
helper (the same one `person_addresses`/emergency-contact tables always
used correctly) instead of `format!`, and the read side now uses
`tag_to_enum` instead of hand-rolled `PascalCase` match arms. This is a
**separate commit** from this tutorial, described in its own message and
in `tasks.md`'s `PERSON-CONTACT-CASE` entry — this file only stages
`tutorials/` and `tasks.md`.

### The duplicate is gone — but not as `active: false`

```sh
curl -s -i http://localhost:5150/api/persons/921c48ac-d073-49db-b235-4ad82f8278c3
```

```
HTTP/1.1 404 Not Found

{"success":false,"data":null,"error":{"code":"NOT_FOUND","message":"Person with id '921c48ac-d073-49db-b235-4ad82f8278c3' not found","details":null}}
```

Not `200` with `"active":false` in the body — `get_by_id` (and every
handler built on it) treats a soft-deleted record as absent, returning
`404`. The row still exists in Postgres (`deleted_at` is set, nothing is
actually erased — that's what `POST /{id}/erase` is for, a distinct,
GDPR-Art.17 operation), but the REST surface answers exactly as it would
for an id that never existed.

## 9. Audit trail

```sh
curl -s "http://localhost:5150/api/persons/26578248-52f2-45a0-8a4f-ebae2ba81b32/audit"
```

```json
{"success":true,"data":[
  {"seq":6,"action":"UPDATE","entity_type":"Person","entity_id":"26578248-...",
   "old_values":{"additional_names":[],"links":[],"...":"..."},
   "new_values":{"additional_names":[{"family":"Okonkow","use_type":"old","...":"..."}],"links":[{"link_type":"replaces","other_person_id":"921c48ac-..."}],"...":"..."},
   "hash":"15cabdf2181f32835d6bd69970525efb5755b0eb62dcfc67db1fcd2f90236b37",
   "prev_hash":"fad5b66c06fe5c05441115c0f165d3d7467f8c8c0e7a07546588fa458234feaf","...":"..."},
  {"seq":1,"action":"UPDATE","entity_type":"Person","entity_id":"26578248-...",
   "old_values":{"updated_at":"2026-08-04T07:51:25.300170Z","...":"..."},
   "new_values":{"updated_at":"2026-08-04T07:17:43.650089Z","...":"..."},
   "hash":"87789dee4ec98bde0a39e1888c5490709a97e148a4a4c1ae3651cd54f066b285",
   "prev_hash":null,"...":"..."}
]}
```

Two rows, not the "create + merge" pair you might expect — a genuine,
live-verified finding rather than an assumption. `seq 1` is the no-op
`PUT` reindex from step 2 (the seed task writes no audit row at all, by
its own documented design), and `seq 6` is the merge — the survivor's
`additional_names`/`links` visibly changing in `old_values`/
`new_values`. Each row chains to the previous one's `hash` via
`prev_hash` (this is the tamper-evident chain
[`compliance-for-healthcare.md`](../agents/share/compliance-for-healthcare.md)
§2.1 describes); `seq 1`'s `prev_hash: null` marks it as the first row
ever written for this record.

The system-wide view fills in what happened in between, including the
duplicate's own soft-delete as a distinct row:

```sh
curl -s "http://localhost:5150/api/audit/recent?limit=10"
```

```
seq 7  DELETE          Person         921c48ac-...  (the duplicate's soft-delete)
seq 6  UPDATE          Person         26578248-...  (the merge)
seq 5  review_decision review_queue   42843791-...  (confirmed Okonkwo)
seq 4  review_decision review_queue   03513dd5-...  (rejected Nakamura)
seq 3  UPDATE          Person         195503f4-...  (Ren's reindex PUT)
seq 2  UPDATE          Person         1e8ebf4f-...  (Kenji's reindex PUT)
seq 1  UPDATE          Person         26578248-...  (Okonkwo's reindex PUT)
```

Note what's *not* there: no `read`/`disclosure` rows, despite several
plain `GET`s against these same records over the course of this
tutorial. `agents/share/compliance-for-healthcare.md` §2.1 calls out
read-auditing as a HIPAA expectation, and the handler code does call a
`disclosure::record_access` hook on reads — but nothing from those calls
shows up in either `/audit/recent` or the per-person `/audit` endpoint
in this run. Worth knowing if you're relying on this trail for a
§164.528 accounting: verify what your own deployment's reads actually
produce rather than assuming from the code alone.

## 10. The same lifecycle through the UI

Install and type-check the front-end, then point its BFF at the running
backend. **Port 5150 is person-service's own dev-mode default**, which
happens to be the front-end's default too — no override needed for the
backend port, only for `AUTH_API_URL` (also 5150, since
authentication-service isn't running for this tutorial and nothing here
needs it).

```sh
cd person/person-front-end-with-svelte
pnpm install
pnpm check
```

```
1 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
```

This front-end's `.env.example` (`PUBLIC_API_BASE_URL=...`) is stale —
the same class of gotcha TUT-1 found in the case front-end. The
variables the code actually reads
(`src/lib/server/config.ts`, matching the front-end's own `README.md`
table) are `PERSON_API_URL` and `AUTH_API_URL`:

```sh
cat > .env <<'EOF'
PERSON_API_URL=http://localhost:5150
AUTH_API_URL=http://localhost:5150
EOF
pnpm dev
```

```
VITE v6.4.2  ready in 349 ms

  ➜  Local:   http://localhost:5173/
```

Open **http://localhost:5173/review**. With the backend state left from
the curl walkthrough above, the board shows all five pairs: Okonkwo
*confirmed* (already merged via curl), Nakamura *rejected*, and three
still *pending* — Halloran, Bergström, and Achterberg.

In the browser:

1. Click the **Halloran** card (or its row in the keyboard-accessible
   table below the Kanban board) — the comparison panel opens below,
   fetching both records (`GET /api/persons/{id}` for each side — there
   is no combined "fetch the pair" endpoint) and rendering the matcher's
   per-component score breakdown.
2. Click **Confirm**.
3. Click through the panel's merge deep-link,
   `/persons/merge?main=…&duplicate=…` — both ids arrive pre-filled but
   stay editable, since a review item names an unordered pair and the
   service doesn't designate a survivor.
4. Click **Merge**.

To prove this path is real rather than describe it on faith, the same
two calls the UI's own `PersonRepository` makes were run directly
against the front-end's **own server** (`/api/proxy/...` — the
same-origin BFF proxy every browser call goes through; see
[`authentication-sessions.md`](../agents/share/authentication-sessions.md)
§6 for why the browser never calls person-service directly):

```sh
curl -s -X POST http://localhost:5173/api/proxy/api/persons/review-queue/0678f250-7ce4-44c4-8a9e-8a61a92fadfa/decision \
  -H "Content-Type: application/json" -d '{"status":"confirmed"}'
```

```json
{"success":true,"data":{"id":"0678f250-...","status":"confirmed","...":"..."}}
```

```sh
curl -s -X POST http://localhost:5173/api/proxy/api/persons/merge \
  -H "Content-Type: application/json" \
  -d '{"main_person_id":"a0ea036d-81ab-418d-b1fa-3903d3fbb851","duplicate_person_id":"aabda370-cc31-4642-a581-6b5678c5f815","merge_reason":"Confirmed via the /review board — review item 0678f250"}'
```

```json
{"success":true,"data":{
  "merge_record":{"main_person_id":"a0ea036d-...","duplicate_person_id":"aabda370-...","status":"completed","...":"..."},
  "main_person":{"name":{"family":"Halloran","given":["Bill"]},"additional_names":[{"family":"Halloran","given":["William"],"use_type":"old"}],"links":[{"link_type":"replaces","other_person_id":"aabda370-..."}],"...":"..."}
},"error":null}
```

Bill Halloran survives with William's name folded in as an `"old"`
alias — a **different pair** from the Okonkwo merge done via curl in
step 8, proving both the curl and the UI paths independently, on real
distinct data. And the duplicate is gone through the same proxy that
served the merge:

```sh
curl -s -o /dev/null -w "%{http_code}\n" \
  http://localhost:5173/api/proxy/api/persons/aabda370-cc31-4642-a581-6b5678c5f815
```

```
404
```

When you're done, stop the dev server (`Ctrl-C`).

## 11. Tear down

```sh
# stop the front-end dev server (Ctrl-C if running in the foreground)

# stop person-service (Ctrl-C, or kill the backgrounded process)

# stop and remove the test Postgres
scripts/test-db.sh down person/person-service-with-loco
```

`test-db.sh down` drops the tmpfs-backed container entirely, so the next
`up` starts from a genuinely empty database.

## What's next

- **TUT-3 — authentication & ABAC**: magic-link sign-in, session cookie,
  `POST /token`, a protected call, the 401/403 matrix, writing and
  hot-reloading an ABAC policy, and the `mask` obligation.
- **TUT-4 — cross-service linking**: `subject_of` and `same_identity`
  writes, then querying the link-graph aggregator's `neighbors` /
  `single-view` / `freshness`, plus a break-and-reconcile demo.
- **TUT-5 — bulk import/export**: fixture import (dry-run, error report),
  idempotent re-import, masked vs. full export.
- **TUT-6 — event bus**: outbox rows, the relay, `/events/recent`.

See [`tasks.md`](../tasks.md) for their current status.
