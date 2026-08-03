# Runbook: bulk import/export job failure recovery

The OPS-1 slice for `bulk_jobs` (person, organization, case — see
[`bulk-import-export.md`](../bulk-import-export.md) for the design). Read
§6–§7 of that doc first for the intended import semantics and error
model; this runbook is what to actually do when a job doesn't behave
that way.

## The five states, and the one that can last forever

A job is exactly one of `queued`, `running`, `completed`,
`completed_with_errors`, `failed` — no more. **There is no
heartbeat, no lease, no started-at-vs-now staleness check anywhere in
this code.** If the worker process handling a job dies mid-run — an
OOM kill, a deploy that terminates the container, a panic the job
runner doesn't catch — that job's row sits in `running` **forever**.
Nothing in this codebase will ever move it out of that state on its
own.

(loco's own Postgres-backed queue has an *opt-in* reaper that can
requeue stale `Processing` rows — but it is off by default, and no
service in this family turns it on. Don't assume it's protecting you
unless you specifically enabled it.)

## What "failed" actually tells you — and what it doesn't

`bulk_jobs` has **no error-message column**. When a job fails, the
worker logs `bulk job {id} failed: {e}` at `ERROR` and does a best-effort
`set_status(..., Failed)` — best-effort in the literal sense: if the
thing that broke was the database connection itself, the write that
would mark the job `failed` can silently fail too, leaving the row
stuck in `running` with no record anywhere in `bulk_jobs` that anything
went wrong. **The log line is the only place the actual error text
lives** — check there first, always, before assuming the failure was
data-related just because the status says `failed`.

(The underlying error is *also* captured in loco's own job-queue table,
`pg_loco_queue.task_data->>'error'`, if you need a second source — the
`bulk_jobs` row's status and that queue row's status are two separate
pieces of bookkeeping that can, in principle, disagree.)

## Per-row errors — the actual format

The error report is **CSV only** (despite the design doc mentioning
JSONL as an option — not implemented). Four columns, in this exact
order:

```
row_number,field,code,message
```

`code` is one of exactly `parse`, `validation`, `database`. `field` is
blank for a whole-line parse failure or a database-layer failure.
`row_number` is 1-based.

**The report is only written when there's at least one error** — a
job with zero row errors has no `errors_url` at all, which is the
correct read of "field absent" here, not a bug to investigate.

**There is no download endpoint.** `errors_url` (and `download_url` for
exports) is an opaque artifact-store reference. On the local-filesystem
backend (the default, and the only backend organization/case implement)
it is literally a `file://<path>` string — you retrieve it by reading
that path directly on the server's disk, not by fetching a URL. Only
the S3 backend (person only) can in principle produce a fetchable
presigned URL, and no handler in this codebase actually calls
`presigned_get` to produce one — so even there, today, you're reading
the object directly from the bucket.

## Recovering from a stuck (`running` forever) job

There is no supported "unstick" operation — no admin endpoint, no CLI
task, no code path that clears an existing `bulk_jobs` row back to a
runnable state. In practice:

1. Confirm it's actually stuck, not merely slow: check whether the
   worker process that would be running it is still alive, and whether
   `MAX_IMPORT_ROWS` (1,000,000) or `MAX_IMPORT_BYTES` (64 MiB) make its
   expected runtime plausible for the file size actually submitted.
2. If the process is gone, the row is orphaned — there is nothing to
   wait for. Note its `job_id` and move on to resubmission (below);
   don't spend time trying to "resume" it, since nothing in this
   codebase tracks partial progress within a job. Leave the row as a
   forensic record rather than deleting it, unless you have a specific
   reason to reclaim the idempotency-key slot (§ below).
3. Resubmit the file as a **new** request. If you want the retry to be
   safe against your own client having *also* retried the original
   submission (not just the file), see idempotency below — but a stuck
   `running` row does not, by itself, block a fresh submission from
   proceeding.

## Resubmission is safe — but two different mechanisms make it safe, for two different things

**Fixing bad rows and re-uploading the corrected file** is safe because
of **stable-key upsert**, not the idempotency key: person keys on a
strong-typed identifier (SSN/TAX/NPI/PPN, in that preference order) or a
`tax_id` fallback, else the row's own `pid`; case keys on the pair
`(agency_id, case_number)`, else `pid`. A row that already landed
correctly the first time around simply upserts back onto itself when you
resubmit the whole (fixed) file — that's the "operator fixes the error
file and re-submits" loop the design doc describes, and it costs nothing
to redo the good rows.

**Retrying an HTTP submission your client isn't sure landed** — a
timeout, a dropped connection — is a *different* problem, solved by the
`Idempotency-Key` request header, not by stable-key upsert. Send the
same key on the retried `POST /import` (or `/export`) and you get back
the **same job**, unprocessed a second time, rather than a duplicate
job silently doing the work twice. This only suppresses re-enqueueing
the *job* — it has nothing to do with whether individual *rows* upsert
cleanly, which is the stable-key mechanism above. Don't conflate the
two: an idempotency key protects you from double-submitting a request;
a stable key protects you from double-creating a record.

**Keyless rows** (no strong identifier, no explicit `pid`) are never
deduplicated by idempotency key and are **always created**, never
silently dropped — they're additionally queued into the review queue
with `provenance = "import"` when their similarity to an existing
record crosses the family's duplicate threshold, exactly like an
interactively-created record would be.

## Artifact-store failures — these fail loud, which is what you want

Both backends propagate a real error message on disk-full, permission
issues, or a bucket problem (`write artifact {key}: {e}`, `put artifact
{key}: {e}`) rather than silently dropping bytes — such a failure
becomes an ordinary job `failed`. One asymmetry worth knowing on
**import**: rows are committed to the database *before* the error
report is written, so if the error-report write itself fails (disk full
at exactly the wrong moment, say), the job shows `failed` with **no
counts at all** even though the data load actually succeeded. If you
hit this, don't assume `failed` means the import didn't happen — check
whether the records actually landed before deciding whether to
resubmit.

An unrecognised `<ENTITY>_BULK_ARTIFACT_BACKEND` value doesn't error —
it logs `unknown <ENTITY>_BULK_ARTIFACT_BACKEND; falling back to the
local store` at WARN and proceeds on local disk. If you were expecting
S3 and artifacts are landing on local disk instead, check for a typo'd
backend value and this exact warning.

## Checks

**There are no bulk-specific Prometheus metrics** — none of the three
crates export a job-count, running-duration, or failure-rate metric for
bulk jobs. Detection is entirely manual:

| Check | How | What it tells you |
|---|---|---|
| Recent jobs | `GET /api/<plural>/bulk-jobs?limit=` | Newest-first list — **note it takes `limit` only, no `status`/`kind` filter**, despite the design doc describing one; you're eyeballing the list, not querying it |
| Piled-up / stuck jobs | direct SQL: `SELECT id, status, created_at, updated_at FROM bulk_jobs WHERE status = 'running' ORDER BY created_at;` (the `bulk_jobs_kind_status`/`_status_idx` index makes this cheap) | Anything `running` for far longer than its row/byte count justifies is your stuck-job list |
| Why a specific job failed | grep logs for `bulk job {id} failed:` | The **only** place the real error text lives — `bulk_jobs` itself has no error column |
| A misrouted artifact backend | grep `unknown <ENTITY>_BULK_ARTIFACT_BACKEND; falling back to the local store` | Confirms artifacts are landing on local disk despite an intended S3 config |

## Organization's known concurrency gap — a real, documented, unfixed risk

Organization's per-row upsert is **not** advisory-lock-protected the way
person's is (case has the same class of gap, by explicit design choice,
not oversight). Two imports racing the **same** stable key (LEI, DUNS,
or `pid`) at the same instant can both miss the existing-record lookup
and both create a row — a genuine duplicate, not a bug in the matcher.
**If you run concurrent imports against organization or case, serialize
them yourself** (one at a time, or partition by known-disjoint stable
keys) until this is fixed; there is no code-level protection to fall
back on. If you see an unexplained duplicate organization or case
appear right after two overlapping bulk imports, this is almost
certainly why — resolve it the same way any other duplicate is
resolved (the matcher + merge workflow), not as a data-corruption
incident.

## Symptoms → checks → actions

**"A job has been `running` for way longer than it should."**
Check whether its worker process is even alive. If it's gone, the row
is permanently orphaned — there is no recovery for the row itself.
Resubmit the file as a new job; the stable-key upsert makes redoing any
rows that did complete free.

**"A job is `failed` but I don't know why."**
The reason is in the logs, not in `bulk_jobs` — grep `bulk job {id}
failed:`. If you also need to confirm whether partial data landed
before the failure (relevant for both the disk-full-on-error-report
case above and an ordinary mid-import crash), query the target table
directly for rows matching the job's known stable keys.

**"I fixed the error file and resubmitted, but I'm not sure whether the
good rows will get re-processed or skipped."**
They'll get re-processed and upsert back onto themselves — this is
free and expected, not wasted work. It's the deliberately-designed
recovery loop, not a workaround.

**"My client retried the same upload after a timeout and now I'm
worried about a duplicate job."**
Only prevented if you sent the same `Idempotency-Key` header on both
attempts — if you didn't, you may now have two jobs. Check
`bulk-jobs?limit=` for two recent jobs against the same file/time and
treat the newer, redundant one accordingly (there's no built-in
cancel — the job either already ran or is running).

**"Artifacts are showing up somewhere I didn't expect."**
Check for the `unknown …_BULK_ARTIFACT_BACKEND; falling back to the
local store` warning — a typo'd backend value silently uses local disk
rather than erroring.

## What this runbook cannot help you do

- **Resume a stuck job from where it left off.** No progress is
  tracked within a job; the only recovery is a fresh submission.
- **Query jobs by status or kind through the API.** The list endpoint
  is `limit`-only; use direct SQL against `bulk_jobs` for anything more
  targeted.
- **Retrieve an error report or export artifact via a URL**, on the
  local-filesystem backend (the default everywhere, and the only backend
  organization and case have). You need direct access to the server's
  filesystem.
- **Protect a concurrent organization or case import from creating a
  duplicate record.** Serialize these yourself until the advisory-lock
  gap is closed.

These are gaps in the system, not steps you're missing — file them as
follow-up work if bulk-job operations become frequent enough for any of
them to matter in practice.
