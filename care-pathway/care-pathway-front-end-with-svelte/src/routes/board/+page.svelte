<!--
  Instance board (`/board`) — one pathway's enrolled instances as SVAR
  Kanban columns (Active / On hold / Completed / Discontinued). Instances
  across pathways are not directly listable, so the board is built from a
  single selected pathway (a `<select>` seeded from the registry list) via
  `GET /api/care-pathways/{pathway}/instances`. Dragging a card issues
  `POST /api/instances/{pid}/status`; the service's lifecycle machine
  refuses illegal moves (422), and a reload restores the stored truth —
  mirroring the organization review board's drag-to-decide init pattern.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
    import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import {
        INSTANCE_STATUSES,
        type InstanceStatus,
        type PathwayInstance,
        type PathwayRef,
    } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();

    let refs = $state<PathwayRef[]>([]);
    let selected = $state("");
    let instances = $state<PathwayInstance[] | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let actionError = $state<string | null>(null);
    // Derived caseload context (across all pathways), loaded once.
    let caseload = $state<{ note?: string; total?: number } | null>(null);

    async function loadInstances(pathwayPid: string) {
        if (!pathwayPid) {
            instances = [];
            return;
        }
        try {
            instances = await repo.listInstances(pathwayPid);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    onMount(async () => {
        try {
            // Caseload is loaded for operational context (not the board data).
            [refs, caseload] = await Promise.all([
                repo.list(),
                repo.caseload().then((c) => c as { note?: string; total?: number }),
            ]);
            selected = refs[0]?.pid ?? "";
            await loadInstances(selected);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Column ids are the wire tokens (the service serializes the status
    // lowercase / snake_case); labels stay human-cased.
    const columns = [
        { id: "active", label: "Active", addCard: false },
        { id: "on_hold", label: "On hold", addCard: false },
        { id: "completed", label: "Completed", addCard: false },
        { id: "discontinued", label: "Discontinued", addCard: false },
    ];

    const cards = $derived(
        (instances ?? []).map((instance) => ({
            id: instance.pid,
            label: instance.subject_ref,
            description: `${instance.urgency} · enrolled ${instance.enrolled_on}`,
            status: instance.status,
        })),
    );

    // Drag = a lifecycle move through the API. The status machine refuses
    // illegal transitions (422); the reload restores the stored truth.
    function init(api: KanbanInstanceApi) {
        api.on("move-card", (raw) => {
            const ev = raw as { id: string | number; column?: string | number };
            actionError = null;
            const target = String(ev.column ?? "") as InstanceStatus;
            const source = (instances ?? []).find((i) => i.pid === String(ev.id));
            if (
                source &&
                target !== source.status &&
                INSTANCE_STATUSES.includes(target)
            ) {
                void repo
                    .setInstanceStatus(String(ev.id), target)
                    .catch((cause) => {
                        actionError =
                            cause instanceof Error ? cause.message : String(cause);
                    })
                    .finally(() => loadInstances(selected));
            } else {
                void loadInstances(selected);
            }
        });
    }
</script>

<svelte:head><title>{t("nav.board")} — Main X</title></svelte:head>

<h1>{t("nav.board")}</h1>

<p>
    <label class="locale">
        <span>{t("form.name")}</span>
        <select
            bind:value={selected}
            onchange={() => void loadInstances(selected)}
        >
            {#each refs as ref (ref.pid)}
                <option value={ref.pid}>{ref.name}</option>
            {/each}
        </select>
    </label>
    {#if caseload?.total !== undefined}
        <span class="muted small" data-testid="board-context">
            Caseload: {caseload.total}
        </span>
    {/if}
</p>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else}
    {#if actionError}
        <p class="banner error" role="alert" data-testid="action-error">
            {actionError}
        </p>
    {/if}
    <div class="board-wrap" data-testid="instance-board">
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
