# Bulk import/export: dry-run, error report, idempotent re-import, and masked-vs-full export

This tutorial exercises
[`agents/share/bulk-import-export.md`](../agents/share/bulk-import-export.md)
end to end on **person-service** — the family's bulk-import **reference
implementation** (`agents/share/overview.md`'s capability matrix) — using
[`examples/data/persons.jsonl`](../examples/data/persons.jsonl) (EX-1's 50
synthetic persons, five of them deliberate duplicate pairs): a **dry-run**
import that commits nothing, a **real** import, a small deliberately-broken
file to show the **error report**, an **idempotent re-import** of the same
50 rows, and a **masked vs. full export** with the `403` an unprivileged
caller gets on the unmasked profile once `PERSON_REQUIRE_AUTH` is on.

## The blocker this tutorial needed fixed first

Every job submitted here would have accepted a `202` and then sat in
`queued` forever. `examples/compose/{single-service,full-family}.yml`
started every container with loco's server-only `CMD [..., "start"]`,
which never registers `BulkJobWorker` (loco only wires background workers
under `start --server-and-worker`) — found live by EX-1, tracked as
`COMPOSE-WORKER`, and fixed in its own commit immediately before this one.
See [`tasks.md`](../tasks.md) `COMPOSE-WORKER` for that fix's own live
verification (against `single-service.yml`/case-service).

**This tutorial does not reuse that compose fix directly**, though — it
runs person-service **locally** via `cargo run -- start --server-and-worker`,
the same way TUT-2 and TUT-3 found is what actually works in this
environment (no `cargo-loco` shim installed). That flag is the exact one
`COMPOSE-WORKER` added to the compose files; running it locally exercises
the identical `StartMode::ServerAndWorker` code path loco's CLI parses
into, just without a container in between. Local `cargo run` is also
simply faster to iterate steps 2–5 below against, and step 5 needs a
second service (authentication-service) restarted mid-tutorial with a
different flag anyway — a pattern TUT-3 already established locally.

## Prerequisites

| Tool | Why | Tested with |
|---|---|---|
| [Podman](https://podman.io/) (not Docker) | only for the throwaway test Postgres instances | 6.0.2, with `podman machine` running |
| Rust (this repo pins `1.96.1` in [`rust-toolchain.toml`](../rust-toolchain.toml)) | builds and runs person-service and authentication-service directly | `cargo` on `PATH` |
| `curl` + `python3` (for `python3 -m json.tool`) | verifies everything live | whatever your OS ships |

## 1. Start Postgres and person-service, in worker mode

```sh
scripts/test-db.sh up person/person-service-with-loco
```

```
test-db: mxi-person-test-db ready
  DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test
```

```sh
cd person/person-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test
cargo run -- db migrate
```

Now start the server **with the worker registered** — the local
equivalent of the compose fix above:

```sh
cargo run -- start --server-and-worker
```

```
environment: development
   database: logging, automigrate
     logger: debug
compilation: debug
      modes: server, worker

listening on http://0.0.0.0:5150
worker is online
```

`modes: server, worker` (not just `server`) and `worker is online` are
the tells — this is exactly what was missing from the compose stack.
Confirm it's actually up:

```sh
curl -s http://localhost:5150/api/health
```

```json
{"status":"healthy","service":"person-service","version":"0.5.0"}
```

## 2. Dry-run import — and a real, live-verified gotcha with intra-batch duplicates

`POST /api/persons/import` with `dry_run=true` parses, validates, and
classifies every row without committing anything
([`bulk-import-export.md`](../agents/share/bulk-import-export.md) §6):

```sh
cd person/person-service-with-loco   # if not already there
curl -s -F file=@../../examples/data/persons.jsonl -F format=jsonl -F dry_run=true \
  http://localhost:5150/api/persons/import
```

```json
{"success":true,"data":{"job_id":"c5c69dfa-fb38-42bc-a276-522a90b26582"},"error":null}
```

```sh
curl -s "http://localhost:5150/api/persons/import/c5c69dfa-fb38-42bc-a276-522a90b26582"
```

```json
{"success":true,"data":{"id":"c5c69dfa-fb38-42bc-a276-522a90b26582","kind":"import","entity":"person","format":"jsonl","status":"completed","rows_total":50,"rows_processed":50,"rows_created":50,"rows_upserted":0,"rows_to_review":0,"rows_errored":0,"download_url":null,"errors_url":null},"error":null}
```

Confirm nothing was actually persisted:

```sh
curl -s "http://localhost:5150/api/persons/search?q=&limit=1"
```

```json
{"success":true,"data":{"persons":[],"total":0,"query":"","offset":0,"limit":1}}
```

(A bare `q=` always returns `total: 0` regardless of what's in the
database — Tantivy's empty-query match is empty, not match-all. A real,
mildly surprising quirk found live in this run; use a real search term,
not an empty one, if you want a true row-count signal. Section 4 below
uses one.)

**The genuinely surprising finding**: `rows_to_review` is `0` here, not
`5` — even though `persons.jsonl` carries five deliberate duplicate
pairs and the dry-run code path (`src/bulk/pipeline.rs`) does run the
same `keyless_duplicate` classification the real import uses. Read
`process_import_job`'s dry-run branch closely and the reason is exact:
dry-run **never writes a row**, so when it reaches the *second* half of
a duplicate pair, the *first* half — which exists only earlier in this
same file, not yet anywhere in the database — was never persisted for
it to find. Duplicate detection queries the database (and search index),
not the in-flight batch, so a dry run against an **empty database**
cannot see intra-file duplicates at all; every one of the 50 rows
classifies as a plain `create`. This is not a bug so much as an
inherent limit of "validate and classify without committing": the
moment two rows only duplicate *each other*, dry-run's own
non-persistence removes the evidence needed to catch it. Worth knowing
before trusting a dry-run's `rows_to_review` as a preview of what a
real import will queue — it only reflects duplicates against data that
already exists **in the database before the dry run starts**, not
duplicates first introduced within the file itself.

## 3. The real import — the five pairs actually surface this time

Same file, no `dry_run`. This time each row commits before the next is
processed, so the second half of each pair *does* find the first:

```sh
curl -s -F file=@../../examples/data/persons.jsonl -F format=jsonl \
  http://localhost:5150/api/persons/import
```

```json
{"success":true,"data":{"job_id":"58e05990-43b4-4513-ac21-a288cda65b60"},"error":null}
```

Poll until terminal:

```sh
curl -s "http://localhost:5150/api/persons/import/58e05990-43b4-4513-ac21-a288cda65b60"
```

```json
{"success":true,"data":{"id":"58e05990-43b4-4513-ac21-a288cda65b60","kind":"import","entity":"person","format":"jsonl","status":"completed","rows_total":50,"rows_processed":50,"rows_created":50,"rows_upserted":0,"rows_to_review":5,"rows_errored":0,"download_url":null,"errors_url":null},"error":null}
```

Real, observed timing: five successive polls (3 s apart) still read
`running`; the sixth, roughly 15–18 s after submission, read `completed`
— duplicate detection (search-index blocking query + the full
`ProbabilisticScorer`) and Tantivy indexing per row is real work, unlike
the dry run's classification-only pass.

`rows_created: 50` with `rows_to_review: 5` is exactly
[`match-search-merge.md`](../agents/share/match-search-merge.md)'s
documented behaviour: a keyless row with a likely duplicate is **still
created** — never silently dropped — and *also* queued for review.
Confirm the review queue:

```sh
curl -s "http://localhost:5150/api/persons/review-queue?limit=10" | python3 -m json.tool | head -20
```

```json
{
    "items": [
        {
            "id": "efab01bd-a246-47a6-ad4d-26504f8aee15",
            "person_id_a": "3898399f-6e25-43d0-a303-f9dbbcdc44ca",
            "person_id_b": "a597539f-dd96-4e52-8140-9afa0593b351",
            "match_score": 0.9995,
            "match_quality": "certain",
            "detection_method": "import_duplicate_detection",
            "status": "pending",
            "provenance": "import",
            ...
        },
        ...
```

Five `pending` items, `provenance: "import"` — matching the vocabulary
`cross-service-linking.md` and `bulk-import-export.md` §6 both name, and
`detection_method: "import_duplicate_detection"` distinguishing these
from an interactive `POST /persons/deduplicate` scan.

## 3b. The error report — a deliberately broken second file

`persons.jsonl` imports clean (EX-1 verified every row passes
validation), so demonstrating the **error report**
([`bulk-import-export.md`](../agents/share/bulk-import-export.md) §7)
needs a small file with real, deliberate problems: an empty required
field, a future birth date, and a malformed JSON line.

```sh
cat > /tmp/persons-errors.jsonl <<'EOF'
{"name":{"family":"","given":["NoFamily"]},"gender":"female"}
{"name":{"family":"FutureBorn","given":["Tom"]},"gender":"male","birth_date":"2099-01-01"}
{this is not valid json at all}
{"name":{"family":"Valid","given":["Sara"]},"gender":"female","birth_date":"1990-05-05"}
EOF
curl -s -F file=@/tmp/persons-errors.jsonl -F format=jsonl \
  http://localhost:5150/api/persons/import
```

```json
{"success":true,"data":{"job_id":"07f7beba-5f5d-4387-8b91-0f50a1f4f234"},"error":null}
```

Completed in well under two seconds (a four-row file, no duplicate
detection to speak of):

```sh
curl -s "http://localhost:5150/api/persons/import/07f7beba-5f5d-4387-8b91-0f50a1f4f234"
```

```json
{"success":true,"data":{"id":"07f7beba-5f5d-4387-8b91-0f50a1f4f234","kind":"import","entity":"person","format":"jsonl","status":"completed_with_errors","rows_total":4,"rows_processed":4,"rows_created":1,"rows_upserted":0,"rows_to_review":0,"rows_errored":3,"download_url":null,"errors_url":"file:///private/var/folders/.../person-bulk-artifacts/jobs/07f7beba.../errors.csv"},"error":null}
```

`status: "completed_with_errors"` (not `failed`) — exactly the "one
valid row committing must never abort the load" contract. Fetch the
real error report (`errors_url` is a `file://` reference under this
run's local `PERSON_BULK_ARTIFACT_DIR`; a deployment with the `s3`
feature would instead issue a presigned HTTP download):

```sh
cat /private/var/folders/.../person-bulk-artifacts/jobs/07f7beba.../errors.csv
```

```csv
row_number,field,code,message
1,name.family,validation,Family name is required
2,birth_date,validation,Birth date cannot be in the future
3,,parse,key must be a string at line 1 column 2
```

Real, worth noting: row 1's report carries only the `name.family` error
even though `validate_person` also checks "at least one given name" —
that check passed here (`given: ["NoFamily"]` is non-empty), so only the
one failing rule surfaces; a row failing several validators at once
would report each as its own line, keyed by the same `row_number`. Row
3's `parse`-coded message is Serde's own JSON error text, passed through
verbatim rather than re-worded. Row 4 (`Sara`) is the one valid row and
is the `rows_created: 1`.

## 4. Idempotent re-import — keyed rows upsert, keyless rows grow

Submit `persons.jsonl` **again**, unchanged:

```sh
curl -s -F file=@../../examples/data/persons.jsonl -F format=jsonl \
  http://localhost:5150/api/persons/import
```

```json
{"success":true,"data":{"job_id":"0bf11a2a-e492-4625-a5c2-049917b74565"},"error":null}
```

```sh
curl -s "http://localhost:5150/api/persons/import/0bf11a2a-e492-4625-a5c2-049917b74565"
```

```json
{"success":true,"data":{"id":"0bf11a2a-e492-4625-a5c2-049917b74565","kind":"import","entity":"person","format":"jsonl","status":"completed","rows_total":50,"rows_processed":50,"rows_created":43,"rows_upserted":7,"rows_to_review":43,"rows_errored":0,"download_url":null,"errors_url":null},"error":null}
```

This is
[`examples/data/README.md`](../examples/data/README.md#re-importing-is-safe-for-the-rows-that-carry-a-stable-key)'s
documented split, live-verified exactly: `persons.jsonl` has **7** rows
with a stable key (a strong-typed identifier, `tax_id`, or explicit
`id`) and **43** keyless rows. `rows_upserted: 7` — the keyed rows
updated their own existing record in place, no duplicates. `rows_created:
43, rows_to_review: 43` — every keyless row has **no handle to upsert
against**, so it runs ordinary duplicate detection, finds *itself* (the
copy from step 3, now genuinely present in the database this time — the
opposite of section 2's gap), and is created again as a **new** row
while also being queued for review. Confirm the review queue grew from 5
to 48 (`5 + 43`):

```sh
curl -s "http://localhost:5150/api/persons/review-queue?limit=100" \
  | python3 -c "import json,sys; print(len(json.load(sys.stdin)['data']['items']))"
```

```
48
```

Re-importing the whole file is **safe for the keyed rows and not a
no-op for the keyless ones** — it grows the keyless population every
time, exactly as documented, not a fixture bug. "Idempotent" here means
"upsert-by-key is idempotent," not "the whole file is a no-op on
replay" — worth being precise about, since the two are easy to conflate.

## 5. Masked vs. full export, and the `403` on ungated full

### With auth off (today's default): both profiles work

```sh
curl -s -X POST http://localhost:5150/api/persons/export \
  -H "Content-Type: application/json" -d '{"format":"jsonl"}'
```

```json
{"success":true,"data":{"job_id":"fe07cb6a-e825-43f4-a8a2-014a27fd1166"},"error":null}
```

Poll to completion (`rows_total: 94` — the 50 from step 3, +1 from
step 3b's `Sara`, +43 from step 4), then read the masked output for a
row that carries a real identifier:

```sh
curl -s "http://localhost:5150/api/persons/export/fe07cb6a-.../"   # → download_url
grep -m1 Wainscott /path/to/export.jsonl | python3 -m json.tool
```

```json
{
  "identifiers": [
    {"identifier_type": "SSN", "value": "***-**-4728", "...": "..."},
    {"identifier_type": "MRN", "value": "MRN-EX-000108", "...": "..."}
  ],
  "documents": [{"document_type": "PASSPORT", "number": "*****0108", "...": "..."}]
}
```

`mask_person` (`src/privacy/mod.rs`) redacts SSN/TAX/PPN/DL identifiers
and document numbers to their last four characters; MRN is untouched
(not in its masked-type list). Now the **full** profile, still with auth
off — `export_requires_elevation` gates this on `authorize_record`,
which is a documented no-op when `PERSON_REQUIRE_AUTH` is off:

```sh
curl -s -X POST http://localhost:5150/api/persons/export \
  -H "Content-Type: application/json" -d '{"format":"jsonl","masking_profile":"full"}'
# … poll, then …
grep -m1 Wainscott /path/to/export.jsonl | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['identifiers'])"
```

```
[{'identifier_type': 'SSN', 'value': '000-31-4728', ...}, ...]
```

Real SSN, unmasked — confirming §8's "full is privileged" is enforced
by the ABAC gate specifically, not by anything else standing in for it
when that gate is off.

### Turn on `PERSON_REQUIRE_AUTH` — the matrix

This needs a real PASETO, so start authentication-service the same way
TUT-3 did (its own test Postgres, on a free port since person already
holds 5150 for its own server):

```sh
TEST_DB_PORT=5433 scripts/test-db.sh up authentication/authentication-service-with-loco
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5433/authentication_service_test cargo run -- db migrate
cat > config/development.local.yaml <<'EOF'
server:
  port: 5151
EOF
DATABASE_URL=postgres://loco:loco@localhost:5433/authentication_service_test cargo run -- start
```

```
environment: development
listening on http://localhost:5151
```

`PERSON_REQUIRE_AUTH` is read once at boot
([`security.md`](../agents/share/security.md) §4), so restart
person-service (`Ctrl-C` the step-1 process) pointed at the running
authentication-service's published keys:

```sh
cd person/person-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5432/person_service_test \
  PERSON_REQUIRE_AUTH=true \
  PERSON_PASETO_KEYS_URL=http://localhost:5151/.well-known/paseto-keys \
  cargo run -- start --server-and-worker
```

```
PASETO key set fetched at boot; fetched set overrides PERSON_PASETO_KEYS
  url=http://localhost:5151/.well-known/paseto-keys key_count=1
```

The 94 persons from before are untouched (same `DATABASE_URL`; only the
process restarted). **No token, either profile:**

```sh
curl -s -i -X POST http://localhost:5150/api/persons/export \
  -H "Content-Type: application/json" -d '{"format":"jsonl","masking_profile":"full"}'
```

```
HTTP/1.1 401 Unauthorized
missing authorization header
```

Same `401` for the default masked profile too — the blanket guard
requires *some* valid token on every `/api/*` path once the flag is on,
before the export-specific elevation check ever runs.

Sign up, verify the magic link (same mechanism TUT-3 walks in depth),
and get a token with **no `attrs`** — mirroring TUT-3's own finding, the
console log carries the link because `cargo run -- start` boots as
`environment: development` with no override:

```sh
curl -s -X POST http://localhost:5151/api/auth/signup \
  -H "Content-Type: application/json" -d '{"email":"tut5@example.com","name":"TUT5 Demo"}'
grep "magic link issued" /tmp/auth-service.log | tail -1
curl -s "http://localhost:5151/api/auth/magic-link/<token-from-the-log>"
```

**That token, no `attrs`, either profile — both `403`:**

```sh
TOKEN=v4.public...
curl -s -i -X POST http://localhost:5150/api/persons/export \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"format":"jsonl"}'
curl -s -i -X POST http://localhost:5150/api/persons/export \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"format":"jsonl","masking_profile":"full"}'
```

```
HTTP/1.1 403 Forbidden   (both)
```

Real, worth being precise about: `POST /api/persons/export` is **not**
one of person's `DESTRUCTIVE_POST_SUFFIXES` (`/merge`, `/deduplicate`,
`/import`, `/erase`) — the blanket guard derives its action as plain
`write`, denied here under the built-in default policy's
read-allow/mutation-deny rule regardless of masking profile. So *both*
profiles are blocked by the blanket guard at this attribute level, not
yet by the export-specific elevation check.

**`access=write` (the CLI task revokes the session — SEC-A8 — so a
fresh sign-in is required for a new token, same as TUT-3):**

```sh
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5433/authentication_service_test \
  cargo run -- task user_attributes op:set email:tut5@example.com key:access values:write
# … fresh magic-link sign-in + new TOKEN …
curl -s -i -X POST http://localhost:5150/api/persons/export \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"format":"jsonl"}'
curl -s -i -X POST http://localhost:5150/api/persons/export \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"format":"jsonl","masking_profile":"full"}'
```

```
masked → HTTP/1.1 202 Accepted
full   → HTTP/1.1 403 Forbidden   {"error":{"code":"FORBIDDEN","message":"default deny"}}
```

**This is the tutorial's title finding, live-verified precisely**: with
`access=write`, the *masked* export is accepted (`write` is granted) but
the *full* export is still denied — `export_requires_elevation` demands
`Action::Destructive` specifically, and the default policy's
`access=write` rule grants `write` only, not `destructive`. Write does
not imply destructive; `authorization-attributes.md` §2 states this
("delete implies destructive... a rule targeting only write does not
cover delete") and this is the same asymmetry applied to a masking
profile instead of an HTTP verb.

**`access=admin` — full export finally succeeds:**

```sh
cd authentication/authentication-service-with-loco
DATABASE_URL=postgres://loco:loco@localhost:5433/authentication_service_test \
  cargo run -- task user_attributes op:set email:tut5@example.com key:access values:admin
# … fresh magic-link sign-in + new TOKEN …
curl -s -X POST http://localhost:5150/api/persons/export \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"format":"jsonl","masking_profile":"full"}'
```

```json
{"success":true,"data":{"job_id":"5bf83818-a22c-4266-81c3-65966ad9190d"},"error":null}
```

```sh
curl -s -H "Authorization: Bearer $TOKEN" "http://localhost:5150/api/persons/export/5bf83818-a22c-4266-81c3-65966ad9190d"
```

```json
{"success":true,"data":{"id":"5bf83818-a22c-4266-81c3-65966ad9190d","kind":"export","entity":"person","format":"jsonl","status":"completed","rows_total":94,"rows_processed":94,"...":"..."},"error":null}
```

`200`/`202` all the way through, `rows_total: 94` — the full,
authorized export of everything created across steps 3–4. The complete,
live-verified matrix:

| Caller | masked export | full export |
|---|---|---|
| no token | 401 | 401 |
| token, no `attrs` | 403 | 403 |
| `access=write` | 202 | 403 |
| `access=admin` | 202 | 202 |

## 6. Tear down

```sh
# Ctrl-C both cargo run -- start processes (person-service, authentication-service)
scripts/test-db.sh down person/person-service-with-loco
TEST_DB_PORT=5433 scripts/test-db.sh down authentication/authentication-service-with-loco
rm -f /tmp/persons-errors.jsonl /tmp/person-service.log /tmp/auth-service.log
rm -f authentication/authentication-service-with-loco/config/development.local.yaml
```

`test-db.sh down` drops the tmpfs-backed Postgres containers entirely
(same clean-slate guarantee as every prior tutorial's teardown); the
bulk artifacts under `PERSON_BULK_ARTIFACT_DIR` (the OS temp dir by
default — nothing was overridden in this run) are not tracked files and
can be left for the OS to reclaim.

## What's next

- **TUT-6 — event bus**: outbox rows, the relay, `/events/recent`.

See [`tasks.md`](../tasks.md) for its current status.
