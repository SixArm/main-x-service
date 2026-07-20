<script lang="ts">
    // Detail route ("/[pid]") — view one care pathway plus its actions:
    // edit (link), delete, check-duplicates, inline two-step merge, and a
    // lazily-loaded audit trail.
    //
    // State ($state):
    //   - pathway / loading / error — the fetched record + load lifecycle.
    //   - duplicates — null until "Check duplicates" runs, then the scored
    //     list (self-row filtered out); checking — its busy flag.
    //   - confirming — pid of the row whose inline "Confirm merge?" prompt
    //     is armed (null = none); merging — pid of the merge in flight.
    //   - mergeMessage — success banner text after a merge.
    //   - showAudit / audit / auditLoading / auditError — the audit panel.
    // No props. The detail record is always treated as the merge SURVIVOR.
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type {
        AuditEntry,
        CarePathway,
        PathwayInstance,
        ScoredRef,
    } from "$lib/api/types";
    import { t, tf } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();
    const pid = page.params.pid ?? "";

    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let duplicates = $state<ScoredRef[] | null>(null);
    let checking = $state(false);
    // pid of the merge currently in flight, or null.
    let merging = $state<string | null>(null);
    // pid of the row with an armed inline confirm prompt, or null.
    let confirming = $state<string | null>(null);
    let mergeMessage = $state<string | null>(null);

    // Audit trail: lazy-loaded behind a toggle so the detail page stays
    // lean on first paint.
    let showAudit = $state(false);
    let audit = $state<AuditEntry[] | null>(null);
    let auditLoading = $state(false);
    let auditError = $state<string | null>(null);

    // The pathway's enrolled instances (`GET /{pid}/instances`). Loaded
    // alongside the record; a failure here is non-fatal to the detail view.
    let instances = $state<PathwayInstance[] | null>(null);

    // Load the record on mount, then its instances (best-effort).
    onMount(async () => {
        try {
            pathway = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("detail.notFound");
        } finally {
            loading = false;
        }
        try {
            instances = await repo.listInstances(pid);
        } catch {
            instances = [];
        }
    });

    // Soft-delete this record, then return to the list.
    async function handleDelete() {
        await repo.remove(pid);
        await goto("/");
    }

    // Run a duplicate check against this record and show the candidates.
    async function handleCheckDuplicates() {
        if (!pathway) return;
        checking = true;
        // Reset any prior merge banner / armed confirm before re-checking.
        mergeMessage = null;
        confirming = null;
        try {
            const hits = await repo.checkDuplicates(pathway);
            // Drop the self-match: a record always matches itself.
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("detail.checkFailed");
        } finally {
            checking = false;
        }
    }

    /// This detail record is the survivor (main); the row's pid is the
    /// duplicate to fold in. Two-step: arm `confirming`, then merge.
    async function handleMerge(duplicatePid: string) {
        // Guard: equal pids would 422; should never happen here.
        if (duplicatePid === pid) {
            error = t("detail.cannotMergeSelf");
            confirming = null;
            return;
        }
        // Mark this row's merge in flight (disables its buttons).
        merging = duplicatePid;
        error = null;
        mergeMessage = null;
        try {
            const result = await repo.merge(pid, duplicatePid);
            // The survivor's data may have changed: use the returned
            // record, then re-fetch the duplicates list.
            pathway = result.main;
            mergeMessage = tf("detail.mergedInto", { dup: duplicatePid });
            confirming = null;
            // Refresh candidates against the post-merge survivor (the just-
            // merged duplicate should now be gone).
            const hits = await repo.checkDuplicates(result.main);
            duplicates = hits.filter((h) => h.pid !== pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("detail.mergeFailed");
        } finally {
            merging = null;
        }
    }

    /// Toggle the audit panel; lazy-load the trail on first open.
    async function toggleAudit() {
        showAudit = !showAudit;
        // Fetch only on open, and only once / not while already loading.
        if (!showAudit || audit !== null || auditLoading) return;
        auditLoading = true;
        auditError = null;
        try {
            const rows = await repo.audit(pid);
            // Defensive newest-first sort by created_at (the service
            // already orders this way; tolerate missing timestamps).
            audit = [...rows].sort((a, b) =>
                (b.created_at ?? "").localeCompare(a.created_at ?? ""),
            );
        } catch (err) {
            auditError = err instanceof Error ? err.message : t("detail.auditLoadFailed");
        } finally {
            auditLoading = false;
        }
    }
</script>

<svelte:head><title>{pathway?.name ?? t("detail.fallbackName")} — Main X</title></svelte:head>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if pathway}
    <h1>{pathway.name}</h1>
    <div class="surface stack">
        {#if pathway.care_setting}
            <div><strong>{t("detail.careSetting")}</strong> {typeof pathway.care_setting === "string" ? pathway.care_setting : pathway.care_setting.Custom}</div>
        {/if}
        {#if pathway.provider_name}<div><strong>{t("detail.provider")}</strong> {pathway.provider_name}</div>{/if}
        {#if pathway.pathway_code}<div><strong>{t("detail.pathwayCode")}</strong> <code>{pathway.pathway_code}</code></div>{/if}
        {#if pathway.condition_codes && pathway.condition_codes.length > 0}
            <div>
                <strong>{t("detail.conditionCodes")}</strong>
                {pathway.condition_codes
                    .map((c) => `${typeof c.system === "string" ? c.system : c.system.Custom}:${c.code}`)
                    .join(", ")}
            </div>
        {/if}
        {#if pathway.identifiers && pathway.identifiers.length > 0}
            <div>
                <strong>{t("detail.identifiers")}</strong>
                <ul>
                    {#each pathway.identifiers as id, i (i)}
                        <li>{typeof id.scheme === "string" ? id.scheme : `Custom(${id.scheme.Custom})`}: <code>{id.value}</code></li>
                    {/each}
                </ul>
            </div>
        {/if}
        {#if pathway.interventions && pathway.interventions.length > 0}
            <div><strong>{t("detail.interventions")}</strong> {pathway.interventions.join(", ")}</div>
        {/if}
        {#if pathway.keywords && pathway.keywords.length > 0}
            <div><strong>{t("detail.keywords")}</strong> {pathway.keywords.join(", ")}</div>
        {/if}
        {#if pathway.in_language && pathway.in_language.length > 0}
            <div><strong>{t("detail.languages")}</strong> {pathway.in_language.join(", ")}</div>
        {/if}
        <div><strong>{t("detail.id")}</strong> <code>{pid}</code></div>
    </div>

    <!-- Enrolled instances on this pathway template (people on the pathway). -->
    <section data-testid="pathway-instances" style="margin-top:1rem">
        <h2>Instances</h2>
        {#if instances === null}
            <p>{t("detail.loading")}</p>
        {:else if instances.length === 0}
            <p class="surface">No instances enrolled on this pathway.</p>
        {:else}
            <ul class="stack">
                {#each instances as instance (instance.pid)}
                    <li class="surface row">
                        <a href={`/board`}>{instance.subject_ref}</a>
                        <span class="muted">{instance.status}</span>
                        <span class="muted">{instance.urgency}</span>
                        <span class="muted small">{instance.enrolled_on}</span>
                    </li>
                {/each}
            </ul>
        {/if}
    </section>

    <div class="row" style="margin-top:1rem">
        <a class="button" href={`/${pid}/edit`}>{t("detail.edit")}</a>
        <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
            {checking ? t("detail.checking") : t("detail.checkDuplicates")}
        </button>
        <button onclick={handleDelete}>{t("detail.delete")}</button>
    </div>

    {#if mergeMessage}
        <p class="banner" role="status">{mergeMessage}</p>
    {/if}

    <!-- Potential-duplicates list (shown after a check). Each row offers a
         two-step merge: "Merge into this record" arms the inline confirm
         (`confirming === dup.pid`), then "Confirm merge" folds it in. -->
    {#if duplicates}
        <h2>{t("detail.potentialDuplicates")}</h2>
        {#if duplicates.length === 0}
            <p>{t("detail.noneAboveThreshold")}</p>
        {:else}
            <ul class="stack">
                {#each duplicates as dup (dup.pid)}
                    <li class="surface row">
                        <a href={`/${dup.pid}`}>{dup.name}</a>
                        <span>{dup.score.toFixed(3)} · {dup.confidence}</span>
                        {#if confirming === dup.pid}
                            <span>{t("detail.mergeIntoConfirm")}</span>
                            <button
                                class="button"
                                onclick={() => handleMerge(dup.pid)}
                                disabled={merging === dup.pid}
                            >
                                {merging === dup.pid ? t("detail.merging") : t("detail.confirmMerge")}
                            </button>
                            <button onclick={() => (confirming = null)} disabled={merging === dup.pid}>
                                {t("detail.cancel")}
                            </button>
                        {:else}
                            <button
                                onclick={() => {
                                    confirming = dup.pid;
                                }}
                                disabled={merging !== null}
                            >
                                {t("detail.mergeInto")}
                            </button>
                        {/if}
                    </li>
                {/each}
            </ul>
        {/if}
    {/if}

    <div class="row" style="margin-top:1rem">
        <button class="button" onclick={toggleAudit}>
            {showAudit ? t("detail.hideAudit") : t("detail.showAudit")}
        </button>
    </div>

    <!-- Audit-trail panel: rendered only when toggled open; rows are
         newest-first by created_at (sorted in `toggleAudit`), with "—"
         shown for a null actor. -->
    {#if showAudit}
        <h2>{t("detail.auditTrail")}</h2>
        {#if auditLoading}
            <p>{t("detail.loadingAudit")}</p>
        {:else if auditError}
            <p class="banner" role="alert">{auditError}</p>
        {:else if audit && audit.length > 0}
            <ul class="stack">
                {#each audit as entry, i (i)}
                    <li class="surface row">
                        <strong>{entry.action}</strong>
                        <span>{entry.actor ?? "—"}</span>
                        {#if entry.created_at}<span>{entry.created_at}</span>{/if}
                    </li>
                {/each}
            </ul>
        {:else}
            <p>{t("detail.noAuditEntries")}</p>
        {/if}
    {/if}
{/if}
