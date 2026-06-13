<script lang="ts">
    import { onMount } from "svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import type { AuditEntry } from "$lib/api/types.js";

    let healthStatus = $state<"ok" | "down" | "loading">("loading");
    let healthMessage = $state<string | null>(null);
    let recent = $state<AuditEntry[]>([]);
    let recentError = $state<string | null>(null);

    onMount(async () => {
        const repo = CourseRepository.withFetch();
        try {
            const h = await repo.health();
            // Service emits `{ status: "healthy", ... }`. Accept any
            // affirmative string ("healthy" / "ok" / "up") as healthy;
            // anything else surfaces as a degraded "down" so the badge
            // is honest when the service is reachable but reporting
            // trouble.
            const s = h.status?.toLowerCase() ?? "";
            healthStatus =
                s.includes("healthy") || s.includes("ok") || s.includes("up")
                    ? "ok"
                    : "down";
            if (healthStatus === "down") {
                healthMessage = `Service reports status: ${h.status ?? "(empty)"}`;
            }
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

<svelte:head><title>Dashboard · Course Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Dashboard</h1>
    <span class="status" data-status={healthStatus}>
        Service: {healthStatus}
    </span>
</header>

{#if healthMessage}
    <div class="banner error">{healthMessage}</div>
{/if}

<section class="surface stack">
    <h2>Recent activity</h2>
    {#if recentError}
        <div class="banner error">{recentError}</div>
    {:else if recent.length === 0}
        <p class="muted">No recent audit entries.</p>
    {:else}
        <ul class="audit">
            {#each recent as entry}
                <li>
                    <code>{entry.action}</code>
                    <a href={`/courses/${entry.entity_id}`}>{entry.entity_id.slice(0, 8)}…</a>
                    <span class="muted small">{new Date(entry.created_at).toLocaleString()}</span>
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
    .status[data-status="ok"] { background: #dcfce7; color: var(--mxi-color-success); }
    .status[data-status="down"] { background: #fee2e2; color: var(--mxi-color-danger); }
    .audit { list-style: none; padding: 0; margin: 0; }
    .audit li {
        display: flex; gap: 0.5rem; align-items: baseline;
        padding: 0.375rem 0;
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .audit li:last-child { border-bottom: none; }
</style>
