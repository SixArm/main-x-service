<!--
  Merge courses (route "/courses/merge") — operator enters a main
  (surviving) and a duplicate course ID, optionally previews both, then
  merges. The duplicate is soft-deleted into the main; on success a
  merge record and a link to the surviving course are shown.

  Reactive state:
    - mainId / duplicateId / reason — merge inputs.
    - preview — optionally-loaded main/duplicate courses for confirmation.
    - result — the MergeResponse on success.
    - loading / error — request status and validation/error messages.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MergeResponse, Course } from "$lib/api/types.js";

    const repo = CourseRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Course | null; duplicate: Course | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch whichever IDs are filled, in parallel, for the preview pane.
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

    // Validate the two IDs, confirm, then run the merge.
    async function doMerge() {
        if (!mainId || !duplicateId) {
            error = "Both IDs required";
            return;
        }
        // Guard against merging a record into itself.
        if (mainId === duplicateId) {
            error = "Main and duplicate must differ";
            return;
        }
        if (!confirm(`Merge ${duplicateId.slice(0, 8)}… into ${mainId.slice(0, 8)}…?\nThis soft-deletes the duplicate.`)) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_course_id: mainId,
                duplicate_course_id: duplicateId,
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

    // One-line label for a preview course: name plus optional type.
    function summary(p: Course | null): string {
        if (!p) return "—";
        return `${p.name}${p.additional_type ? ` (${p.additional_type})` : ""}`;
    }
</script>

<svelte:head><title>Merge courses · Course Service</title></svelte:head>

<header><h1>Merge courses</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label="Main course ID" for="merge-main" required hint="The surviving record">
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label="Duplicate course ID" for="merge-dup" required hint="Will be soft-deleted">
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
        <a href={`/courses/${result.main_course.id}`} class="button primary"
           onclick={() => result?.main_course.id && goto(`/courses/${result.main_course.id}`)}>
            View merged main course
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
