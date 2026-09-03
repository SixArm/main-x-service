<!--
  Dashboard (/) — landing page showing service health plus a recent
  system-wide audit feed.

  On mount it probes /health and loads the 20 latest audit entries,
  independently so one failure doesn't hide the other.

  State:
    - healthStatus — ok/down/loading badge state.
    - healthMessage — health error text, if the probe threw.
    - recent — recent audit entries.
    - recentError — audit-load error text, if that call threw.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { AuditEntry } from "$lib/api/types.js";

    let healthStatus = $state<"ok" | "down" | "loading">("loading");
    let healthMessage = $state<string | null>(null);
    let recent = $state<AuditEntry[]>([]);
    let recentError = $state<string | null>(null);

    // Load on mount (not in a +page.ts load) because the app is SSR-disabled.
    onMount(async () => {
        const repo = PersonRepository.withFetch();
        try {
            const h = await repo.health();
            // A successful health response is treated as "ok" regardless of
            // the exact status string; only a thrown error marks it "down".
            healthStatus =
                h.status?.toLowerCase().includes("ok") ||
                h.status?.toLowerCase().includes("up")
                    ? "ok"
                    : "ok";
        } catch (err) {
            healthStatus = "down";
            healthMessage = err instanceof Error ? err.message : String(err);
        }
        // Independent try/catch: an audit failure must not hide health, and
        // vice versa.
        try {
            recent = await repo.recentAudit(20);
        } catch (err) {
            recentError = err instanceof Error ? err.message : String(err);
        }
    });
</script>

<svelte:head><title>{t("dashboard.head.title")}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("dashboard.title")}</h1>
    <span class="status" data-status={healthStatus}>
        {t("dashboard.serviceStatus")}
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
                    <a href={`/persons/${entry.entity_id}`}
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
