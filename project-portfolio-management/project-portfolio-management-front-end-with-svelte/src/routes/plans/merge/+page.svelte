<!--
  Merge route (`/plans/merge`) — fold a confirmed-duplicate plan into a
  surviving one.

  Purpose:
    Collects the survivor's pid, the duplicate's pid, and an optional
    reason; optionally previews both records side by side; then merges
    after an explicit confirmation (the duplicate is soft-deleted). On
    success it links to the surviving plan and refreshes the recent-merge
    history below.

  $state:
    - mainId / duplicateId / reason : bound form fields.
    - preview                       : the two loaded plans, for confirmation.
    - result                        : the MergeResponse after a merge.
    - recent                        : the merge-history rows (newest first).
    - error / merging               : request + validation status.

  Wire shape (service `src/controllers/plans.rs`): the response is
  `{main_pid, duplicate_pid, main}` — there is no `merge_record` wrapper,
  so the history row is read back from `/api/plans/merges/recent`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { PlanRepository } from "$lib/api/plans";
  import { ApiError } from "$lib/api/client";
  import { validateMerge } from "$lib/components/merge-validation";
  import type { MergeRecordRow, MergeResponse, Plan } from "$lib/api/types";
  import { t, translate } from "$lib/i18n.svelte";

  const repo = PlanRepository.withFetch();

  let mainId = $state("");
  let duplicateId = $state("");
  let reason = $state("");
  let preview = $state<{ main: Plan | null; duplicate: Plan | null }>({
    main: null,
    duplicate: null,
  });
  let result = $state<MergeResponse | null>(null);
  let recent = $state<MergeRecordRow[]>([]);
  let error = $state<string | null>(null);
  let merging = $state(false);

  // Fetch whichever pids are filled, in parallel, for a side-by-side look
  // before the operator commits to a destructive merge.
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

  async function loadRecent() {
    try {
      recent = await repo.recentMerges();
    } catch (err) {
      error = describe(err);
    }
  }

  async function doMerge() {
    // Guard first: both pids present and distinct. The check lives in a
    // pure helper so it is unit-testable without mounting this component.
    const guard = validateMerge(mainId, duplicateId);
    if (guard) {
      error = translate(guard);
      return;
    }
    // Destructive — the duplicate is soft-deleted. Confirm explicitly.
    const question = translate("merge.confirm")
      .replace("{dup}", duplicateId)
      .replace("{main}", mainId);
    if (!confirm(question)) return;
    merging = true;
    error = null;
    try {
      result = await repo.merge({
        main_pid: mainId,
        duplicate_pid: duplicateId,
        reason: reason.trim() || null,
      });
      // The history now has one more row; keep the table honest.
      await loadRecent();
    } catch (err) {
      error = describe(err);
    } finally {
      merging = false;
    }
  }

  // This client's ApiError carries `status` + `message` (no error code), so
  // the status is what makes a failure legible ("404: not found").
  function describe(err: unknown): string {
    if (err instanceof ApiError) return `${err.status}: ${err.message}`;
    return err instanceof Error ? err.message : String(err);
  }

  // One-line summary "Name (kind)" for the preview panel.
  function summary(plan: Plan | null): string {
    if (!plan) return "—";
    return `${plan.name}${plan.kind ? ` (${plan.kind})` : ""}`;
  }

  onMount(loadRecent);
</script>

<svelte:head><title>{t("merge.title")} — PPM</title></svelte:head>

<h1>{t("merge.title")}</h1>

<section class="surface stack">
  <div class="row">
    <label
      >{t("merge.mainId")} <small class="muted">{t("merge.mainIdHint")}</small>
      <input id="merge-main" type="text" bind:value={mainId} required />
    </label>
    <label
      >{t("merge.dupId")} <small class="muted">{t("merge.dupIdHint")}</small>
      <input id="merge-dup" type="text" bind:value={duplicateId} required />
    </label>
  </div>
  <label
    >{t("merge.reason")} <small class="muted">{t("merge.reasonHint")}</small>
    <input
      id="merge-reason"
      type="text"
      bind:value={reason}
      placeholder={t("merge.reasonPlaceholder")}
    />
  </label>
  <div class="row">
    <button type="button" class="button" onclick={loadPreview}>
      {t("merge.loadPreview")}
    </button>
    <button
      type="button"
      class="button primary"
      onclick={doMerge}
      disabled={merging}
    >
      {merging ? t("merge.merging") : t("merge.merge")}
    </button>
  </div>
  {#if error}<p class="banner error" role="alert">{error}</p>{/if}
</section>

{#if preview.main || preview.duplicate}
  <section class="surface stack" style="margin-top:1rem">
    <h2>{t("merge.preview")}</h2>
    <div><strong>{t("merge.main")}</strong> {summary(preview.main)}</div>
    <div>
      <strong>{t("merge.duplicate")}</strong>
      {summary(preview.duplicate)}
    </div>
  </section>
{/if}

{#if result}
  <section class="surface stack" style="margin-top:1rem">
    <h2>{t("merge.completed")}</h2>
    <div><strong>{t("merge.main")}</strong> <code>{result.main_pid}</code></div>
    <div>
      <strong>{t("merge.duplicate")}</strong>
      <code>{result.duplicate_pid}</code>
    </div>
    <!-- SPA navigation to the surviving plan's detail page. -->
    <a
      class="button primary"
      href={`/plans/${result.main_pid}`}
      onclick={() => result && goto(`/plans/${result.main_pid}`)}
    >
      {t("merge.viewMain")}
    </a>
  </section>
{/if}

<section style="margin-top:1.5rem">
  <h2>{t("merge.recent")}</h2>
  <table data-testid="recent-merges">
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
          <td><a href={`/plans/${row.main_pid}`}><code>{row.main_pid}</code></a></td>
          <td><code>{row.duplicate_pid}</code></td>
          <td>{row.reason ?? "—"}</td>
          <td>{row.actor ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">{t("merge.recentEmpty")}</td></tr>
      {/each}
    </tbody>
  </table>
</section>
