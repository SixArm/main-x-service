<!--
  Merge persons (/persons/merge) — manually merge a duplicate record into a
  surviving main record by id.

  Both ids may arrive pre-filled as `?main=…&duplicate=…` (the review
  board deep-links a confirmed pair here); otherwise the operator types
  them.

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
    import { page } from "$app/state";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { ApiError } from "$lib/api/client.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { MergeResponse, Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();

    // Seeded once from `?main=` / `?duplicate=` so the review board can
    // deep-link a confirmed pair straight into this form. Both stay fully
    // editable afterwards — a review item names an unordered pair, so which
    // record survives is the operator's call, not the link's.
    let mainId = $state(page.url.searchParams.get("main") ?? "");
    let duplicateId = $state(page.url.searchParams.get("duplicate") ?? "");
    let reason = $state("");
    let preview = $state<{ main: Person | null; duplicate: Person | null }>({
        main: null,
        duplicate: null,
    });
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
            error = t("merge.bothIdsRequired");
            return;
        }
        if (mainId === duplicateId) {
            error = t("merge.mustDiffer");
            return;
        }
        if (
            !confirm(
                `${t("merge.confirm.prefix")}${duplicateId.slice(0, 8)}…${t("merge.confirm.into")}${mainId.slice(0, 8)}…${t("merge.confirm.suffix")}`,
            )
        )
            return;
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
        if (!p) return t("merge.noRecord");
        return `${p.name.given.join(" ")} ${p.name.family} (${p.birth_date ?? t("merge.noDob")}, ${p.gender})`;
    }
</script>

<svelte:head><title>{t("merge.head.title")}</title></svelte:head>

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
    <section class="surface stack">
        <h2>{t("merge.completed")}</h2>
        <p>
            {t("merge.recordPrefix")} <code>{result.merge_record.id}</code>
            {t("merge.recordCreatedAt")}
            {new Date(result.merge_record.merged_at).toLocaleString()}.
        </p>
        <a
            href={`/persons/${result.main_person.id}`}
            class="button primary"
            onclick={() =>
                result?.main_person.id &&
                goto(`/persons/${result.main_person.id}`)}
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
