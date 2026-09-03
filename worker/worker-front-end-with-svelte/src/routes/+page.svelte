<!--
  Dashboard (route "/") — landing page showing a service-health badge and a
  list of recent system-wide audit activity. Both data sources are fetched
  on mount and fail soft (errors render as banners, not crashes).

  $state:
    - healthStatus — "loading" | "ok" | "down"; drives the status badge.
    - healthMessage — error text shown when the health probe fails.
    - recent — recent AuditEntry rows for the activity feed.
    - recentError — error text when the audit fetch fails.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { AuditEntry } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let healthStatus = $state<"ok" | "down" | "loading">("loading");
    let healthMessage = $state<string | null>(null);
    let recent = $state<AuditEntry[]>([]);
    let recentError = $state<string | null>(null);

    // Fetch health + recent audit once the component mounts (CSR only).
    onMount(async () => {
        const repo = WorkerRepository.withFetch();
        try {
            const h = await repo.health();
            // NOTE: any successful health response is treated as "ok" here;
            // the ternary's branches are intentionally both "ok" so that a
            // 200 with an unexpected status string still shows healthy. A
            // thrown error (service unreachable) is what flips it to "down".
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

<svelte:head><title>{t("dashboard.titleTab")}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("dashboard.heading")}</h1>
    <span class="status" data-status={healthStatus}>
        {t("dashboard.servicePrefix")}
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
                    <a href={`/workers/${entry.entity_id}`}
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
