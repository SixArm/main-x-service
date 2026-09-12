<!--
  Bulk import / export (/bulk) — upload a file of care pathways, or
  extract a filtered set, and watch the resulting background job (§13
  T-10; `agents/share/bulk-import-export.md`).

  Both submits return `202 {job_id}`; the page then polls the job's status
  endpoint until it reaches a terminal state, showing the row-count
  breakdown as it fills in.

  Two deliberate gaps, both service-side rather than UI omissions:
    - `download_url` / `errors_url` are **opaque artifact-store references**
      (`file://…` / `s3://…`), and the service exposes no endpoint that
      serves their bytes — so they are rendered as plain text, not links.
    - `include_soft_deleted` is not offered: the endpoint accepts it but the
      worker rejects it, so the job would be accepted and then fail.

  Also out of scope here (a documented follow-up, not an oversight): the
  duplicate **review queue** a keyless import row feeds
  (`GET/POST .../review-queue`) has no UI yet — this page only submits and
  monitors bulk jobs, mirroring person's own bulk page's scope.

  State:
    - importPanel / exportPanel — the submitted job being polled.
    - jobs / kindFilter / statusFilter — the recent-jobs table; this
      service's `bulk-jobs` endpoint filters `kind`/`status` server-side,
      so changing either re-queries rather than filtering client-side.
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { CarePathwayRepository } from "$lib/api/care-pathways";
  import { ApiError } from "$lib/api/client";
  import { t } from "$lib/i18n.svelte";
  import {
    BULK_IMPORT_FORMATS,
    BULK_FORMATS,
    BULK_JOB_STATUSES,
    MASKING_PROFILES,
    POLL_INTERVAL_MS,
    isTerminalStatus,
    progressPercent,
    type BulkFormat,
    type BulkImportFormat,
    type MaskingProfile,
  } from "$lib/bulk";
  import type { BulkJobView } from "$lib/api/types";

  const repo = CarePathwayRepository.withFetch();

  /** A submitted job plus the client-side context the wire type lacks. */
  interface JobPanel {
    jobId: string;
    job: BulkJobView | null;
    /** Tracked here because `BulkJobView` carries no timestamp at all. */
    submittedAt: Date;
    /** Whether this import was a dry run (also absent from the view). */
    dryRun: boolean;
    polling: boolean;
    error: string | null;
  }

  // ─── Import ────────────────────────────────────────────────────────
  let importFile = $state<File | null>(null);
  let importFormat = $state<BulkImportFormat>("jsonl");
  let dryRun = $state(false);
  let importSubmitting = $state(false);
  let importError = $state<string | null>(null);
  let importPanel = $state<JobPanel | null>(null);

  // ─── Export ────────────────────────────────────────────────────────
  let exportFormat = $state<BulkFormat>("jsonl");
  let exportQuery = $state("");
  let exportLimit = $state("");
  let maskingProfile = $state<MaskingProfile>("masked");
  let exportSubmitting = $state(false);
  let exportError = $state<string | null>(null);
  let exportPanel = $state<JobPanel | null>(null);

  // ─── Recent jobs ───────────────────────────────────────────────────
  let jobs = $state<BulkJobView[]>([]);
  let jobsLoading = $state(false);
  let jobsError = $state<string | null>(null);
  let kindFilter = $state("");
  let statusFilter = $state("");

  // Set on unmount so an in-flight poll loop stops instead of updating a
  // destroyed component (and retrying forever in the background).
  let destroyed = false;
  onDestroy(() => (destroyed = true));

  onMount(loadJobs);

  /** Human message for a thrown error, preferring the API's own code. */
  function describe(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  /** Translated label for a wire status token; unknown tokens pass through. */
  function statusLabel(status: string): string {
    switch (status) {
      case "queued":
        return t("bulk.status.queued");
      case "running":
        return t("bulk.status.running");
      case "completed":
        return t("bulk.status.completed");
      case "completed_with_errors":
        return t("bulk.status.completedWithErrors");
      case "failed":
        return t("bulk.status.failed");
      default:
        return status;
    }
  }

  /** Translated label for a wire kind token; unknown tokens pass through. */
  function kindLabel(kind: string): string {
    if (kind === "import") return t("bulk.kind.import");
    if (kind === "export") return t("bulk.kind.export");
    return kind;
  }

  /** Translated label for a wire format token; unknown tokens pass through. */
  function formatLabel(format: string): string {
    switch (format) {
      case "jsonl":
        return t("bulk.format.jsonl");
      case "csv":
        return t("bulk.format.csv");
      case "tsv":
        return t("bulk.format.tsv");
      default:
        return format;
    }
  }

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  /**
   * Poll `panel`'s job until it reaches a terminal state.
   *
   * Stops on unmount, on a supersede (the operator submitted another job
   * of the same kind), and on any error. A 404 is reported as expired
   * rather than crashing: the service answers 404 both for a job past its
   * retention TTL and for one belonging to another actor, and does not
   * distinguish the two.
   */
  async function pollJob(
    panel: JobPanel,
    fetchJob: (id: string) => Promise<BulkJobView>,
    current: () => JobPanel | null,
  ) {
    panel.polling = true;
    // Supersede check: `current()` re-reads the live panel each pass, so
    // a newly submitted job silently retires this loop.
    while (!destroyed && current()?.jobId === panel.jobId) {
      try {
        const job = await fetchJob(panel.jobId);
        panel.job = job;
        if (isTerminalStatus(job.status)) break;
      } catch (err) {
        panel.error =
          err instanceof ApiError && err.isNotFound
            ? t("bulk.error.expired")
            : describe(err);
        break;
      }
      await sleep(POLL_INTERVAL_MS);
    }
    panel.polling = false;
  }

  /** Capture the chosen file off the input's change event. */
  function onFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    importFile = input.files?.[0] ?? null;
  }

  /** Submit the upload, then start polling its job. */
  async function startImport() {
    if (!importFile) {
      importError = t("bulk.import.fileRequired");
      return;
    }
    importSubmitting = true;
    importError = null;
    try {
      // A fresh key per submit: the service dedupes a *retried* submit
      // on this header (SEC-B9), so reusing one across distinct uploads
      // would silently return the earlier job instead of importing.
      const accepted = await repo.importPathways(importFile, {
        format: importFormat,
        dryRun,
        idempotencyKey: crypto.randomUUID(),
      });
      importPanel = {
        jobId: accepted.job_id,
        job: null,
        submittedAt: new Date(),
        dryRun,
        polling: true,
        error: null,
      };
      void pollJob(
        importPanel,
        (id) => repo.getImportJob(id),
        () => importPanel,
      );
      void loadJobs();
    } catch (err) {
      importError = describe(err);
    } finally {
      importSubmitting = false;
    }
  }

  /** Submit the export request, then start polling its job. */
  async function startExport() {
    exportSubmitting = true;
    exportError = null;
    try {
      const parsedLimit = Number.parseInt(exportLimit, 10);
      const accepted = await repo.exportPathways(
        {
          format: exportFormat,
          q: exportQuery.trim() || undefined,
          limit:
            Number.isFinite(parsedLimit) && parsedLimit > 0
              ? parsedLimit
              : undefined,
          masking_profile: maskingProfile,
        },
        crypto.randomUUID(),
      );
      exportPanel = {
        jobId: accepted.job_id,
        job: null,
        submittedAt: new Date(),
        dryRun: false,
        polling: true,
        error: null,
      };
      void pollJob(
        exportPanel,
        (id) => repo.getExportJob(id),
        () => exportPanel,
      );
      void loadJobs();
    } catch (err) {
      // Includes the 403 a `full` masking profile draws without
      // elevated authorisation — an inline banner, not a crash.
      exportError = describe(err);
    } finally {
      exportSubmitting = false;
    }
  }

  /** (Re)load the recent-jobs table, applying the current filters
   *  server-side. */
  async function loadJobs() {
    jobsLoading = true;
    jobsError = null;
    try {
      jobs = await repo.listBulkJobs({
        kind: kindFilter || undefined,
        status: statusFilter || undefined,
      });
    } catch (err) {
      jobsError = describe(err);
    } finally {
      jobsLoading = false;
    }
  }
</script>

<svelte:head><title>{t("bulk.head.title")}</title></svelte:head>

<header>
  <h1>{t("bulk.title")}</h1>
  <p class="muted">{t("bulk.intro")}</p>
</header>

<!-- Reusable job panel: progress, the row-count breakdown, and the two
     artifact references (as text — the service exposes no endpoint that
     serves their bytes). -->
{#snippet jobPanel(panel: JobPanel)}
  <section class="surface stack" aria-label={t("bulk.job.title")}>
    <h2>{t("bulk.job.title")}</h2>
    {#if panel.dryRun}
      <div class="banner success">{t("bulk.import.dryRunNotice")}</div>
    {/if}
    <dl class="kv">
      <dt>{t("bulk.job.id")}</dt>
      <dd><code>{panel.jobId}</code></dd>
      <dt>{t("bulk.job.submittedAt")}</dt>
      <dd>{panel.submittedAt.toLocaleString()}</dd>
      <dt>{t("bulk.job.status")}</dt>
      <dd>
        {panel.job ? statusLabel(panel.job.status) : t("bulk.job.polling")}
      </dd>
      {#if panel.job}
        {@const pct = progressPercent(
          panel.job.rows_processed,
          panel.job.rows_total,
        )}
        <dt>{t("bulk.job.progress")}</dt>
        <dd>
          {panel.job.rows_processed}
          {#if pct === null}
            · {t("bulk.job.unknownTotal")}
          {:else}
            / {panel.job.rows_total} · {pct}%
          {/if}
        </dd>
        <dt>{t("bulk.job.rowsCreated")}</dt>
        <dd>{panel.job.rows_created}</dd>
        <dt>{t("bulk.job.rowsUpserted")}</dt>
        <dd>{panel.job.rows_upserted}</dd>
        <dt>{t("bulk.job.rowsToReview")}</dt>
        <dd>{panel.job.rows_to_review}</dd>
        <dt>{t("bulk.job.rowsErrored")}</dt>
        <dd>{panel.job.rows_errored}</dd>
      {/if}
    </dl>
    {#if panel.polling}<p class="muted small">
        {t("bulk.job.polling")}
      </p>{/if}
    {#if panel.job?.download_url}
      <div>
        <div class="artifact-label">{t("bulk.artifact.output")}</div>
        <code class="artifact">{panel.job.download_url}</code>
        <p class="muted small">{t("bulk.artifact.note")}</p>
      </div>
    {/if}
    {#if panel.job?.errors_url}
      <div>
        <div class="artifact-label">{t("bulk.artifact.errors")}</div>
        <code class="artifact">{panel.job.errors_url}</code>
        <p class="muted small">{t("bulk.artifact.note")}</p>
      </div>
    {/if}
    {#if panel.error}<div class="banner error" role="alert">
        {panel.error}
      </div>{/if}
  </section>
{/snippet}

<section class="surface stack">
  <h2>{t("bulk.import.title")}</h2>
  <label
    >{t("bulk.import.file")}
    <input
      type="file"
      accept=".jsonl,.csv,.tsv,application/jsonl,application/json,text/csv,text/tab-separated-values"
      onchange={onFileChange}
    />
    <small>{t("bulk.import.fileHint")}</small>
  </label>
  <div class="row">
    <label
      >{t("bulk.import.format")}
      <select bind:value={importFormat}>
        {#each BULK_IMPORT_FORMATS as fmt (fmt)}
          <option value={fmt}>{formatLabel(fmt)}</option>
        {/each}
      </select>
      <small>{t("bulk.import.formatHint")}</small>
    </label>
  </div>
  <label class="check">
    <input type="checkbox" bind:checked={dryRun} />
    {t("bulk.import.dryRun")}
  </label>
  <small class="hint">{t("bulk.import.dryRunHint")}</small>
  <div class="row">
    <button
      type="button"
      class="button primary"
      onclick={startImport}
      disabled={importSubmitting}
    >
      {importSubmitting ? t("bulk.import.submitting") : t("bulk.import.submit")}
    </button>
  </div>
  {#if importError}<div class="banner error" role="alert">
      {importError}
    </div>{/if}
</section>

{#if importPanel}
  {@render jobPanel(importPanel)}
{/if}

<section class="surface stack">
  <h2>{t("bulk.export.title")}</h2>
  <div class="row">
    <label
      >{t("bulk.export.format")}
      <select bind:value={exportFormat}>
        {#each BULK_FORMATS as fmt (fmt)}
          <option value={fmt}>{formatLabel(fmt)}</option>
        {/each}
      </select>
      <small>{t("bulk.export.formatHint")}</small>
    </label>
    <label
      >{t("bulk.export.masking")}
      <select bind:value={maskingProfile}>
        {#each MASKING_PROFILES as profile (profile)}
          <option value={profile}>
            {profile === "masked"
              ? t("bulk.masking.masked")
              : t("bulk.masking.full")}
          </option>
        {/each}
      </select>
      <small>{t("bulk.export.maskingHint")}</small>
    </label>
  </div>
  <div class="row">
    <label
      >{t("bulk.export.query")}
      <input bind:value={exportQuery} />
      <small>{t("bulk.export.queryHint")}</small>
    </label>
    <label
      >{t("bulk.export.limit")}
      <input type="number" min="1" bind:value={exportLimit} />
      <small>{t("bulk.export.limitHint")}</small>
    </label>
  </div>
  <div class="row">
    <button
      type="button"
      class="button primary"
      onclick={startExport}
      disabled={exportSubmitting}
    >
      {exportSubmitting ? t("bulk.export.submitting") : t("bulk.export.submit")}
    </button>
  </div>
  {#if exportError}<div class="banner error" role="alert">
      {exportError}
    </div>{/if}
</section>

{#if exportPanel}
  {@render jobPanel(exportPanel)}
{/if}

<section class="surface stack">
  <h2>{t("bulk.jobs.title")}</h2>
  <div class="row">
    <label class="inline">
      {t("bulk.jobs.filterKind")}
      <select bind:value={kindFilter} onchange={loadJobs}>
        <option value="">{t("bulk.jobs.all")}</option>
        <option value="import">{t("bulk.kind.import")}</option>
        <option value="export">{t("bulk.kind.export")}</option>
      </select>
    </label>
    <label class="inline">
      {t("bulk.jobs.filterStatus")}
      <select bind:value={statusFilter} onchange={loadJobs}>
        <option value="">{t("bulk.jobs.all")}</option>
        {#each BULK_JOB_STATUSES as status (status)}
          <option value={status}>{statusLabel(status)}</option>
        {/each}
      </select>
    </label>
    <button type="button" class="button" onclick={loadJobs}
      >{t("bulk.jobs.refresh")}</button
    >
  </div>
  <p class="muted small">{t("bulk.jobs.orderNote")}</p>
  {#if jobsError}<div class="banner error" role="alert">{jobsError}</div>{/if}
  {#if jobsLoading}
    <p class="muted">{t("bulk.jobs.loading")}</p>
  {:else if jobs.length === 0}
    <p class="muted">{t("bulk.jobs.empty")}</p>
  {:else}
    <table class="jobs">
      <thead>
        <tr>
          <th>{t("bulk.jobs.col.id")}</th>
          <th>{t("bulk.jobs.col.kind")}</th>
          <th>{t("bulk.jobs.col.format")}</th>
          <th>{t("bulk.jobs.col.status")}</th>
          <th>{t("bulk.jobs.col.rows")}</th>
        </tr>
      </thead>
      <tbody>
        {#each jobs as job (job.id)}
          <tr>
            <td><code>{job.id}</code></td>
            <td>{kindLabel(job.kind)}</td>
            <td>{formatLabel(job.format)}</td>
            <td>{statusLabel(job.status)}</td>
            <td>{job.rows_processed} / {job.rows_total ?? "—"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .kv {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 1rem;
    row-gap: 0.25rem;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    font-size: 0.875rem;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.875rem;
  }
  .hint {
    color: var(--mxi-color-muted);
    font-size: 0.75rem;
  }
  .artifact-label {
    font-weight: 600;
    font-size: 0.875rem;
  }
  .artifact {
    display: block;
    overflow-wrap: anywhere;
  }
  .jobs {
    width: 100%;
    border-collapse: collapse;
  }
  .jobs th,
  .jobs td {
    text-align: start;
    padding: 0.375rem 0.5rem;
    border-bottom: 1px solid var(--mxi-color-border);
  }
  .jobs th {
    font-size: 0.8125rem;
  }
</style>
