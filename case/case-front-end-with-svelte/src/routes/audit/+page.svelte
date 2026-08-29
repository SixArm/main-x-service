<!--
  Recent activity route (`/audit`) — system-wide, across every case:
  the recent audit-log entries (`GET /api/cases/audit/recent`, cap 100)
  and the recent CRUD/merge event stream (`GET /api/cases/events/recent`,
  cap 100). Each panel loads independently so one failing does not block
  the other.

  State:
    - audit / auditLoading / auditError — the recent-audit panel.
    - events / eventsLoading / eventsError — the recent-events panel.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { AuditEntry, CaseEvent } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();

  let audit = $state<AuditEntry[]>([]);
  let auditLoading = $state(true);
  let auditError = $state<string | null>(null);

  let events = $state<CaseEvent[]>([]);
  let eventsLoading = $state(true);
  let eventsError = $state<string | null>(null);

  onMount(() => {
    void (async () => {
      try {
        audit = await repo.recentAudit();
      } catch (err) {
        auditError = err instanceof Error ? err.message : t("activity.loadFailed");
      } finally {
        auditLoading = false;
      }
    })();
    void (async () => {
      try {
        events = await repo.recentEvents();
      } catch (err) {
        eventsError = err instanceof Error ? err.message : t("activity.loadFailed");
      } finally {
        eventsLoading = false;
      }
    })();
  });
</script>

<svelte:head><title>{t("activity.title")} — Main X</title></svelte:head>

<h1>{t("activity.title")}</h1>

<h2>{t("activity.recentAudit")}</h2>
<section class="surface stack" style="margin-bottom:1rem">
  {#if auditLoading}
    <p>{t("activity.loading")}</p>
  {:else if auditError}
    <p class="banner" role="alert">{auditError}</p>
  {:else if audit.length === 0}
    <p class="muted">{t("activity.noAuditEntries")}</p>
  {:else}
    <ul class="stack">
      {#each audit as entry (entry.id)}
        <li class="surface row">
          <code>{entry.action}</code>
          <a href={`/${entry.entity_pid}`}><code>{entry.entity_pid}</code></a>
          <span class="small muted">{new Date(entry.created_at).toLocaleString()}</span>
          {#if entry.actor}<span class="small muted">{t("audit.by")} {entry.actor}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<h2>{t("activity.recentEvents")}</h2>
<section class="surface stack">
  {#if eventsLoading}
    <p>{t("activity.loading")}</p>
  {:else if eventsError}
    <p class="banner" role="alert">{eventsError}</p>
  {:else if events.length === 0}
    <p class="muted">{t("activity.noEvents")}</p>
  {:else}
    <ul class="stack">
      {#each events as event (event.seq)}
        <li class="surface row">
          <code>{event.kind}</code>
          <a href={`/${event.pid}`}>{event.name}</a>
          <span class="small muted">#{event.seq}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
