<!--
  Merge persons (/persons/merge) — manually merge a duplicate record into a
  surviving main record by id.

  The operator enters both ids (+ optional reason), optionally loads a
  side-by-side preview, then merges with a confirmation. The merge
  soft-deletes the duplicate; on success a link to the surviving record is
  shown.

  State:
    - mainId/duplicateId/reason — the merge inputs.
    - preview — loaded main/duplicate records for the side-by-side view.
    - result — the MergeResponse after a successful merge.
    - error / loading — request lifecycle.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MergeResponse, Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Person | null; duplicate: Person | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch both records in parallel for the side-by-side preview. A blank id
    // resolves to null so a partial preview still works.
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

    // Validate locally, confirm the destructive action, then merge.
    async function doMerge() {
        // Guard: both ids required and they must differ (can't merge a record
        // into itself).
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
                main_person_id: mainId,
                duplicate_person_id: duplicateId,
                merge_reason: reason || null,
            });
        } catch (err) {
            // Prefer the structured "CODE: message" form for ApiError.
            if (err instanceof ApiError) {
                error = `${err.code}: ${err.message}`;
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            loading = false;
        }
    }

    // One-line human summary of a person for the preview table.
    function summary(p: Person | null): string {
        if (!p) return "—";
        return `${p.name.given.join(" ")} ${p.name.family} (${p.birth_date ?? "no DOB"}, ${p.gender})`;
    }
</script>

<svelte:head><title>Merge persons · Person Service</title></svelte:head>

<header><h1>Merge persons</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label="Main person ID" for="merge-main" required hint="The surviving record">
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label="Duplicate person ID" for="merge-dup" required hint="Will be soft-deleted">
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
        <a href={`/persons/${result.main_person.id}`} class="button primary"
           onclick={() => result?.main_person.id && goto(`/persons/${result.main_person.id}`)}>
            View merged main person
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
