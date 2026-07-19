<!--
  Intervention sequence (`/sequence`) — the selected pathway's
  interventions as ordered bars in the SVAR Gantt. **This is a
  sequence view, not a schedule**: the pathway model carries only the
  intervention order (no durations or dates), so each step renders as
  one nominal unit on an ordinal axis, clearly labelled. Per-step
  durations on the service model are the seam that would make this a
  real timeline.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { Gantt, Willow } from "@svar-ui/svelte-gantt";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway, PathwayRef } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();

    let refs = $state<PathwayRef[]>([]);
    let selected = $state("");
    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    async function loadPathway(pid: string) {
        if (!pid) return;
        loading = true;
        error = null;
        try {
            pathway = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            pathway = null;
        } finally {
            loading = false;
        }
    }

    onMount(async () => {
        try {
            refs = await repo.list();
            selected = refs[0]?.pid ?? "";
            if (selected) await loadPathway(selected);
            else loading = false;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            loading = false;
        }
    });

    // Ordinal axis: step i occupies [epoch + i, epoch + i + 1) days.
    // The epoch is arbitrary and hidden — the scale labels steps, not
    // dates, so no false schedule is implied.
    const EPOCH = new Date(2000, 0, 3); // a Monday; never rendered
    const tasks = $derived(
        (pathway?.interventions ?? []).map((intervention, index) => ({
            id: index + 1,
            text: intervention,
            start: new Date(EPOCH.getTime() + index * 86_400_000),
            end: new Date(EPOCH.getTime() + (index + 1) * 86_400_000),
            type: "task",
        })),
    );

    // Each step depends on the previous — the sequence itself.
    const links = $derived(
        (pathway?.interventions ?? []).slice(1).map((_, index) => ({
            id: index + 1,
            source: index + 1,
            target: index + 2,
            type: "e2s",
        })),
    );

    // Label the axis with step ordinals, not dates.
    const scales = [{ unit: "day" as const, step: 1, format: "" }];
</script>

<svelte:head><title>{t("nav.sequence")} — Main X</title></svelte:head>

<h1>{t("nav.sequence")}</h1>

<p>
    <label class="locale">
        <span>{t("form.name")}</span>
        <select
            bind:value={selected}
            onchange={() => void loadPathway(selected)}
        >
            {#each refs as ref (ref.pid)}
                <option value={ref.pid}>{ref.name}</option>
            {/each}
        </select>
    </label>
</p>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if tasks.length === 0}
    <p>{t("list.empty")}</p>
{:else}
    <div class="gantt-wrap" data-testid="pathway-sequence">
        <Willow>
            <Gantt {tasks} {links} {scales} readonly cellWidth={56} />
        </Willow>
    </div>
{/if}

<style>
    .gantt-wrap {
        height: 480px;
    }
</style>
