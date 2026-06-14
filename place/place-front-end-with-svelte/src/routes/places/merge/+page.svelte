<!--
  Merge places (route "/places/merge") — operator tool to merge a duplicate
  place into a surviving main place by id, with an optional preview step and
  a confirm() guard (the merge soft-deletes the duplicate).

  Local $state:
    - mainId / duplicateId / reason — merge request inputs.
    - preview  — optional fetched main/duplicate records for review.
    - result   — successful MergeResponse, rendered as a confirmation.
    - error / loading — request status.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MergeResponse, Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Place | null; duplicate: Place | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch both records in parallel for the preview; each id is optional,
    // so a missing id resolves to null rather than firing a request.
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

    // Validate, confirm, then perform the merge. Guards prevent empty ids
    // and self-merge; confirm() is the last chance before the destructive op.
    async function doMerge() {
        if (!mainId || !duplicateId) { error = "Both IDs required"; return; }
        if (mainId === duplicateId) { error = "Main and duplicate must differ"; return; }
        if (!confirm(`Merge ${duplicateId.slice(0, 8)}… into ${mainId.slice(0, 8)}…?\nThis soft-deletes the duplicate.`)) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_place_id: mainId,
                duplicate_place_id: duplicateId,
                merge_reason: reason || null, // empty string → null (omit reason)
            });
        } catch (err) {
            // Prefer the structured "code: message" for ApiErrors.
            if (err instanceof ApiError) {
                error = `${err.code}: ${err.message}`;
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            loading = false;
        }
    }

    // One-line "name (city)" label for the preview rows.
    function summary(p: Place | null): string {
        if (!p) return "—";
        const city = p.address?.address_locality ?? "";
        return `${p.name}${city ? ` (${city})` : ""}`;
    }
</script>

<svelte:head><title>Merge places · Place Service</title></svelte:head>

<header><h1>Merge places</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label="Main place ID" for="merge-main" required hint="The surviving record">
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label="Duplicate place ID" for="merge-dup" required hint="Will be soft-deleted">
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
        <a href={`/places/${result.main_place.id}`} class="button primary"
           onclick={() => result?.main_place.id && goto(`/places/${result.main_place.id}`)}>
            View merged main place
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
