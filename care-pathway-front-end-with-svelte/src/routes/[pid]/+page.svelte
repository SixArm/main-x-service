<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway, ScoredRef } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    const pid = page.params.pid ?? "";

    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let duplicates = $state<ScoredRef[] | null>(null);
    let checking = $state(false);

    onMount(async () => {
        try {
            pathway = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Not found";
        } finally {
            loading = false;
        }
    });

    async function handleDelete() {
        await repo.remove(pid);
        await goto("/");
    }

    async function handleCheckDuplicates() {
        if (!pathway) return;
        checking = true;
        try {
            const hits = await repo.checkDuplicates(pathway);
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Check failed";
        } finally {
            checking = false;
        }
    }
</script>

<svelte:head><title>{pathway?.name ?? "Care pathway"} — Main X</title></svelte:head>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if pathway}
    <h1>{pathway.name}</h1>
    <div class="surface stack">
        {#if pathway.care_setting}
            <div><strong>Care setting:</strong> {typeof pathway.care_setting === "string" ? pathway.care_setting : pathway.care_setting.Custom}</div>
        {/if}
        {#if pathway.provider_name}<div><strong>Provider:</strong> {pathway.provider_name}</div>{/if}
        {#if pathway.pathway_code}<div><strong>Pathway code:</strong> <code>{pathway.pathway_code}</code></div>{/if}
        {#if pathway.condition_codes && pathway.condition_codes.length > 0}
            <div>
                <strong>Condition codes:</strong>
                {pathway.condition_codes
                    .map((c) => `${typeof c.system === "string" ? c.system : c.system.Custom}:${c.code}`)
                    .join(", ")}
            </div>
        {/if}
        {#if pathway.identifiers && pathway.identifiers.length > 0}
            <div>
                <strong>Identifiers:</strong>
                <ul>
                    {#each pathway.identifiers as id, i (i)}
                        <li>{typeof id.scheme === "string" ? id.scheme : `Custom(${id.scheme.Custom})`}: <code>{id.value}</code></li>
                    {/each}
                </ul>
            </div>
        {/if}
        {#if pathway.interventions && pathway.interventions.length > 0}
            <div><strong>Interventions:</strong> {pathway.interventions.join(", ")}</div>
        {/if}
        {#if pathway.keywords && pathway.keywords.length > 0}
            <div><strong>Keywords:</strong> {pathway.keywords.join(", ")}</div>
        {/if}
        <div><strong>ID:</strong> <code>{pid}</code></div>
    </div>

    <div class="row" style="margin-top:1rem">
        <a class="button" href={`/${pid}/edit`}>Edit</a>
        <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
            {checking ? "Checking…" : "Check duplicates"}
        </button>
        <button onclick={handleDelete}>Delete</button>
    </div>

    {#if duplicates}
        <h2>Potential duplicates</h2>
        {#if duplicates.length === 0}
            <p>None above the match threshold.</p>
        {:else}
            <ul class="stack">
                {#each duplicates as dup (dup.pid)}
                    <li class="surface row">
                        <a href={`/${dup.pid}`}>{dup.name}</a>
                        <span>{dup.score.toFixed(3)} · {dup.confidence}</span>
                    </li>
                {/each}
            </ul>
        {/if}
    {/if}
{/if}
