<!--
  Detail route (`/[pid]`): shows one organization, with delete and
  check-duplicates actions. A masked-view toggle (ORGFE-T2) re-fetches
  through GET /api/organizations/{pid}/masked instead of the plain
  record; a GDPR export action downloads the export envelope.

  $state:
    - org:        Organization | null — the loaded record.
    - loading:    boolean             — true until the first fetch settles.
    - error:      string | null       — fetch/action failure (inline banner).
    - duplicates: ScoredRef[] | null  — null = not checked; array = results.
    - checking:   boolean             — disables the button during a check.
    - masked:     boolean             — whether the masked view is shown;
      re-fetches on toggle rather than masking client-side, so this
      always reflects the server's actual masking rules.
    - exporting:  boolean             — disables the export button
      while the GDPR export request is in flight.
    - showAudit / audit / auditLoading / auditError — the audit panel
      (ORGFE-T3), lazy-loaded behind a toggle.

  `pid` comes from the route param. Loads on mount (SPA, client-only).
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { OrganizationRepository } from "$lib/api/organizations";
    import { excludeSelf } from "$lib/api/build";
    import { t } from "$lib/i18n.svelte";
    import type { AuditEntry, Organization, ScoredRef } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    // Route param; `?? ""` satisfies strict typing (param is always set here).
    const pid = page.params.pid ?? "";

    let org = $state<Organization | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let duplicates = $state<ScoredRef[] | null>(null);
    let checking = $state(false);
    let masked = $state(false);
    let exporting = $state(false);

    // Audit trail: lazy-loaded behind a toggle so the detail page stays
    // lean on first paint (ORGFE-T3, copy-adapted from
    // care-pathway-front-end-with-svelte's equivalent panel).
    let showAudit = $state(false);
    let audit = $state<AuditEntry[] | null>(null);
    let auditLoading = $state(false);
    let auditError = $state<string | null>(null);

    // Fetch the plain or masked record depending on `masked`, replacing
    // whatever is currently shown. Shared by the initial load and the
    // toggle handler so both go through one code path.
    async function load() {
        loading = true;
        error = null;
        try {
            org = masked ? await repo.masked(pid) : await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("detail.notFound");
        } finally {
            loading = false;
        }
    }

    // Flip the toggle and re-fetch through the new endpoint. A dedicated
    // request per view, not client-side redaction — the server, not this
    // page, decides what counts as sensitive.
    function toggleMasked() {
        masked = !masked;
        void load();
    }

    onMount(load);

    // GDPR export: fetch the service's export envelope and hand it to the
    // browser as a downloaded JSON file — the payload shape is
    // service-defined (`exportGdpr` returns `unknown`), so this never
    // interprets it, only serializes and saves what came back. A Blob
    // object URL through a synthetic anchor is the plain-browser way to
    // save client-held data; the URL is revoked once the click has fired.
    async function handleExportGdpr() {
        exporting = true;
        try {
            const data = await repo.exportGdpr(pid);
            const blob = new Blob([JSON.stringify(data, null, 2)], {
                type: "application/json",
            });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `organization-${pid}-export.json`;
            document.body.appendChild(a);
            a.click();
            a.remove();
            URL.revokeObjectURL(url);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            exporting = false;
        }
    }

    /** Soft-delete this record, then return to the list. */
    async function handleDelete() {
        await repo.remove(pid);
        await goto("/");
    }

    /**
     * Match this record against the registry and show potential
     * duplicates, excluding the record itself from the results.
     */
    async function handleCheckDuplicates() {
        if (!org) return;
        checking = true;
        try {
            const hits = await repo.checkDuplicates(org);
            // Drop the self-match; the record always matches itself.
            duplicates = excludeSelf(hits, pid);
        } catch (err) {
            error = err instanceof Error ? err.message : t("detail.checkFailed");
        } finally {
            checking = false;
        }
    }

    /** Toggle the audit panel; lazy-load the trail on first open. */
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
            auditError =
                err instanceof Error ? err.message : t("detail.auditLoadFailed");
        } finally {
            auditLoading = false;
        }
    }
</script>

<svelte:head><title>{org?.name ?? t("detail.organization")} — Main X</title></svelte:head>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if org}
    <h1>{org.name}</h1>
    {#if masked}
        <p class="banner" role="status">{t("detail.maskedNotice")}</p>
    {/if}
    <div class="surface stack">
        {#if org.legal_name}<div><strong>{t("detail.legalName")}</strong> {org.legal_name}</div>{/if}
        {#if org.url}<div><strong>{t("detail.url")}</strong> <a href={org.url}>{org.url}</a></div>{/if}
        {#if org.jurisdiction}<div><strong>{t("detail.jurisdiction")}</strong> {org.jurisdiction}</div>{/if}
        {#if org.founding_date}<div><strong>{t("detail.founded")}</strong> {org.founding_date}</div>{/if}
        {#if org.identifiers && org.identifiers.length > 0}
            <div>
                <strong>{t("detail.identifiers")}</strong>
                <ul>
                    <!-- Render bare-string schemes directly; unwrap the
                         `{ Custom: label }` variant as `Custom(label)`. -->
                    {#each org.identifiers as id, i (i)}
                        <li>{typeof id.scheme === "string" ? id.scheme : `Custom(${id.scheme.Custom})`}: <code>{id.value}</code></li>
                    {/each}
                </ul>
            </div>
        {/if}
        {#if org.keywords && org.keywords.length > 0}
            <div><strong>{t("detail.keywords")}</strong> {org.keywords.join(", ")}</div>
        {/if}
        <div><strong>{t("detail.id")}</strong> <code>{pid}</code></div>
    </div>

    <div class="row" style="margin-top:1rem">
        <button class="button" aria-pressed={masked} onclick={toggleMasked}>
            {masked ? t("detail.showFull") : t("detail.showMasked")}
        </button>
        <a class="button" href={`/${pid}/edit`}>{t("detail.edit")}</a>
        <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
            {checking ? t("detail.checking") : t("detail.checkDuplicates")}
        </button>
        <button
            class="button"
            onclick={handleExportGdpr}
            disabled={exporting}
        >
            {exporting ? t("detail.exportingGdpr") : t("detail.exportGdpr")}
        </button>
        <button onclick={handleDelete}>{t("detail.delete")}</button>
    </div>

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
