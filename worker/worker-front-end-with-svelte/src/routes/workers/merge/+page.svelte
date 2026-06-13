<script lang="ts">
    import { goto } from "$app/navigation";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MergeResponse, Worker } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Worker | null; duplicate: Worker | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    async function loadPreview() {
        preview = { main: null, duplicate: null };
        error = null;
        try {
            const [m, d] = await Promise.all([
                mainId ? repo.get(mainId) : Promise.resolve(null),
                duplicateId ? repo.get(duplicateId) : Promise.resolve(null),
            ]);
            preview = { main: m, duplicate: d };
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        }
    }

    async function doMerge() {
        if (!mainId || !duplicateId) {
            error = "Both IDs required";
            return;
        }
        if (mainId === duplicateId) {
            error = "Main and duplicate must differ";
            return;
        }
        if (!confirm(`Merge ${duplicateId.slice(0, 8)}… into ${mainId.slice(0, 8)}…?\nThis soft-deletes the duplicate.`)) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_worker_id: mainId,
                duplicate_worker_id: duplicateId,
                merge_reason: reason || null,
            });
        } catch (err) {
            if (err instanceof ApiError) {
                error = `${err.code}: ${err.message}`;
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            loading = false;
        }
    }

    function summary(p: Worker | null): string {
        if (!p) return "—";
        return `${p.name.given.join(" ")} ${p.name.family} (${p.birth_date ?? "no DOB"}, ${p.gender})`;
    }
</script>

<svelte:head><title>Merge workers · Worker Service</title></svelte:head>

<header><h1>Merge workers</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label="Main worker ID" for="merge-main" required hint="The surviving record">
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label="Duplicate worker ID" for="merge-dup" required hint="Will be soft-deleted">
            <input id="merge-dup" bind:value={duplicateId} />
        </LabeledField>
    </FieldRow>
    <LabeledField label="Reason" for="merge-reason" hint="Recorded in the merge audit trail">
        <input id="merge-reason" bind:value={reason} placeholder="Confirmed duplicate" />
    </LabeledField>
    <div class="row">
        <button type="button" class="button" onclick={loadPreview}>Load preview</button>
        <button type="button" class="button primary" onclick={doMerge} disabled={loading}>
            {loading ? "Merging…" : "Merge"}
        </button>
    </div>
    {#if error}<div class="banner error">{error}</div>{/if}
</section>

{#if preview.main || preview.duplicate}
    <section class="surface stack">
        <h2>Preview</h2>
        <dl class="kv">
            <dt>Main</dt><dd>{summary(preview.main)}</dd>
            <dt>Duplicate</dt><dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    <section class="surface stack">
        <h2>Merge completed</h2>
        <p>Merge record <code>{result.merge_record.id}</code> created at {new Date(result.merge_record.merged_at).toLocaleString()}.</p>
        <a href={`/workers/${result.main_worker.id}`} class="button primary"
           onclick={() => result?.main_worker.id && goto(`/workers/${result.main_worker.id}`)}>
            View merged main worker
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
