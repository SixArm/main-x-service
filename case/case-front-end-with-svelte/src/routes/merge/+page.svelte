<!--
  Merge route (`/merge`) — fold a confirmed duplicate case into a survivor.

  Purpose: collect a surviving main pid and a duplicate pid (plus an
  optional reason), optionally preview both records side by side, then
  merge after an explicit confirmation. Below the form, the recent
  merge-history rows the service records.

  State:
    - mainId / duplicateId / reason : bound form fields.
    - preview      : the two loaded cases, for a side-by-side check.
    - result       : the MergeResponse after a successful merge.
    - recent       : rows from `merges/recent` (loaded on mount, refreshed
                     after a successful merge).
    - error / loading / recentError : request + validation status.

  Wire shape (pinned against `case-service-with-loco`): the response is
  `{main_pid, duplicate_pid, main}` — there is NO `merge_record` wrapper,
  so this page shows the survivor's pid and links to it, and reads the
  merge row's timestamp from the recent-merges table instead.

  The merge is destructive (the duplicate is soft-deleted), so it is
  guarded by `validateMerge` and a native `confirm()`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { CaseRepository } from "$lib/api/cases";
  import { ApiError } from "$lib/api/client";
  import { validateMerge } from "$lib/components/merge-validation";
  import type { Case, MergeRecordRow, MergeResponse } from "$lib/api/types";
  import { t, translate } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();

  let mainId = $state("");
  let duplicateId = $state("");
  let reason = $state("");
  let preview = $state<{ main: Case | null; duplicate: Case | null }>({
    main: null,
    duplicate: null,
  });
  let result = $state<MergeResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let recent = $state<MergeRecordRow[]>([]);
  let recentError = $state<string | null>(null);
  let recentLoading = $state(true);

  // Show the service's HTTP status alongside its message for API errors;
  // this client's ApiError carries `status` (there is no `code` field).
  function describe(err: unknown): string {
    if (err instanceof ApiError) return `${err.status}: ${err.message}`;
    return err instanceof Error ? err.message : String(err);
  }

  // Recent merge history — loaded once on mount, refreshed after a merge.
  async function loadRecent() {
    recentLoading = true;
    recentError = null;
    try {
      recent = await repo.recentMerges();
    } catch (err) {
      recentError = `${t("merge.recentFailed")}: ${describe(err)}`;
    } finally {
      recentLoading = false;
    }
  }

  onMount(loadRecent);

  // Fetch whichever ids are filled, in parallel, for the preview panel.
  async function loadPreview() {
    preview = { main: null, duplicate: null };
    error = null;
    try {
      const [main, duplicate] = await Promise.all([
        mainId ? repo.get(mainId) : Promise.resolve(null),
        duplicateId ? repo.get(duplicateId) : Promise.resolve(null),
      ]);
      preview = { main, duplicate };
    } catch (err) {
      error = describe(err);
    }
  }

  async function doMerge() {
    // Guard: both ids required and they must differ (no self-merge — the
    // service answers 422). Pure helper so it is unit-testable.
    const guardKey = validateMerge(mainId, duplicateId);
    if (guardKey) {
      error = t(guardKey);
      return;
    }
    // Destructive (soft-deletes the duplicate) — confirm before proceeding.
    const question = translate("merge.confirm")
      .replace("{dup}", duplicateId.slice(0, 8))
      .replace("{main}", mainId.slice(0, 8));
    if (!confirm(question)) return;
    loading = true;
    error = null;
    try {
      result = await repo.merge({
        main_pid: mainId,
        duplicate_pid: duplicateId,
        reason: reason.trim() || null,
      });
      // The merge just wrote a history row; reflect it immediately.
      await loadRecent();
    } catch (err) {
      error = describe(err);
    } finally {
      loading = false;
    }
  }

  // One-line preview summary "Title (case number)" for the preview panel.
  function summary(record: Case | null): string {
    if (!record) return "—";
    return `${record.title}${record.case_number ? ` (${record.case_number})` : ""}`;
  }
</script>

<svelte:head><title>Merge cases — Main X</title></svelte:head>

<h1>{t("merge.title")}</h1>

<section class="surface stack">
  <div class="row">
    <label>
      {t("merge.mainId")}
      <input type="text" bind:value={mainId} data-testid="merge-main" />
      <small class="muted">{t("merge.mainIdHint")}</small>
    </label>
    <label>
      {t("merge.dupId")}
      <input type="text" bind:value={duplicateId} data-testid="merge-duplicate" />
      <small class="muted">{t("merge.dupIdHint")}</small>
    </label>
  </div>
  <label>
    {t("merge.reason")}
    <input
      type="text"
      bind:value={reason}
      placeholder={t("merge.reasonPlaceholder")}
      data-testid="merge-reason"
    />
    <small class="muted">{t("merge.reasonHint")}</small>
  </label>
  <div class="row">
    <button type="button" class="button" onclick={loadPreview}>
      {t("merge.loadPreview")}
    </button>
    <button
      type="button"
      class="button primary"
      onclick={doMerge}
      disabled={loading}
      data-testid="merge-submit"
    >
      {loading ? t("merge.merging") : t("merge.merge")}
    </button>
  </div>
  {#if error}
    <p class="banner error" role="alert" data-testid="merge-error">{error}</p>
  {/if}
</section>

{#if preview.main || preview.duplicate}
  <section class="surface stack" style="margin-top:1rem">
    <h2>{t("merge.preview")}</h2>
    <div><strong>{t("merge.main")}:</strong> {summary(preview.main)}</div>
    <div><strong>{t("merge.duplicate")}:</strong> {summary(preview.duplicate)}</div>
  </section>
{/if}

{#if result}
  <section class="surface stack" style="margin-top:1rem" data-testid="merge-result">
    <h2>{t("merge.completed")}</h2>
    <div><strong>{t("merge.main")}:</strong> <code>{result.main_pid}</code></div>
    <div>
      <strong>{t("merge.duplicate")}:</strong> <code>{result.duplicate_pid}</code>
    </div>
    <!-- SPA navigation to the surviving case's detail route (`/[pid]`). -->
    <a
      class="button primary"
      href={`/${result.main_pid}`}
      onclick={() => result && goto(`/${result.main_pid}`)}
    >
      {t("merge.viewMain")}
    </a>
  </section>
{/if}

<section class="stack" style="margin-top:1.5rem">
  <h2>{t("merge.recent")}</h2>
  {#if recentLoading}
    <p>{t("list.loading")}</p>
  {:else if recentError}
    <p class="banner error" role="alert" data-testid="merge-recent-error">
      {recentError}
    </p>
  {:else if recent.length === 0}
    <p class="surface">{t("merge.recentEmpty")}</p>
  {:else}
    <table class="surface" data-testid="merge-recent">
      <thead>
        <tr>
          <th>{t("merge.mergedAt")}</th>
          <th>{t("merge.main")}</th>
          <th>{t("merge.duplicate")}</th>
          <th>{t("merge.reason")}</th>
          <th>{t("merge.actor")}</th>
        </tr>
      </thead>
      <tbody>
        {#each recent as row (row.id)}
          <tr>
            <td>{new Date(row.created_at).toLocaleString()}</td>
            <td><a href={`/${row.main_pid}`}><code>{row.main_pid}</code></a></td>
            <td><code>{row.duplicate_pid}</code></td>
            <td>{row.reason ?? "—"}</td>
            <td>{row.actor ?? "—"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
  th,
  td {
    text-align: start;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--mxi-color-border);
  }
</style>
