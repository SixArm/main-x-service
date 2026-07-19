<!--
  Duplicate-review board (`/review`) — the batch-deduplication scan's
  review queue as SVAR Kanban columns (Pending / Confirmed / Rejected
  / AutoMerged). The scan runs only on the button (POST /deduplicate
  is a destructive-classed action, never a page-load side effect).
  Read-only: the services expose no review-decision endpoint yet —
  that endpoint is the seam that would make the columns drag targets.
-->
<script lang="ts">
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import { PlaceRepository } from "$lib/api/places";
    import type { ReviewQueueItem } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = PlaceRepository.withFetch();

    let items = $state<ReviewQueueItem[]>([]);
    let scanned = $state<number | null>(null);
    let running = $state(false);
    let error = $state<string | null>(null);

    async function runScan() {
        running = true;
        error = null;
        try {
            const report = await repo.deduplicate({});
            items = report.review_items;
            scanned = report.places_scanned;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            running = false;
        }
    }

    const columns = [
        { id: "Pending", label: "Pending", addCard: false },
        { id: "Confirmed", label: "Confirmed", addCard: false },
        { id: "Rejected", label: "Rejected", addCard: false },
        { id: "AutoMerged", label: "AutoMerged", addCard: false },
    ];

    const cards = $derived(
        items.map((item) => ({
            id: item.id,
            label: `${item.place_id_a.slice(0, 8)} ↔ ${item.place_id_b.slice(0, 8)}`,
            description: `${item.match_quality} · ${item.match_score.toFixed(2)}`,
            status: item.status,
        })),
    );
</script>

<svelte:head><title>{t("nav.review")} — Main X</title></svelte:head>

<h1>{t("nav.review")}</h1>

<p>
    <button onclick={() => void runScan()} disabled={running}>
        {t("review.run")}
    </button>
    {#if scanned !== null}
        <span class="muted">{scanned} · {items.length}</span>
    {/if}
</p>

{#if error}
    <p class="error" role="alert">{error}</p>
{:else if scanned !== null}
    <div class="board-wrap" data-testid="review-board">
        <Willow>
            <Kanban
                {cards}
                {columns}
                columnAccessor="status"
                card={{ ...getCardShape(), menu: false }}
                readonly
            />
        </Willow>
    </div>
{/if}

<style>
    .board-wrap {
        height: 560px;
        overflow-x: auto;
    }
</style>
