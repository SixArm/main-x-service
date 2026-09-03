<!--
  Bulk import / export (/persons/bulk) — upload a file of records, or
  extract a filtered set, and watch the resulting background job.

  Both submits return `202 {job_id}`; the page then polls the job's status
  endpoint until it reaches a terminal state, showing the row-count
  breakdown as it fills in.

  Two deliberate gaps, both service-side rather than UI omissions:
    - `download_url` / `errors_url` are **opaque artifact-store references**
      (`file://…` / `s3://…`), and the service exposes no endpoint that
      serves their bytes — so they are rendered as plain text, not links.
      Tracked in tasks.md FE-3.
    - `include_soft_deleted` is not offered: the endpoint accepts it but the
      worker rejects it, so the job would be accepted and then fail.

  State:
    - importPanel / exportPanel — the submitted job being polled.
    - jobs / kindFilter / statusFilter — the recent-jobs table and its
      client-side filters (the endpoint has no server-side filtering).
-->
<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { ApiError } from "$lib/api/client.js";
    import { t } from "$lib/i18n.svelte.js";
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
    } from "$lib/bulk.js";
    import type { BulkJobView } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();

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

    // The endpoint takes only `limit` — no kind/status filtering server-side
    // — so the filters apply to the already-fetched array.
    const filteredJobs = $derived(
        jobs.filter(
            (j) =>
                (kindFilter === "" || j.kind === kindFilter) &&
                (statusFilter === "" || j.status === statusFilter),
        ),
    );

    // Set on unmount so an in-flight poll loop stops instead of updating a
    // destroyed component (and retrying forever in the background).
    let destroyed = false;
    onDestroy(() => (destroyed = true));

    onMount(loadJobs);

    /** Human message for a thrown error, preferring the API's own code. */
    function describe(err: unknown): string {
        if (err instanceof ApiError) return `${err.code}: ${err.message}`;
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
            const accepted = await repo.importPersons(importFile, {
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
            const accepted = await repo.exportPersons(
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

    /** (Re)load the recent-jobs table. */
    async function loadJobs() {
        jobsLoading = true;
        jobsError = null;
        try {
            jobs = await repo.listBulkJobs();
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
     artifact references (as text, per the FE-3 scope note). -->
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
                {panel.job
                    ? statusLabel(panel.job.status)
                    : t("bulk.job.polling")}
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
    <LabeledField
        label={t("bulk.import.file")}
        for="bulk-import-file"
        required
        hint={t("bulk.import.fileHint")}
    >
        <input
            id="bulk-import-file"
            type="file"
            accept=".jsonl,.csv,application/jsonl,application/json,text/csv"
            onchange={onFileChange}
        />
    </LabeledField>
    <FieldRow>
        <LabeledField
            label={t("bulk.import.format")}
            for="bulk-import-format"
            hint={t("bulk.import.formatHint")}
        >
            <select id="bulk-import-format" bind:value={importFormat}>
                {#each BULK_IMPORT_FORMATS as fmt (fmt)}
                    <option value={fmt}
                        >{t(
                            fmt === "jsonl"
                                ? "bulk.format.jsonl"
                                : "bulk.format.csv",
                        )}</option
                    >
                {/each}
            </select>
        </LabeledField>
    </FieldRow>
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
            {importSubmitting
                ? t("bulk.import.submitting")
                : t("bulk.import.submit")}
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
    <FieldRow>
        <LabeledField
            label={t("bulk.export.format")}
            for="bulk-export-format"
            hint={t("bulk.export.formatHint")}
        >
            <select id="bulk-export-format" bind:value={exportFormat}>
                {#each BULK_FORMATS as fmt (fmt)}
                    <option value={fmt}>
                        {fmt === "jsonl"
                            ? t("bulk.format.jsonl")
                            : fmt === "csv"
                              ? t("bulk.format.csv")
                              : t("bulk.format.parquet")}
                    </option>
                {/each}
            </select>
        </LabeledField>
        <LabeledField
            label={t("bulk.export.masking")}
            for="bulk-export-masking"
            hint={t("bulk.export.maskingHint")}
        >
            <select id="bulk-export-masking" bind:value={maskingProfile}>
                {#each MASKING_PROFILES as profile (profile)}
                    <option value={profile}>
                        {profile === "masked"
                            ? t("bulk.masking.masked")
                            : t("bulk.masking.full")}
                    </option>
                {/each}
            </select>
        </LabeledField>
    </FieldRow>
    <!-- Parquet is behind a default-off Cargo feature the front-end cannot
         detect, so a `failed` job may be a build choice, not a UI bug. -->
    <small class="hint">{t("bulk.export.parquetNote")}</small>
    <FieldRow>
        <LabeledField
            label={t("bulk.export.query")}
            for="bulk-export-query"
            hint={t("bulk.export.queryHint")}
        >
            <input id="bulk-export-query" bind:value={exportQuery} />
        </LabeledField>
        <LabeledField
            label={t("bulk.export.limit")}
            for="bulk-export-limit"
            hint={t("bulk.export.limitHint")}
        >
            <input
                id="bulk-export-limit"
                type="number"
                min="1"
                bind:value={exportLimit}
            />
        </LabeledField>
    </FieldRow>
    <div class="row">
        <button
            type="button"
            class="button primary"
            onclick={startExport}
            disabled={exportSubmitting}
        >
            {exportSubmitting
                ? t("bulk.export.submitting")
                : t("bulk.export.submit")}
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
            <select bind:value={kindFilter}>
                <option value="">{t("bulk.jobs.all")}</option>
                <option value="import">{t("bulk.kind.import")}</option>
                <option value="export">{t("bulk.kind.export")}</option>
            </select>
        </label>
        <label class="inline">
            {t("bulk.jobs.filterStatus")}
            <select bind:value={statusFilter}>
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
    {:else if filteredJobs.length === 0}
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
                {#each filteredJobs as job (job.id)}
                    <tr>
                        <td><code>{job.id}</code></td>
                        <td>{kindLabel(job.kind)}</td>
                        <td>{job.format}</td>
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
