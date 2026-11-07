# Runbook: bulk import/export job failure recovery

The OPS-1 slice for `bulk_jobs` (person, organization, case, care-pathway;
see [`bulk-import-export.md`](../bulk-import-export.md) for the design). Read
§6–§7 of that doc first for the intended import semantics and error
model; this runbook is what to actually do when a job doesn't behave
that way.

## The states, and how a job stops being stuck

A job is one of `queued`, `running`, `completed`,
`completed_with_errors`, or `failed`. Care-pathway adds a sixth, `cancelled`.

Two tables track a job. `bulk_jobs` is what clients see, and `axa_queue` is
axa's own queue. Nothing links them by foreign key: a queue row belongs to a job
when its `name` is the worker's class name and `args->>'job_id'` is the job's
id. The worker is `BulkJobWorker`, or `BulkExportWorker` for a care-pathway FHIR
`$export`, which shares the table.

If a worker dies mid-run (an OOM kill, or a deploy that terminates the
container), recovery goes in three steps:

1. **The heartbeat stops.** A running job updates its `axa_queue` row every
   minute. `bulk_jobs.updated_at` does not move, so an old value there means
   nothing by itself.
2. **axa gives up.** A worker process sweeps the queue at start and every five
   minutes. A row not heard from for `stale_minutes` (60) is queued again if it
   has attempts left, and otherwise marked `failed` with the error `abandoned by
   its worker`. These jobs get **one attempt**, because neither worker raises
   `max_attempts`, so a lost job is failed rather than repeated.
3. **`bulk_jobs` follows.** `crate::bulk::reconcile` runs in every process that
   serves the queue (`start --worker` or `start --all`), at start and every five
   minutes. It moves a `queued` or `running` job to `failed` when no queue row
   for it is alive and either a queue row has `failed`, or it has no queue row
   at all and has not changed for 15 minutes. It logs `bulk job lost by the job
   queue; marked failed` at `ERROR`.

So a job lost with its worker reaches `failed` about 70 minutes after the last
heartbeat at worst, **provided a worker process is running**. All of this runs
in workers. With none running, a job stays as it is, and the service warns at
boot that nothing serves its queue.

A job axa has requeued keeps its live queue row, so it is not failed while it
still has attempts. Nor is a slow job, however old its `bulk_jobs.updated_at`,
because its heartbeat keeps the queue row alive.

The 15 minutes exist because the `bulk_jobs` row commits before its job is
queued, so a young job with no queue row may be mid-request. The longest such
request is 10 minutes. An older one was never queued (`perform_later` failed
after the row committed), or its failed queue row was purged after seven days.
It cannot be a finished job whose row was purged, because a finished job's own
row is already terminal and is never selected.

**On the first deploy of this reconciliation**, any job already stuck moves to
`failed` within five minutes of a worker starting. That is correct, since
nothing would ever have run those jobs.

## What "failed" actually tells you, and what it doesn't

`bulk_jobs` has **no error-message column** on person, organization, and case.
Care-pathway has one, `error`, though its native status endpoints don't return
it, so read it with SQL. When a job fails, the worker logs `bulk job {id} failed:
{e}` at `ERROR` and does a best-effort write of `failed`. If the thing that
broke was the database connection, that write fails too, and reconciliation
later moves the row once axa has recorded the failure. A care-pathway FHIR
`$export` logs differently: `bulk export failed`, with the job id as a field.

The other place to look is `axa_queue.error`, which keeps the failure text for
seven days after the queue row finishes:

```sql
SELECT status, attempts, max_attempts, error, updated_at
FROM axa_queue WHERE args->>'job_id' = '<job id>';
```

A job failed by reconciliation has no `bulk job {id} failed:` line, because its
worker never got that far. Its error is in `axa_queue.error`, or there is no
queue row at all, in which case look for the failed request around the job's
`created_at`.

A worker that axa gave up on may still be alive, for example if it lost the
queue connection but not the job's. If it finishes, it overwrites `failed` with
its real outcome, and axa logs `job finished after being given up on`. No status
write checks the current status, so trust the later value: the data landed.

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
backend (the default, and the only backend organization and case implement)
it is literally a `file://<path>` string — you retrieve it by reading
that path directly on the server's disk, not by fetching a URL. Only
the S3 backend (person and care-pathway, built with the `s3` feature) could
produce a fetchable presigned URL, and no handler in this codebase calls
`presigned_get` to produce one — so even there, today, you're reading
the object directly from the bucket.

## Recovering from a lost job

There is no supported way to run an existing job again: no admin endpoint, no
CLI task, and no code path that puts a `bulk_jobs` row back in a runnable state.
Nothing tracks progress within a job, so there is nothing to resume.

1. **Confirm it's lost, not merely slow.** Read its queue row with the query
   above. `running` with a recent `updated_at` means it is alive, whatever
   `bulk_jobs` says. `MAX_IMPORT_ROWS` (1,000,000) and `MAX_IMPORT_BYTES`
   (64 MiB) bound how long it can plausibly take.
2. **If it's dead, wait or don't.** It will reach `failed` by itself as described
   above. You don't need to wait for that before resubmitting.
3. **Check whether a worker is running** if a job stays `queued` with a queue row
   also `queued`. That is an unserved queue, not a lost job. Start a worker and
   the job runs.
4. **Resubmit the file as a new request, with a new `Idempotency-Key` or none.**
   The old key still names the old job, whatever its status, so reusing it
   returns the failed job and runs nothing. Leave the old row as a forensic
   record.

## Resubmission is safe — but two different mechanisms make it safe, for two different things

**Fixing bad rows and re-uploading the corrected file** is safe because
of **stable-key upsert**, not the idempotency key: person keys on a
strong-typed identifier (SSN/TAX/NPI/PPN, in that preference order) or a
`tax_id` fallback, else the row's own `pid`; organization on an LEI, then a
DUNS, else `pid`; case on the pair `(agency_id, case_number)`, else `pid`;
care-pathway on a deterministic identifier (DOI, Wikidata, guideline id, URI,
UUID), else `pid`. A row that already landed
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

On person, case, and care-pathway, an unrecognised
`<ENTITY>_BULK_ARTIFACT_BACKEND` value doesn't error — it logs `unknown
<ENTITY>_BULK_ARTIFACT_BACKEND; falling back to the local store` at WARN and
proceeds on local disk. If you were expecting S3 and artifacts are landing on
local disk instead, check for a typo'd backend value and this exact warning.
Asking for `s3` where it isn't available is an error rather than a fallback.
Organization doesn't read this variable at all: it always uses local disk.

## Checks

**There are no bulk-specific Prometheus metrics** — none of the four
crates export a job-count, running-duration, or failure-rate metric for
bulk jobs. Detection is manual:

| Check | How | What it tells you |
|---|---|---|
| Recent jobs | `GET /api/<plural>/bulk-jobs?limit=` | Newest-first list. Person, organization, and case take `limit` only, despite the design doc describing filters. Care-pathway also takes `kind` and `status` |
| Job and queue row side by side | `SELECT b.id, b.status, b.updated_at, q.status AS queue_status, q.attempts, q.updated_at AS heartbeat, q.error FROM bulk_jobs b LEFT JOIN axa_queue q ON q.args->>'job_id' = b.id::text WHERE b.status IN ('queued', 'running') ORDER BY b.created_at;` | Your stuck-job list. A row with `queue_status` of `running` and an old `heartbeat` is about to be given up on. `queue_status` of `queued` means it is waiting for a worker |
| Jobs reconciliation moved | grep logs for `bulk job lost by the job queue` | Each job moved to `failed`, with its `job_id` |
| Reconciliation failing | grep logs for `could not reconcile bulk jobs` | The pass itself failed. It tries again in five minutes |
| Why a specific job failed | grep logs for `bulk job {id} failed:`, then read `axa_queue.error` | The error text, while the queue row lasts. Care-pathway also has `bulk_jobs.error` |
| A misrouted artifact backend | grep `unknown <ENTITY>_BULK_ARTIFACT_BACKEND; falling back to the local store` | Confirms artifacts are landing on local disk despite an intended S3 config |

## The concurrency gap on organization, case, and care-pathway

Only person's per-row upsert is advisory-lock-protected. Organization,
case, and care-pathway don't take that lock (on case it's an explicit design
choice). Two imports racing the **same** stable key at the same instant can both
miss the existing-record lookup and both create a row — a genuine duplicate, not
a bug in the matcher. **If you run concurrent imports against these services,
serialize them yourself** (one at a time, or partition by known-disjoint stable
keys); there is no code-level protection to fall back on. If you see an
unexplained duplicate appear right after two overlapping bulk imports, this is
almost certainly why — resolve it the same way any other duplicate is resolved
(the matcher + merge workflow), not as a data-corruption incident.

## Symptoms → checks → actions

**"A job has been `running` for way longer than it should."**
Read its queue row. If the heartbeat is recent, it is still working: axa can't
tell a slow job from a hung one, so a job that hangs keeps its heartbeat and
holds its worker until the process stops. If the heartbeat is old, it will reach
`failed` within about 70 minutes of the last one. Resubmit as a new job with a
new key; the stable-key upsert makes redoing rows free.

**"A job is `failed` but I don't know why."**
Grep `bulk job {id} failed:`, then read `axa_queue.error`, and on care-pathway
`bulk_jobs.error`. `abandoned by its worker` means the process died or lost the
database. If you also need to confirm whether partial data landed (relevant for
the disk-full-on-error-report case above and for a crash mid-import), query the
target table directly for rows matching the job's known stable keys. A failed
job may have committed some of its rows.

**"I resubmitted with the same key and got the failed job back."**
That's the idempotency key doing its job: it names that job for good. Use a new
key.

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
treat the newer, redundant one accordingly (only care-pathway has a cancel,
on its FHIR `$export` status endpoint; elsewhere the job either already ran or
is running).

**"Artifacts are showing up somewhere I didn't expect."**
Check for the `unknown …_BULK_ARTIFACT_BACKEND; falling back to the
local store` warning — a typo'd backend value silently uses local disk
rather than erroring.

## What this runbook cannot help you do

- **Resume a lost job from where it left off.** No progress is tracked within a
  job; the only recovery is a fresh submission.
- **Query jobs by status or kind through the API**, except on care-pathway. Use
  direct SQL against `bulk_jobs` for anything more targeted.
- **Retrieve an error report or export artifact via a URL**, on the
  local-filesystem backend (the default everywhere, and the only backend
  organization and case have). You need direct access to the server's
  filesystem.
- **Protect a concurrent organization, case, or care-pathway import from
  creating a duplicate record.** Serialize these yourself until the
  advisory-lock gap is closed.

These are gaps in the system, not steps you're missing — file them as
follow-up work if bulk-job operations become frequent enough for any of
them to matter in practice.
