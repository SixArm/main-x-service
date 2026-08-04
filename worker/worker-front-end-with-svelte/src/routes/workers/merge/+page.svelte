<!--
  Merge workers (route "/workers/merge") — operator enters a surviving
  ("main") id and a duplicate id, optionally previews both, then merges.
  The duplicate is soft-deleted; on success a link to the merged main
  record is shown.

  Both ids may arrive pre-filled as `?main=…&duplicate=…` (the review
  board deep-links a confirmed pair here); otherwise the operator types
  them.

  $state:
    - mainId / duplicateId / reason — merge inputs.
    - preview — the two fetched records for side-by-side confirmation.
    - result — the MergeResponse after a successful merge.
    - error / loading — request status.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MergeResponse, Worker } from "$lib/api/types.js";
    import { t, tf } from "$lib/i18n.svelte.js";

    const repo = WorkerRepository.withFetch();

    // Seeded once from `?main=` / `?duplicate=` so the review board can
    // deep-link a confirmed pair straight into this form. Both stay fully
    // editable afterwards — a review item names an unordered pair, so which
    // record survives is the operator's call, not the link's.
    let mainId = $state(page.url.searchParams.get("main") ?? "");
    let duplicateId = $state(page.url.searchParams.get("duplicate") ?? "");
    let reason = $state("");
    let preview = $state<{ main: Worker | null; duplicate: Worker | null }>({ main: null, duplicate: null });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch whichever ids are filled in (in parallel) for the preview panel;
    // a blank id resolves to null rather than erroring.
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

    // Validate, confirm, then perform the merge. Both ids are required and
    // must differ; the confirm spells out the destructive soft-delete.
    async function doMerge() {
        if (!mainId || !duplicateId) {
            error = t("merge.bothIdsRequired");
            return;
        }
        if (mainId === duplicateId) {
            error = t("merge.mustDiffer");
            return;
        }
        if (!confirm(tf("merge.confirm", { dup: duplicateId.slice(0, 8), main: mainId.slice(0, 8) }))) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_worker_id: mainId,
                duplicate_worker_id: duplicateId,
                merge_reason: reason || null,
            });
        } catch (err) {
            // Show the service's error code alongside the message for ApiErrors.
            if (err instanceof ApiError) {
                error = `${err.code}: ${err.message}`;
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            loading = false;
        }
    }

    // One-line label for a previewed worker (or em dash when absent).
    function summary(p: Worker | null): string {
        if (!p) return t("merge.emDash");
        return `${p.name.given.join(" ")} ${p.name.family} (${p.birth_date ?? t("merge.noDob")}, ${p.gender})`;
    }
</script>

<svelte:head><title>{t("merge.titleTab")}</title></svelte:head>

<header><h1>{t("merge.heading")}</h1></header>

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
            <dt>{t("merge.previewMain")}</dt><dd>{summary(preview.main)}</dd>
            <dt>{t("merge.previewDuplicate")}</dt><dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    <section class="surface stack">
        <h2>{t("merge.completed")}</h2>
        <p>{tf("merge.recordCreated", { id: result.merge_record.id, when: new Date(result.merge_record.merged_at).toLocaleString() })}</p>
        <a href={`/workers/${result.main_worker.id}`} class="button primary"
           onclick={() => result?.main_worker.id && goto(`/workers/${result.main_worker.id}`)}>
            {t("merge.viewMain")}
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
