<!--
  Case audit trail route (`/[pid]/audit`).

  Purpose: load and render one case's audit-log rows
  (`GET /api/cases/{pid}/audit`), newest first (the service already
  orders this way).

  State:
    - entries / loading / error — fetch result and status.
-->
<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { AuditEntry } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();
  const pid = page.params.pid ?? "";

  let entries = $state<AuditEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      entries = await repo.audit(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("audit.loadFailed");
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head><title>{t("audit.title")} — Main X</title></svelte:head>

<header class="row" style="justify-content: space-between">
  <h1>{t("audit.title")}</h1>
  <a href={`/${pid}`} class="button">{t("audit.backToCase")}</a>
</header>

<section class="surface stack">
  {#if loading}
    <p>{t("audit.loading")}</p>
  {:else if error}
    <p class="banner" role="alert">{error}</p>
  {:else if entries.length === 0}
    <p class="muted">{t("audit.noEntries")}</p>
  {:else}
    <ul class="stack">
      {#each entries as entry (entry.id)}
        <li class="surface">
          <div class="row">
            <code>{entry.action}</code>
            <span class="small muted">{new Date(entry.created_at).toLocaleString()}</span>
            {#if entry.actor}<span class="small muted">{t("audit.by")} {entry.actor}</span>{/if}
          </div>
          {#if entry.snapshot}
            <details>
              <summary class="small">{t("audit.payload")}</summary>
              <pre class="small">{JSON.stringify(entry.snapshot, null, 2)}</pre>
            </details>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  pre {
    background: var(--mxi-color-bg);
    padding: 0.5rem;
    border-radius: var(--mxi-radius);
    overflow-x: auto;
  }
</style>
