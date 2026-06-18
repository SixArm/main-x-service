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
    import { t, translate, i18n } from "$lib/i18n.svelte.js";

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
            error = t("merge.bothIdsRequired");
            return;
        }
        // Guard against merging a record into itself.
        if (mainId === duplicateId) {
            error = t("merge.mustDiffer");
            return;
        }
        if (!confirm(translate("merge.confirm", i18n.locale).replace("{dup}", duplicateId.slice(0, 8)).replace("{main}", mainId.slice(0, 8)))) return;
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
        if (!p) return t("detail.empty");
        return `${p.name}${p.additional_type ? ` (${p.additional_type})` : ""}`;
    }
</script>

<svelte:head><title>Merge courses · Course Service</title></svelte:head>

<header><h1>{t("merge.title")}</h1></header>

<section class="surface stack">
    <FieldRow>
        <LabeledField label={t("merge.mainId")} for="merge-main" required hint={t("merge.mainIdHint")}>
            <input id="merge-main" bind:value={mainId} />
        </LabeledField>
        <LabeledField label={t("merge.duplicateId")} for="merge-dup" required hint={t("merge.duplicateIdHint")}>
            <input id="merge-dup" bind:value={duplicateId} />
        </LabeledField>
    </FieldRow>
    <LabeledField label={t("merge.reason")} for="merge-reason" hint={t("merge.reasonHint")}>
        <input id="merge-reason" bind:value={reason} placeholder={t("merge.reasonPlaceholder")} />
    </LabeledField>
    <div class="row">
        <button type="button" class="button" onclick={loadPreview}>{t("merge.loadPreview")}</button>
        <button type="button" class="button primary" onclick={doMerge} disabled={loading}>
            {loading ? t("merge.merging") : t("merge.merge")}
        </button>
    </div>
    {#if error}<div class="banner error">{error}</div>{/if}
</section>

{#if preview.main || preview.duplicate}
    <section class="surface stack">
        <h2>{t("merge.preview")}</h2>
        <dl class="kv">
            <dt>{t("merge.main")}</dt><dd>{summary(preview.main)}</dd>
            <dt>{t("merge.duplicate")}</dt><dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    <section class="surface stack">
        <h2>{t("merge.completed")}</h2>
        <p>{translate("merge.recordCreated", i18n.locale).replace("{id}", result.merge_record.id).replace("{at}", new Date(result.merge_record.merged_at).toLocaleString())}</p>
        <a href={`/courses/${result.main_course.id}`} class="button primary"
           onclick={() => result?.main_course.id && goto(`/courses/${result.main_course.id}`)}>
            {t("merge.viewMerged")}
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
