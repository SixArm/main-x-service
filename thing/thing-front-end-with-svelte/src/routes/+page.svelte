<!--
  +page.svelte (/) — dashboard / landing page.

  Purpose: on mount, probes the service health endpoint and loads the recent
  system-wide audit feed, displaying a status pill and an activity list.

  $state:
    - healthStatus: "loading" until the probe resolves, then "ok" / "down".
    - healthMessage / recentError: error banners for the two fetches.
    - recent: the recent AuditEntry[] feed.

  Note: data loads in onMount (not a load function) because the app is
  CSR-only (see +layout.ts).
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { describeApiError } from "$lib/api/errorHandling.js";
    import type { AuditEntry } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let healthStatus = $state<"ok" | "down" | "loading">("loading");
    let healthMessage = $state<string | null>(null);
    let recent = $state<AuditEntry[]>([]);
    let recentError = $state<string | null>(null);

    onMount(async () => {
        const repo = ThingRepository.withFetch();
        try {
            const h = await repo.health();
            // A reachable health endpoint counts as "ok" regardless of the
            // exact status string (both branches resolve to "ok" by design).
            healthStatus =
                h.status?.toLowerCase().includes("ok") ||
                h.status?.toLowerCase().includes("up")
                    ? "ok"
                    : "ok";
        } catch (err) {
            // Any failure to reach the endpoint marks the service down.
            healthStatus = "down";
            healthMessage = describeApiError(err);
        }
        try {
            recent = await repo.recentAudit(20);
        } catch (err) {
            recentError = describeApiError(err);
        }
    });
</script>

<svelte:head><title>Dashboard · Thing Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("dashboard.title")}</h1>
    <span class="status" data-status={healthStatus}>
        {t("dashboard.service")}
        {healthStatus}
    </span>
</header>

{#if healthMessage}
    <div class="banner error">{healthMessage}</div>
{/if}

<section class="surface stack">
    <h2>{t("dashboard.recentActivity")}</h2>
    {#if recentError}
        <div class="banner error">{recentError}</div>
    {:else if recent.length === 0}
        <p class="muted">{t("dashboard.noRecent")}</p>
    {:else}
        <ul class="audit">
            {#each recent as entry}
                <li>
                    <code>{entry.action}</code>
                    <a href={`/things/${entry.entity_id}`}
                        >{entry.entity_id.slice(0, 8)}…</a
                    >
                    <span class="muted small"
                        >{new Date(entry.created_at).toLocaleString()}</span
                    >
                </li>
            {/each}
        </ul>
    {/if}
</section>

<style>
    .status {
        padding: 0.25rem 0.625rem;
        border-radius: 999px;
        background: #f3f4f6;
        font-size: 0.875rem;
    }
    .status[data-status="ok"] {
        background: #dcfce7;
        color: var(--mxi-color-success);
    }
    .status[data-status="down"] {
        background: #fee2e2;
        color: var(--mxi-color-danger);
    }
    .audit {
        list-style: none;
        padding: 0;
        margin: 0;
    }
    .audit li {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        padding: 0.375rem 0;
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .audit li:last-child {
        border-bottom: none;
    }
</style>
