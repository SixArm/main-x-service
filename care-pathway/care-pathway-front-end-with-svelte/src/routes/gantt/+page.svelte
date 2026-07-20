<!--
  Instance timeline (`/gantt`) — one pathway's enrolled instances as bars
  in the SVAR Gantt. A `<select>` (seeded from the registry list) chooses
  the pathway; each instance renders as a bar from `enrolled_on` to
  `next_review_on ?? closed_on ?? today`, labelled by `subject_ref`.
  Instances with no enrolment date are listed below the chart rather than
  invented onto the timeline (honesty). Read-only — the Gantt is a derived
  view, not a second editor.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { Gantt, Willow } from "@svar-ui/svelte-gantt";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { PathwayInstance, PathwayRef } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();

    let refs = $state<PathwayRef[]>([]);
    let selected = $state("");
    let instances = $state<PathwayInstance[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    async function loadInstances(pathwayPid: string) {
        if (!pathwayPid) {
            instances = [];
            return;
        }
        loading = true;
        error = null;
        try {
            instances = await repo.listInstances(pathwayPid);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            instances = [];
        } finally {
            loading = false;
        }
    }

    onMount(async () => {
        try {
            refs = await repo.list();
            selected = refs[0]?.pid ?? "";
            if (selected) await loadInstances(selected);
            else loading = false;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            loading = false;
        }
    });

    const today = new Date();

    // Datable instances (an enrolment date exists) become task bars from
    // enrolled_on → next_review_on ?? closed_on ?? today.
    const tasks = $derived(
        instances
            .filter((i) => i.enrolled_on)
            .map((i) => ({
                id: i.pid,
                text: i.subject_ref,
                start: new Date(i.enrolled_on),
                end: new Date(i.next_review_on ?? i.closed_on ?? today),
                type: "task",
            })),
    );

    // Instances with no enrolment date are surfaced, not invented.
    const undated = $derived(instances.filter((i) => !i.enrolled_on));
</script>

<svelte:head><title>{t("nav.gantt")} — Main X</title></svelte:head>

<h1>{t("nav.gantt")}</h1>

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
</p>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else if tasks.length === 0}
    <p class="surface">No dated instances on this pathway.</p>
{:else}
    <div class="gantt-wrap" data-testid="instance-gantt">
        <Willow>
            <Gantt {tasks} readonly />
        </Willow>
    </div>
{/if}

{#if undated.length > 0}
    <p class="muted small" data-testid="gantt-undated">
        Undated instances:
        {#each undated as instance (instance.pid)}
            <span class="chip">{instance.subject_ref}</span>
        {/each}
    </p>
{/if}

<style>
    .gantt-wrap {
        height: 480px;
    }
    .chip {
        display: inline-block;
        margin: 0 0.25rem;
        padding: 0.1rem 0.4rem;
        border: 1px solid var(--mxi-color-border, #ddd);
        border-radius: var(--mxi-radius, 6px);
    }
</style>
