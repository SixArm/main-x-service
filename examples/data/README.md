# Main X Index — example data fixtures

Synthetic JSONL fixtures for the tutorials and the demo seed path (EX-1
in the repository root [`tasks.md`](../../tasks.md)). Three importable
files plus one mapping document:

| File | Rows | Target service | Import endpoint |
|---|---:|---|---|
| [`persons.jsonl`](persons.jsonl) | 50 | person-service | `POST /api/persons/import` |
| [`organizations.jsonl`](organizations.jsonl) | 20 | organization-service | `POST /api/organizations/import` |
| [`cases.jsonl`](cases.jsonl) | 10 | case-service | `POST /api/cases/import` |
| [`case-subject-links.md`](case-subject-links.md) | 10 edges | case-service | `POST /api/cases/{pid}/links` (not bulk-importable) |

## Provenance — read this before using the data anywhere

**Every value in these files is invented.** No name, address, telephone
number, email address, identifier, tax reference, document number, case
number, or company name was sampled, copied, or derived from any real
person, real organization, real registry, or real dataset. There is no
real PII here, and nothing here is a redacted or perturbed version of
anything real.

**Generation method: written by an AI coding assistant (Claude) directly
for this repository's tutorials.** Not sourced from a public dataset, not
scraped, not synthesised from a real corpus, not statistically modelled
on one. The names were chosen to span a range of scripts and naming
conventions so the matcher components (Jaro-Winkler, Soundex, the
diacritic and hyphenation paths) get exercised on something other than
Anglophone input — that variety is a deliberate test property, not a
sample of any real population.

Where a field format is a real, checkable standard, the values sit in
ranges reserved for fiction or never issued:

| Field | Convention used | Why |
|---|---|---|
| US telephone | `+1 555 01xx` | the `555-01xx` block is reserved for fictional use |
| UK telephone | `+44 20 7946 0xxx` | Ofcom's reserved drama range |
| Email / URL | `example.org`, `example.com`, `.example` | RFC 2606 / RFC 6761 reserved for documentation |
| US SSN | area `000` (`000-31-4728`) | area 000 is never issued by the SSA |
| NPI | `9999999925` | real NPIs begin `1` or `2` |
| LEI | `999900MXIDEMO…` | `9999` is not an issued LOU prefix — see below |
| GLN | `999000000001`-series | `999` is not an allocated GS1 prefix |
| VAT | `DE999999999`, `FR99999999901`, `IE9999999XA` | all-9 bodies |
| DUNS | `9000000xx` | 9-prefixed, not issued |
| Case numbers, agency ids | `example-*`, `…-000101` | agencies are all named `Example …` |

**The LEIs and GLNs carry genuinely valid check digits**, because the
organization service validates them (SEC-M5): LEI by ISO 7064 MOD 97-10,
GLN by the GS1 mod-10 weighting. A made-up LEI is a `422`, so these were
computed. They are structurally valid and semantically meaningless — they
identify nothing.

## Loading into a running service — the `seed_examples` task (EX-4)

The reliable way to get this data into a database today is each
service's **`seed_examples` loco task**, not bulk import (see the
warning in [Importing](#importing) below — the demo compose stack's
bulk-import path is currently broken, tracked as
[`COMPOSE-WORKER`](../../tasks.md)). The task reads the same fixture
files documented here and inserts through the **model layer** directly
— bypassing the `POST /api/<plural>` create endpoint's real-time
duplicate detection, which would otherwise return `409` on the second
half of every one of `persons.jsonl`'s five duplicate pairs (confirmed
live by EX-1) and silently drop them. No audit row or event is written
by the seed itself; the tutorials that exercise duplicate detection,
audit, and events do so against the seeded records afterward, not
against the act of seeding. Each task refuses to insert into a
non-empty table, so re-running it is a no-op rather than a duplicate
load.

Run from each service crate's own directory, against its own database
(`DATABASE_URL` / `config/development.yaml` as usual):

```sh
cd person/person-service-with-loco       && cargo loco task seed_examples
cd organization/organization-service-with-loco && cargo loco task seed_examples
cd case/case-service-with-loco           && cargo loco task seed_examples
```

This seeds 50 persons, 20 organizations, and 10 cases. It does **not**
create the ten `subject_of` links documented in
[`case-subject-links.md`](case-subject-links.md) — those require the
seeded records' pids, which are not known until after both the person
and case tasks have run; create them afterward via
`POST /api/cases/{pid}/links` as that file describes.

Once the tutorials TUT-1/TUT-2/TUT-4 exist (`tasks.md`), they should
reference `cargo loco task seed_examples` in each of the three crates
as the seed step, rather than the bulk-import path below.

## Importing

Each service's bulk import is multipart, async, and returns a job id
([`bulk-import-export.md`](../../agents/share/bulk-import-export.md) §4).
Ports below are the `full-family.yml` ones from
[`examples/compose/README.md`](../compose/README.md).

```sh
# Dry run first: parses, validates, and classifies every row, commits nothing.
curl -F file=@examples/data/persons.jsonl -F format=jsonl -F dry_run=true \
  http://localhost:8081/api/persons/import

# For real. Note person wraps its response (`.data.job_id`) while
# organization and case return `{job_id}` bare.
JOB=$(curl -s -F file=@examples/data/persons.jsonl -F format=jsonl \
  http://localhost:8081/api/persons/import | jq -r '.data.job_id')
curl -s "http://localhost:8081/api/persons/import/${JOB}" | jq

curl -F file=@examples/data/organizations.jsonl -F format=jsonl \
  http://localhost:8087/api/organizations/import
curl -F file=@examples/data/cases.jsonl -F format=jsonl \
  http://localhost:8089/api/cases/import
```

`format` defaults to `jsonl` when omitted. Poll
`GET /api/<plural>/import/{job_id}` for `rows_created` / `rows_upserted` /
`rows_to_review` / `rows_errored` and the error-report URL.

> **Update 2026-08-04: fixed.** Earlier, all three commands above were
> run against a live `full-family.yml` stack and all three were
> accepted, returning a job id — but the job stayed `queued` forever.
> The containers started with `CMD [..., "start"]`, which is loco's
> server-only mode; `BulkJobWorker` is registered in `connect_workers`,
> which only runs under `start --server-and-worker`. Both compose files
> now set `command: ["/app/<bin>", "start", "--server-and-worker"]` on
> every service (`COMPOSE-WORKER`, `tasks.md`), live-verified against
> `single-service.yml`/case-service: a real `cases.jsonl` import reached
> `completed` with `rows_created: 10`. TUT-5 verifies the same fix's
> effect via `cargo run -- start --server-and-worker` directly (faster
> to iterate against than a container rebuild) against person-service.
> `cargo loco task seed_examples`
> ([above](#loading-into-a-running-service--the-seed_examples-task-ex-4))
> remains a valid, even simpler way to load these fixtures when you
> don't need the async job semantics themselves.

Then create the ten `subject_of` edges — see
[`case-subject-links.md`](case-subject-links.md).

### Re-importing is safe, for the rows that carry a stable key

| File | Stable key | Keyed rows | Keyless rows |
|---|---|---:|---:|
| `persons.jsonl` | first `SSN` / `TAX` / `NPI` / `PPN` identifier, else `tax_id`, else explicit `id` | 7 | 43 |
| `organizations.jsonl` | `Lei`, else `Duns`, else explicit `pid` | 15 | 5 |
| `cases.jsonl` | `(agency_id, case_number)`, else explicit `pid` | 8 | 2 |

A keyed row **upserts in place** on re-import — that is the idempotency
demo for TUT-5. A keyless row has no handle, so it runs the entity's
duplicate detection instead and, on a likely match, is still created but
also queued for review with `provenance = "import"`. Re-importing the
whole file therefore grows the keyless population every time; that is
correct behaviour, not a fixture bug, and it is worth showing.

No two rows in any file share a stable key, and no two person rows share
an identifier `(system, value)` pair — the latter matters because the
person schema has a `UNIQUE (system, value)` constraint that would turn a
collision into a database error rather than a clean upsert.

## The duplicate pairs

`persons.jsonl` contains **five** deliberate duplicate pairs. All ten
rows are keyless by construction — no `id`, no `tax_id`, no strong-typed
identifier — so they cannot silently upsert into one row on import; they
have to surface through duplicate detection, which is the point.

Scores below are what the person service's own `ProbabilisticScorer`
actually returns, not estimates.

| Lines | Person | What differs | Score | Quality |
|---|---|---|---:|---|
| 1 & 4 | Adaeze Okonkwo | family name transposed (`Okonkwo` / `Okonkow`) | 0.9934 | certain |
| 7 & 9 | William / Bill Halloran | given name is a known nickname variant | 0.9908 | certain |
| 12 & 14 | Annika Bergström | diacritic dropped, birth date one day apart | 0.9705 | certain |
| 17 & 19 | Ren / Kenji Nakamura | **different given name**, same family + birth date | 0.9426 | **probable** |
| 21 & 23 | Marta Achterberg | same person, street number `12` vs `12A` | 0.9995 | certain |

The 17 & 19 pair is deliberately the ambiguous one: it lands in the
*probable* band rather than *certain*, so a tutorial has something that
genuinely warrants an operator decision rather than an auto-merge. It is
also the pair where a reviewer might reasonably decide **not** to merge.

Two design constraints made these pairs work, both worth knowing before
adding more:

- **The blocking query gates everything.** Candidates come from a Tantivy
  `FuzzyTermQuery` on the *family name* with max edit distance 2. Two
  records whose family names are 3+ edits apart are never scored at all,
  no matter how identical the rest is. Every pair above is within 2.
- **Components only count when both sides have them.** Address,
  identifier, tax-ID and document weights are renormalised away when
  either side lacks the field. Giving one row of a pair an identifier the
  other lacks is therefore neutral; giving both rows *different*
  identifiers actively drags the score down.

No unintended pair anywhere in the file scores above 0.70.

## Known omissions, and why

**`persons.jsonl` carries no top-level `telecom`, and no `use_type` on
names or identifiers.** Not a stylistic choice — those fields cannot
currently be persisted by person-service. `src/db/repositories.rs` writes
them with `format!("{:?}", …)`, producing `"Phone"` / `"Official"`, while
the migration's CHECK constraints require lowercase (`'phone'`,
`'official'`). Any person carrying either field is rejected at the
database with a `500 DATABASE_ERROR`, not a `422`. The newer
`person_emergency_contact_telecom` table is unaffected — it goes through
`enum_to_tag` (serde, correct case) and has no CHECK — so **emergency
contacts in this file do carry telephone and email**, and person
addresses carry `use_type`.

This is a pre-existing service defect, not a fixture problem; it is
recorded as follow-up work in the root [`tasks.md`](../../tasks.md) EX-1
entry. When it is fixed, contact details should be added back here.

Also absent by design: no `relationships` or `tags` field exists on the
person wire type (its equivalents are `links` and `emergency_contacts`),
and `cases.jsonl` carries no inline person reference — see
[`case-subject-links.md`](case-subject-links.md).

## How these files were verified

Every row of all three files was run through **the services' own code**,
not a schema guess:

1. **Offline, all 80 rows** — each file's real `bulk::jsonl::parse_line`,
   its `validation` module (the same validators the import pipeline calls
   per row), and its `bulk::stable_key` resolver; plus, for persons, the
   real `ProbabilisticScorer` over all 1 225 pairs to confirm the five
   intended pairs score in band and nothing else does.
2. **Live, against Postgres** — a representative person sample through
   `POST /api/persons` (minimal rows, the fully-populated rows, and each
   pair's first half), then each pair's second half asserting `409
   DUPLICATE_DETECTED`; and **all 20 organizations and all 10 cases**
   created and read back field-by-field. Plus the `subject_of` link, its
   `GET`-back, and the `422` on a non-person target.

The live pass is what caught the `use_type` / `telecom` defect above: the
offline validators pass those rows, and only a real database rejects
them.

The harnesses were temporary and are not checked in — these fixtures are
data, and adding a permanent test to three service crates to police them
would be a three-part spec change in each. If a fixture is edited, re-run
the checks described in the root `tasks.md` EX-1 entry.
