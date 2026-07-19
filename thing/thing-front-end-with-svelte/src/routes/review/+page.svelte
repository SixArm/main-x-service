<!--
  Duplicate-review board (`/review`) — the STORED review queue as SVAR
  Kanban columns (Pending / Confirmed / Rejected / AutoMerged). The
  queue loads on mount (a safe GET); the batch scan runs only on the
  button (POST /deduplicate is a destructive-classed action, never a
  page-load side effect) and upserts into the same stored queue with
  stable item ids. Dragging a pending card into Confirmed / Rejected
  records the decision through the decision endpoint; the service's
  first-writer-wins guard refuses non-pending transitions and the
  reload puts the card back where the stored truth says it belongs.
-->
<script lang="ts">
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
    import { ThingRepository } from "$lib/api/things";
    import type { ReviewQueueItem } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();

    let items = $state<ReviewQueueItem[] | null>(null);
    let scanned = $state<number | null>(null);
    let running = $state(false);
    let error = $state<string | null>(null);
    let actionError = $state<string | null>(null);

    async function load() {
        try {
            items = await repo.listReviewQueue();
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    $effect(() => {
        void load();
    });

    async function runScan() {
        running = true;
        error = null;
        try {
            const report = await repo.deduplicate({});
            items = report.review_items;
            scanned = report.things_scanned;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            running = false;
        }
    }

    // Column ids are the wire tokens (the services serialize the
    // review status lowercase); labels stay human-cased.
    const columns = [
        { id: "pending", label: "Pending", addCard: false },
        { id: "confirmed", label: "Confirmed", addCard: false },
        { id: "rejected", label: "Rejected", addCard: false },
        { id: "automerged", label: "AutoMerged", addCard: false },
    ];

    const cards = $derived(
        (items ?? []).map((item) => ({
            id: item.id,
            label: `${item.thing_id_a.slice(0, 8)} ↔ ${item.thing_id_b.slice(0, 8)}`,
            description: `${item.match_quality} · ${item.match_score.toFixed(2)} · ${item.detection_method}`,
            status: item.status,
        })),
    );

    // Drag = a review decision through the API. Only pending →
    // confirmed / rejected is a legal move; anything else is refused
    // client-side and the reload restores the stored truth.
    function init(api: KanbanInstanceApi) {
        api.on("move-card", (raw) => {
            const ev = raw as { id: string | number; column?: string | number };
            actionError = null;
            const target = String(ev.column ?? "");
            const source = (items ?? []).find((i) => i.id === String(ev.id));
            if (
                source &&
                source.status === "pending" &&
                (target === "confirmed" || target === "rejected")
            ) {
                void repo
                    .decideReview(String(ev.id), target)
                    .catch((cause) => {
                        actionError =
                            cause instanceof Error ? cause.message : String(cause);
                    })
                    .finally(load);
            } else {
                void load();
            }
        });
    }
</script>

<svelte:head><title>{t("nav.review")} — Main X</title></svelte:head>

<h1>{t("nav.review")}</h1>

<p>
    <button onclick={() => void runScan()} disabled={running}>
        {t("review.run")}
    </button>
    {#if scanned !== null}
        <span class="muted">{scanned} · {(items ?? []).length}</span>
    {/if}
</p>

{#if error}
    <p class="error" role="alert">{error}</p>
{:else if items !== null}
    {#if actionError}
        <p class="error" role="alert" data-testid="action-error">
            {actionError}
        </p>
    {/if}
    <div class="board-wrap" data-testid="review-board">
        <Willow>
            <Kanban
                {cards}
                {columns}
                columnAccessor="status"
                card={{ ...getCardShape(), menu: false }}
                {init}
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
