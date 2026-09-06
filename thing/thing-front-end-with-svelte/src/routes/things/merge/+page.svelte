<!--
  +page.svelte (/things/merge) — merge two Things.

  Purpose: collects a surviving main id and a duplicate id (plus a reason),
  optionally previews both records, then merges the duplicate into the main
  after confirmation. On success shows the merge record and a link to the
  merged main Thing.

  Both ids may arrive pre-filled as `?main=…&duplicate=…` (the review
  screen deep-links a confirmed pair here); otherwise the operator types
  them.

  $state:
    - mainId / duplicateId / reason: bound form fields.
    - preview: the two loaded Things for side-by-side confirmation.
    - result: the MergeResponse after a successful merge.
    - error / loading: validation/request status.

  Reactive notes: doMerge validates locally (both ids present and distinct)
  before confirming and calling the API; ApiError codes are shown verbatim.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { ApiError } from "$lib/api/client.js";
    import { describeApiError } from "$lib/api/errorHandling.js";
    import { validateMerge } from "$lib/components/merge-validation.js";
    import type { MergeResponse, Thing } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();

    // Seeded once from `?main=` / `?duplicate=` so the review screen can
    // deep-link a confirmed pair straight into this form. Both stay fully
    // editable afterwards — a review item names an unordered pair, so
    // which record survives is the operator's call, not the link's.
    let mainId = $state(page.url.searchParams.get("main") ?? "");
    let duplicateId = $state(page.url.searchParams.get("duplicate") ?? "");
    let reason = $state("");
    let preview = $state<{ main: Thing | null; duplicate: Thing | null }>({
        main: null,
        duplicate: null,
    });
    let result = $state<MergeResponse | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Fetch both records (whichever ids are filled) in parallel for preview.
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
            error = describeApiError(err);
        }
    }

    async function doMerge() {
        // Guard (FR-9): both ids required and they must differ (can't merge
        // into self). Logic lives in a pure helper so it is unit-testable.
        const guardError = validateMerge(mainId, duplicateId);
        if (guardError) {
            error = guardError;
            return;
        }
        // Destructive (soft-deletes the duplicate) — confirm before proceeding.
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
                main_thing_id: mainId,
                duplicate_thing_id: duplicateId,
                merge_reason: reason || null,
            });
        } catch (err) {
            if (
                err instanceof ApiError &&
                (err.isUnauthorized || err.isForbidden)
            ) {
                error = describeApiError(err);
            } else if (err instanceof ApiError) {
                // Prefer the service's structured "CODE: message" for API errors.
                error = `${err.code}: ${err.message}`;
            } else {
                error = err instanceof Error ? err.message : String(err);
            }
        } finally {
            loading = false;
        }
    }

    // One-line preview summary "Name (additional_type)" for the preview table.
    function summary(p: Thing | null): string {
        if (!p) return "—";
        return `${p.name}${p.additional_type ? ` (${p.additional_type})` : ""}`;
    }
</script>

<svelte:head><title>Merge things · Thing Service</title></svelte:head>

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
        <h2>{t("merge.preview")}</h2>
        <dl class="kv">
            <dt>{t("merge.main")}</dt>
            <dd>{summary(preview.main)}</dd>
            <dt>{t("merge.duplicate")}</dt>
            <dd>{summary(preview.duplicate)}</dd>
        </dl>
    </section>
{/if}

{#if result}
    {@const parts = translate("merge.recordCreated")
        .replace(
            "{at}",
            new Date(result.merge_record.merged_at).toLocaleString(),
        )
        .split("{id}")}
    <section class="surface stack">
        <h2>{t("merge.completed")}</h2>
        <p>{parts[0]}<code>{result.merge_record.id}</code>{parts[1] ?? ""}</p>
        <!-- SPA navigation to the surviving main thing's detail page. -->
        <a
            href={`/things/${result.main_thing.id}`}
            class="button primary"
            onclick={() =>
                result?.main_thing.id &&
                goto(`/things/${result.main_thing.id}`)}
        >
            {t("merge.viewMain")}
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
