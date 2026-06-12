<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { Organization, ScoredRef } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    const pid = page.params.pid ?? "";

    let org = $state<Organization | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let duplicates = $state<ScoredRef[] | null>(null);
    let checking = $state(false);

    onMount(async () => {
        try {
            org = await repo.get(pid);
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
        if (!org) return;
        checking = true;
        try {
            const hits = await repo.checkDuplicates(org);
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Check failed";
        } finally {
            checking = false;
        }
    }
</script>

<svelte:head><title>{org?.name ?? "Organization"} — Main X</title></svelte:head>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if org}
    <h1>{org.name}</h1>
    <div class="surface stack">
        {#if org.legal_name}<div><strong>Legal name:</strong> {org.legal_name}</div>{/if}
        {#if org.url}<div><strong>URL:</strong> <a href={org.url}>{org.url}</a></div>{/if}
        {#if org.jurisdiction}<div><strong>Jurisdiction:</strong> {org.jurisdiction}</div>{/if}
        {#if org.founding_date}<div><strong>Founded:</strong> {org.founding_date}</div>{/if}
        {#if org.identifiers && org.identifiers.length > 0}
            <div>
                <strong>Identifiers:</strong>
                <ul>
                    {#each org.identifiers as id, i (i)}
                        <li>{typeof id.scheme === "string" ? id.scheme : `Custom(${id.scheme.Custom})`}: <code>{id.value}</code></li>
                    {/each}
                </ul>
            </div>
        {/if}
        {#if org.keywords && org.keywords.length > 0}
            <div><strong>Keywords:</strong> {org.keywords.join(", ")}</div>
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
