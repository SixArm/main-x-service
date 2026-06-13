<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { AuditEntry, CarePathway, ScoredRef } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    const pid = page.params.pid ?? "";

    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let duplicates = $state<ScoredRef[] | null>(null);
    let checking = $state(false);
    let merging = $state<string | null>(null);
    let confirming = $state<string | null>(null);
    let mergeMessage = $state<string | null>(null);

    // Audit trail: lazy-loaded behind a toggle so the detail page stays
    // lean on first paint.
    let showAudit = $state(false);
    let audit = $state<AuditEntry[] | null>(null);
    let auditLoading = $state(false);
    let auditError = $state<string | null>(null);

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
        mergeMessage = null;
        confirming = null;
        try {
            const hits = await repo.checkDuplicates(pathway);
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Check failed";
        } finally {
            checking = false;
        }
    }

    /// This detail record is the survivor (main); the row's pid is the
    /// duplicate to fold in. Two-step: arm `confirming`, then merge.
    async function handleMerge(duplicatePid: string) {
        // Guard: equal pids would 422; should never happen here.
        if (duplicatePid === pid) {
            error = "Cannot merge a record into itself.";
            confirming = null;
            return;
        }
        merging = duplicatePid;
        error = null;
        mergeMessage = null;
        try {
            const result = await repo.merge(pid, duplicatePid);
            // The survivor's data may have changed: use the returned
            // record, then re-fetch the duplicates list.
            pathway = result.main;
            mergeMessage = `Merged ${duplicatePid} into this record.`;
            confirming = null;
            const hits = await repo.checkDuplicates(result.main);
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Merge failed";
        } finally {
            merging = null;
        }
    }

    /// Toggle the audit panel; lazy-load the trail on first open.
    async function toggleAudit() {
        showAudit = !showAudit;
        if (!showAudit || audit !== null || auditLoading) return;
        auditLoading = true;
        auditError = null;
        try {
            const rows = await repo.audit(pid);
            // Defensive newest-first sort by created_at (the service
            // already orders this way; tolerate missing timestamps).
            audit = [...rows].sort((a, b) =>
                (b.created_at ?? "").localeCompare(a.created_at ?? ""),
            );
        } catch (err) {
            auditError = err instanceof Error ? err.message : "Audit load failed";
        } finally {
            auditLoading = false;
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

    {#if mergeMessage}
        <p class="banner" role="status">{mergeMessage}</p>
    {/if}

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
                        {#if confirming === dup.pid}
                            <span>Merge into this record?</span>
                            <button
                                class="button"
                                onclick={() => handleMerge(dup.pid)}
                                disabled={merging === dup.pid}
                            >
                                {merging === dup.pid ? "Merging…" : "Confirm merge"}
                            </button>
                            <button onclick={() => (confirming = null)} disabled={merging === dup.pid}>
                                Cancel
                            </button>
                        {:else}
                            <button
                                onclick={() => {
                                    confirming = dup.pid;
                                }}
                                disabled={merging !== null}
                            >
                                Merge into this record
                            </button>
                        {/if}
                    </li>
                {/each}
            </ul>
        {/if}
    {/if}

    <div class="row" style="margin-top:1rem">
        <button class="button" onclick={toggleAudit}>
            {showAudit ? "Hide audit trail" : "Show audit trail"}
        </button>
    </div>

    {#if showAudit}
        <h2>Audit trail</h2>
        {#if auditLoading}
            <p>Loading audit trail…</p>
        {:else if auditError}
            <p class="banner" role="alert">{auditError}</p>
        {:else if audit && audit.length > 0}
            <ul class="stack">
                {#each audit as entry, i (i)}
                    <li class="surface row">
                        <strong>{entry.action}</strong>
                        <span>{entry.actor ?? "—"}</span>
                        {#if entry.created_at}<span>{entry.created_at}</span>{/if}
                    </li>
                {/each}
            </ul>
        {:else}
            <p>No audit entries.</p>
        {/if}
    {/if}
{/if}
