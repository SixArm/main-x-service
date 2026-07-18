<script lang="ts">
  import { getLead, listLeads } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Lead, ScoreBreakdown } from "$lib/api/crm";

  let leads = $state<Lead[] | null>(null);
  let breakdown = $state<{ pid: string; score: ScoreBreakdown } | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        leads = await listLeads();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  async function explain(lead: Lead) {
    const detail = await getLead(lead.pid);
    breakdown = { pid: lead.pid, score: detail.score };
  }
</script>

<h1>{t("nav.leads")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="lead-queue">
    <thead>
      <tr>
        <th>{t("lead.score")}</th>
        <th>{t("common.name")}</th>
        <th>{t("lead.source")}</th>
        <th>{t("common.status")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each leads as lead (lead.pid)}
        <tr>
          <td><strong>{lead.score}</strong></td>
          <td>{lead.display_name}</td>
          <td>{lead.source}</td>
          <td><span class="chip">{lead.status}</span></td>
          <td><button onclick={() => void explain(lead)}>{t("lead.breakdown")}</button></td>
        </tr>
        {#if breakdown?.pid === lead.pid}
          <tr data-testid="breakdown">
            <td colspan="5">
              <strong>{breakdown.score.score} · {breakdown.score.label}</strong>
              {#each breakdown.score.rules as rule (rule.rule)}
                <span class="chip">{rule.rule}: {rule.points}</span>
              {/each}
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}
