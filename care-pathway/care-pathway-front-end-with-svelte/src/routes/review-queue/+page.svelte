<!--
  Duplicate review queue (/review-queue, T-10) — the stored candidate
  pairs a keyless bulk-import row queues (`provenance = "import"`), with
  a per-row Confirm / Reject decision.

  Deliberately does **not** trigger a merge: "confirmed" only marks the
  pair ready-for-merge (matching the service's own scope); the operator
  still performs the actual merge from either pathway's own detail page
  (`/[pid]`'s existing "Merge into this record" action), which needs to
  pick which side survives — a decision this list has no basis to make
  for them.

  State:
    - items / loading / error — the fetched queue for the current filter.
    - statusFilter — re-queries the server on change (the endpoint
      filters server-side).
    - deciding — per-item-id submitting flag, so one row's action does
      not disable the whole table.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CarePathwayRepository } from "$lib/api/care-pathways";
  import { ApiError } from "$lib/api/client";
  import { t } from "$lib/i18n.svelte";
  import type { ReviewQueueItem, ReviewQueueStatus } from "$lib/api/types";

  const repo = CarePathwayRepository.withFetch();

  let items = $state<ReviewQueueItem[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let statusFilter = $state<ReviewQueueStatus | "">("pending");

  /** Per-item decision state: submitting flag + any inline error. */
  let deciding = $state<Record<string, boolean>>({});
  let decideErrors = $state<Record<string, string>>({});

  onMount(load);

  function describe(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  /** Translated label for a wire status token; unknown tokens pass through. */
  function statusLabel(status: string): string {
    switch (status) {
      case "pending":
        return t("reviewQueue.status.pending");
      case "confirmed":
        return t("reviewQueue.status.confirmed");
      case "rejected":
        return t("reviewQueue.status.rejected");
      case "automerged":
        return t("reviewQueue.status.automerged");
      default:
        return status;
    }
  }

  /** (Re)load the queue for the current status filter. */
  async function load() {
    loading = true;
    error = null;
    try {
      const response = await repo.listReviewQueue({
        status: statusFilter || undefined,
      });
      items = response.items;
    } catch (err) {
      error = describe(err);
    } finally {
      loading = false;
    }
  }

  /**
   * Decide one item, then remove it from the current view when it no
   * longer matches the active filter (e.g. deciding a row while
   * "Pending" is selected) — refreshing in place rather than an extra
   * round trip.
   */
  async function decide(
    item: ReviewQueueItem,
    status: "confirmed" | "rejected",
  ) {
    deciding = { ...deciding, [item.id]: true };
    decideErrors = { ...decideErrors, [item.id]: "" };
    try {
      const decided = await repo.decideReview(item.id, status);
      if (statusFilter && decided.status !== statusFilter) {
        items = items.filter((i) => i.id !== item.id);
      } else {
        items = items.map((i) => (i.id === item.id ? decided : i));
      }
    } catch (err) {
      // A 422 (already decided by someone else, first-writer-wins)
      // or a 404 (swept in the meantime) both read as a plain
      // inline message — a full-page error would be disproportionate
      // to a single stale row.
      const message =
        err instanceof ApiError && err.status === 422
          ? t("reviewQueue.error.alreadyDecided")
          : describe(err);
      decideErrors = { ...decideErrors, [item.id]: message };
    } finally {
      deciding = { ...deciding, [item.id]: false };
    }
  }
</script>

<svelte:head><title>{t("reviewQueue.head.title")}</title></svelte:head>

<header>
  <h1>{t("reviewQueue.title")}</h1>
  <p class="muted">{t("reviewQueue.intro")}</p>
</header>

<section class="surface stack">
  <div class="row">
    <label class="inline">
      {t("reviewQueue.filterStatus")}
      <select bind:value={statusFilter} onchange={load}>
        <option value="">{t("bulk.jobs.all")}</option>
        <option value="pending">{t("reviewQueue.status.pending")}</option>
        <option value="confirmed">{t("reviewQueue.status.confirmed")}</option>
        <option value="rejected">{t("reviewQueue.status.rejected")}</option>
        <option value="automerged">{t("reviewQueue.status.automerged")}</option>
      </select>
    </label>
    <button type="button" class="button" onclick={load}
      >{t("bulk.jobs.refresh")}</button
    >
  </div>
  {#if error}<div class="banner error" role="alert">{error}</div>{/if}
  {#if loading}
    <p class="muted">{t("bulk.jobs.loading")}</p>
  {:else if items.length === 0}
    <p class="muted">{t("reviewQueue.empty")}</p>
  {:else}
    <table class="queue">
      <thead>
        <tr>
          <th>{t("reviewQueue.col.pair")}</th>
          <th>{t("reviewQueue.col.score")}</th>
          <th>{t("reviewQueue.col.detection")}</th>
          <th>{t("reviewQueue.col.provenance")}</th>
          <th>{t("reviewQueue.col.status")}</th>
          <th>{t("reviewQueue.col.queuedAt")}</th>
          <th>{t("reviewQueue.col.actions")}</th>
        </tr>
      </thead>
      <tbody>
        {#each items as item (item.id)}
          <tr>
            <td>
              <a href={`/${item.pathway_id_a}`}
                ><code>{item.pathway_id_a}</code></a
              >
              <span aria-hidden="true">↔</span>
              <a href={`/${item.pathway_id_b}`}
                ><code>{item.pathway_id_b}</code></a
              >
            </td>
            <td>
              {Math.round(item.match_score * 100)}% ({item.match_quality})
            </td>
            <td>{item.detection_method}</td>
            <td>{item.provenance}</td>
            <td>{statusLabel(item.status)}</td>
            <td>{new Date(item.created_at).toLocaleString()}</td>
            <td>
              {#if item.status === "pending"}
                <div class="actions">
                  <button
                    type="button"
                    class="button primary"
                    disabled={deciding[item.id]}
                    onclick={() => decide(item, "confirmed")}
                  >
                    {t("reviewQueue.confirm")}
                  </button>
                  <button
                    type="button"
                    class="button"
                    disabled={deciding[item.id]}
                    onclick={() => decide(item, "rejected")}
                  >
                    {t("reviewQueue.reject")}
                  </button>
                </div>
                {#if decideErrors[item.id]}
                  <p class="banner error small" role="alert">
                    {decideErrors[item.id]}
                  </p>
                {/if}
              {:else}
                <span class="muted small">
                  {item.reviewed_by ?? t("reviewQueue.unknownReviewer")}
                  {#if item.reviewed_at}
                    · {new Date(item.reviewed_at).toLocaleString()}
                  {/if}
                </span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .inline {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.875rem;
  }
  .queue {
    width: 100%;
    border-collapse: collapse;
  }
  .queue th,
  .queue td {
    text-align: start;
    vertical-align: top;
    padding: 0.375rem 0.5rem;
    border-bottom: 1px solid var(--mxi-color-border);
  }
  .queue th {
    font-size: 0.8125rem;
  }
  .queue code {
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    gap: 0.375rem;
  }
  .small {
    font-size: 0.75rem;
  }
</style>
