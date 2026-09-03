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
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { Event, MergeResponse } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();

    let mainId = $state("");
    let duplicateId = $state("");
    let reason = $state("");
    let preview = $state<{ main: Event | null; duplicate: Event | null }>({
        main: null,
        duplicate: null,
    });
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
        if (!mainId || !duplicateId) {
            error = translate("merge.bothIdsRequired");
            return;
        }
        if (mainId === duplicateId) {
            error = translate("merge.idsMustDiffer");
            return;
        }
        if (
            !confirm(
                translate("merge.confirm")
                    .replace("{dup}", duplicateId.slice(0, 8))
                    .replace("{main}", mainId.slice(0, 8)),
            )
        )
            return;
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

    // Split the localized "merge completed" template at {id} so the merge
    // record id renders inside a <code> element while {at} stays inline.
    function completedParts(): { before: string; after: string } {
        const [before = "", rest = ""] = translate("merge.completedBody").split(
            "{id}",
        );
        return { before, after: rest };
    }

    // One-line preview label: event name plus its (localized) start date.
    function summary(e: Event | null): string {
        if (!e) return translate("merge.preview.none");
        const when = e.start_date
            ? new Date(e.start_date).toLocaleString()
            : translate("merge.preview.noDate");
        return `${e.name} (${when})`;
    }
</script>

<svelte:head><title>Merge events · Event Service</title></svelte:head>

<header><h1>{t("merge.title")}</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField
            label={t("merge.mainId")}
            for="merge-main"
            required
            hint={t("merge.mainIdHint")}
        >
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField
            label={t("merge.dupId")}
            for="merge-dup"
            required
            hint={t("merge.dupIdHint")}
        >
            <input id="merge-dup" bind:value={duplicateId} />
        </LabeledField>
    </FieldRow>
    <LabeledField
        label={t("merge.reason")}
        for="merge-reason"
        hint={t("merge.reasonHint")}
    >
        <input
            id="merge-reason"
            bind:value={reason}
            placeholder={t("merge.reasonPlaceholder")}
        />
    </LabeledField>
    <div class="row">
        <button type="button" class="button" onclick={loadPreview}
            >{t("merge.loadPreview")}</button
        >
        <button
            type="button"
            class="button primary"
            onclick={doMerge}
            disabled={loading}
        >
            {loading ? t("merge.merging") : t("merge.merge")}
        </button>
    </div>
    {#if error}<div class="banner error">{error}</div>{/if}
</section>

{#if preview.main || preview.duplicate}
    <section class="surface stack">
        <h2>{t("merge.previewTitle")}</h2>
        <dl class="kv">
            <dt>{t("merge.preview.main")}</dt>
            <dd>{summary(preview.main)}</dd>
            <dt>{t("merge.preview.duplicate")}</dt>
            <dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    <section class="surface stack">
        <h2>{t("merge.completedTitle")}</h2>
        <p>
            {completedParts().before}<code>{result.merge_record.id}</code
            >{completedParts().after.replace(
                "{at}",
                new Date(result.merge_record.merged_at).toLocaleString(),
            )}
        </p>
        <a
            href={`/events/${result.main_event.id}`}
            class="button primary"
            onclick={() =>
                result?.main_event.id &&
                goto(`/events/${result.main_event.id}`)}
        >
            {t("merge.viewMerged")}
        </a>
    </section>
{/if}

<style>
    .kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 1rem;
        row-gap: 0.25rem;
    }
    dt {
        font-weight: 600;
    }
    dd {
        margin: 0;
    }
</style>
