<!--
  Merge page (route "/events/merge") — merge a duplicate event into a
  surviving "main" event by id, with an optional pre-merge preview and a
  confirmation step. The duplicate is soft-deleted on success.

  State ($state): the two ids + reason, an optional preview pair, the merge
  result, and error/loading flags.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { ApiError } from "$lib/api/client.js";
    import type { Event, MergeResponse } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Event | null; duplicate: Event | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch both records (in parallel) so the operator can eyeball them
    // before merging; either id may be blank (resolves to null).
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

    // Validate ids, confirm, then perform the merge. ApiError is shown as
    // "code: message"; other errors fall back to their string form.
    async function doMerge() {
        if (!mainId || !duplicateId) { error = "Both IDs required"; return; }
        if (mainId === duplicateId) { error = "Main and duplicate must differ"; return; }
        if (!confirm(`Merge ${duplicateId.slice(0, 8)}… into ${mainId.slice(0, 8)}…?\nThis soft-deletes the duplicate.`)) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_event_id: mainId,
                duplicate_event_id: duplicateId,
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

    // One-line preview label: event name plus its (localized) start date.
    function summary(e: Event | null): string {
        if (!e) return "—";
        const when = e.start_date ? new Date(e.start_date).toLocaleString() : "no date";
        return `${e.name} (${when})`;
    }
</script>

<svelte:head><title>Merge events · Event Service</title></svelte:head>

<header><h1>Merge events</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label="Main event ID" for="merge-main" required hint="The surviving record">
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label="Duplicate event ID" for="merge-dup" required hint="Will be soft-deleted">
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
        <a href={`/events/${result.main_event.id}`} class="button primary"
           onclick={() => result?.main_event.id && goto(`/events/${result.main_event.id}`)}>
            View merged main event
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
