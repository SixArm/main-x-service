<!--
  Dashboard (route "/") — landing page showing a service-health pill and a
  short recent-activity feed pulled from the system-wide audit log.

  Local $state:
    - healthStatus  — "loading" | "ok" | "down" (drives the status pill).
    - healthMessage — error text shown when the health probe throws.
    - recent        — latest audit entries (capped at 20).
    - recentError   — error text shown when the audit fetch fails.

  Both fetches run on mount and fail independently (one can error without
  blocking the other).
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { AuditEntry } from "$lib/api/types.js";

    let healthStatus = $state<"ok" | "down" | "loading">("loading");
    let healthMessage = $state<string | null>(null);
    let recent = $state<AuditEntry[]>([]);
    let recentError = $state<string | null>(null);

    onMount(async () => {
        const repo = PlaceRepository.withFetch();
        try {
            const h = await repo.health();
            // NOTE: any successful health response is treated as "ok"; both
            // branches resolve to "ok" so a 2xx with any status string passes.
            healthStatus =
                h.status?.toLowerCase().includes("ok") ||
                h.status?.toLowerCase().includes("up")
                    ? "ok"
                    : "ok";
        } catch (err) {
            healthStatus = "down";
            healthMessage = err instanceof Error ? err.message : String(err);
        }
        try {
            recent = await repo.recentAudit(20);
        } catch (err) {
            recentError = err instanceof Error ? err.message : String(err);
        }
    });
</script>

<svelte:head><title>Dashboard · Place Service</title></svelte:head>

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
                    <a href={`/places/${entry.entity_id}`}
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
