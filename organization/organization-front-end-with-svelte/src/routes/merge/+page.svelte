<!--
  Merge route (`/merge`) — fold one organization into another.

  Collects a surviving main pid and a duplicate pid (plus an optional
  reason), optionally previews both records side by side, then merges
  after an explicit confirmation. The service's response carries the two
  pids and the survivor's post-merge payload — there is deliberately no
  `merge_record` wrapper here (see `src/lib/api/types.ts`), so the
  completion panel links to the survivor rather than quoting a record id.

  $state:
    - mainPid / duplicatePid / reason: bound form fields.
    - preview: the two loaded organizations for side-by-side confirmation.
    - result:  the MergeResponse after a successful merge.
    - recent:  the merge history (newest first), loaded on mount.
    - error / loading: validation + request status.

  The merge is destructive (it soft-deletes the duplicate), so it is
  guarded by a pure validator and a native confirm() before the POST.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { OrganizationRepository } from "$lib/api/organizations";
    import { ApiError } from "$lib/api/client";
    import { validateMerge } from "$lib/components/merge-validation";
    import { t, translate } from "$lib/i18n.svelte";
    import type {
        MergeRecordRow,
        MergeResponse,
        Organization,
    } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();

    let mainPid = $state("");
    let duplicatePid = $state("");
    let reason = $state("");
    let preview = $state<{ main: Organization | null; duplicate: Organization | null }>({
        main: null,
        duplicate: null,
    });
    let result = $state<MergeResponse | null>(null);
    let recent = $state<MergeRecordRow[] | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // The history is a safe GET, so it loads on mount; the merge itself
    // is never a page-load side effect.
    onMount(() => {
        void loadRecent();
    });

    /** Render an unknown thrown value as a message, preferring the
     *  service's status + message for an ApiError (this client's
     *  ApiError carries `status`, not a `code`). */
    function describe(err: unknown): string {
        if (err instanceof ApiError) return `${err.status}: ${err.message}`;
        return err instanceof Error ? err.message : String(err);
    }

    /** Fetch both records (whichever pids are filled) in parallel. */
    async function loadPreview() {
        preview = { main: null, duplicate: null };
        error = null;
        try {
            const [main, duplicate] = await Promise.all([
                mainPid ? repo.get(mainPid) : Promise.resolve(null),
                duplicatePid ? repo.get(duplicatePid) : Promise.resolve(null),
            ]);
            preview = { main, duplicate };
        } catch (err) {
            error = describe(err);
        }
    }

    /** Load the merge history (newest first). */
    async function loadRecent() {
        try {
            recent = await repo.recentMerges();
        } catch (err) {
            error = describe(err);
            recent = [];
        }
    }

    async function doMerge() {
        // Both pids required and they must differ (no self-merge). The
        // rule lives in a pure helper so it is unit-testable, and returns
        // an i18n key so the message follows the chosen locale.
        const guardKey = validateMerge(mainPid, duplicatePid);
        if (guardKey) {
            error = t(guardKey);
            return;
        }
        // Destructive (soft-deletes the duplicate) — confirm first.
        const question = translate("merge.confirm")
            .replace("{dup}", duplicatePid.slice(0, 8))
            .replace("{main}", mainPid.slice(0, 8));
        if (!confirm(question)) return;
        loading = true;
        error = null;
        try {
            result = await repo.merge({
                main_pid: mainPid,
                duplicate_pid: duplicatePid,
                reason: reason || null,
            });
            // The history just gained a row; refresh it rather than
            // making the operator reload the page.
            await loadRecent();
        } catch (err) {
            error = describe(err);
        } finally {
            loading = false;
        }
    }

    /** One-line preview summary "Name (legal name)" for the preview list. */
    function summary(org: Organization | null): string {
        if (!org) return "—";
        return `${org.name}${org.legal_name ? ` (${org.legal_name})` : ""}`;
    }
</script>

<svelte:head><title>{t("merge.title")} — Main X</title></svelte:head>

<h1>{t("merge.title")}</h1>

<section class="surface stack">
    <div class="row">
        <label>
            {t("merge.mainId")} <small class="muted">{t("merge.mainIdHint")}</small>
            <input id="merge-main" type="text" bind:value={mainPid} required />
        </label>
        <label>
            {t("merge.dupId")} <small class="muted">{t("merge.dupIdHint")}</small>
            <input id="merge-dup" type="text" bind:value={duplicatePid} required />
        </label>
    </div>
    <label>
        {t("merge.reason")} <small class="muted">{t("merge.reasonHint")}</small>
        <input
            id="merge-reason"
            type="text"
            bind:value={reason}
            placeholder={t("merge.reasonPlaceholder")}
        />
    </label>
    <div class="row">
        <button type="button" class="button" onclick={() => void loadPreview()}>
            {t("merge.loadPreview")}
        </button>
        <button
            type="button"
            class="button primary"
            onclick={() => void doMerge()}
            disabled={loading}
        >
            {loading ? t("merge.merging") : t("merge.merge")}
        </button>
    </div>
    {#if error}
        <p class="banner error" role="alert" data-testid="merge-error">{error}</p>
    {/if}
</section>

{#if preview.main || preview.duplicate}
    <section class="surface stack" data-testid="merge-preview">
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
    <section class="surface stack" data-testid="merge-result">
        <h2>{t("merge.completed")}</h2>
        <p>
            {translate("merge.completedDetail")
                .replace("{dup}", result.duplicate_pid)
                .replace("{main}", result.main_pid)}
        </p>
        <!-- SPA navigation to the surviving organization's detail page. -->
        <a
            class="button primary"
            href={`/${result.main_pid}`}
            onclick={() => result && void goto(`/${result.main_pid}`)}
        >
            {t("merge.viewMain")}
        </a>
    </section>
{/if}

<section class="stack" data-testid="merge-recent">
    <h2>{t("merge.recent")}</h2>
    <p>
        <button type="button" class="button" onclick={() => void loadRecent()}>
            {t("merge.recentRefresh")}
        </button>
    </p>
    {#if recent !== null}
        {#if recent.length === 0}
            <p class="surface">{t("merge.recentEmpty")}</p>
        {:else}
            <div class="table-wrap">
                <table>
                    <thead>
                        <tr>
                            <th>{t("merge.colMergedAt")}</th>
                            <th>{t("merge.main")}</th>
                            <th>{t("merge.duplicate")}</th>
                            <th>{t("merge.colReason")}</th>
                            <th>{t("merge.colActor")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each recent as row (row.id)}
                            <tr>
                                <td>{new Date(row.created_at).toLocaleString()}</td>
                                <td><a href={`/${row.main_pid}`}><code>{row.main_pid}</code></a></td>
                                <td><code>{row.duplicate_pid}</code></td>
                                <td>{row.reason ?? "—"}</td>
                                <td>{row.actor ?? "—"}</td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>
        {/if}
    {/if}
</section>

<style>
    .kv {
        display: grid;
        grid-template-columns: max-content 1fr;
        column-gap: 1rem;
        row-gap: 0.25rem;
        margin: 0;
    }
    dt {
        font-weight: 600;
    }
    dd {
        margin: 0;
    }
    label {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    .table-wrap {
        overflow-x: auto;
    }
    table {
        width: 100%;
        border-collapse: collapse;
        font-size: 0.875rem;
    }
    th,
    td {
        text-align: start;
        padding: 0.4rem 0.6rem;
        border-bottom: 1px solid var(--mxi-color-border, #ddd);
        white-space: nowrap;
    }
    th {
        font-weight: 600;
    }
</style>
