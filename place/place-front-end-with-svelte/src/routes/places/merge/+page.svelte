<!--
  Merge places (route "/places/merge") — operator tool to merge a duplicate
  place into a surviving main place by id, with an optional preview step and
  a confirm() guard (the merge soft-deletes the duplicate).

  Both ids may arrive pre-filled as `?main=…&duplicate=…` (the review
  board deep-links a confirmed pair here); otherwise the operator types
  them.

  Local $state:
    - mainId / duplicateId / reason — merge request inputs.
    - preview  — optional fetched main/duplicate records for review.
    - result   — successful MergeResponse, rendered as a confirmation.
    - error / loading — request status.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { ApiError } from "$lib/api/client.js";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { MergeResponse, Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();

    // Seeded once from `?main=` / `?duplicate=` so the review board can
    // deep-link a confirmed pair straight into this form. Both stay fully
    // editable afterwards — a review item names an unordered pair, so which
    // record survives is the operator's call, not the link's.
    let mainId = $state(page.url.searchParams.get("main") ?? "");
    let duplicateId = $state(page.url.searchParams.get("duplicate") ?? "");
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
        if (!mainId || !duplicateId) { error = translate("merge.bothIdsRequired"); return; }
        if (mainId === duplicateId) { error = translate("merge.mustDiffer"); return; }
        if (!confirm(translate("merge.confirm").replace("{duplicate}", duplicateId.slice(0, 8)).replace("{main}", mainId.slice(0, 8)))) return;
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
            <dt>{t("merge.previewMain")}</dt><dd>{summary(preview.main)}</dd>
            <dt>{t("merge.previewDuplicate")}</dt><dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    <section class="surface stack">
        <h2>{t("merge.completed")}</h2>
        <p>{t("merge.recordCreated").split("{id}")[0]}<code>{result.merge_record.id}</code>{t("merge.recordCreated").split("{id}")[1]?.replace("{at}", new Date(result.merge_record.merged_at).toLocaleString())}</p>
        <a href={`/places/${result.main_place.id}`} class="button primary"
           onclick={() => result?.main_place.id && goto(`/places/${result.main_place.id}`)}>
            {t("merge.viewMain")}
        </a>
    </section>
{/if}

<style>
    .kv { display: grid; grid-template-columns: max-content 1fr; column-gap: 1rem; row-gap: 0.25rem; }
    dt { font-weight: 600; }
    dd { margin: 0; }
</style>
