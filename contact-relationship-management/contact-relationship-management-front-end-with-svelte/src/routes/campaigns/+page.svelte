<script lang="ts">
  import { campaignFunnel, campaignStatus, listCampaigns, money, runCampaign } from "$lib/api/crm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Campaign, Ratio } from "$lib/api/crm";

  let campaigns = $state<Campaign[] | null>(null);
  let funnel = $state<{ pid: string; leads: number; won_revenue_minor: number; roi: Ratio } | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      campaigns = await listCampaigns();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function run(campaign: Campaign) {
    actionError = null;
    try {
      if (campaign.status === "draft") {
        await campaignStatus(campaign.pid, "scheduled");
      }
      await runCampaign(campaign.pid);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function showFunnel(campaign: Campaign) {
    const data = await campaignFunnel(campaign.pid);
    funnel = {
      pid: campaign.pid,
      leads: data.leads,
      won_revenue_minor: data.won_revenue_minor,
      roi: data.roi,
    };
  }

  function roiLabel(roi: Ratio): string {
    return roi.value === null ? "—" : `${Math.round(roi.value * 100)}%`;
  }
</script>

<h1>{t("nav.campaigns")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if campaigns === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <table data-testid="campaigns">
    <thead>
      <tr>
        <th>{t("common.name")}</th>
        <th>{t("common.status")}</th>
        <th>{t("campaign.recipients")}</th>
        <th>{t("common.actions")}</th>
      </tr>
    </thead>
    <tbody>
      {#each campaigns as campaign (campaign.pid)}
        <tr>
          <td>{campaign.name}</td>
          <td><span class="chip">{campaign.status}</span></td>
          <td>
            {campaign.recipients} → {campaign.delivered} → {campaign.opened} → {campaign.clicked}
          </td>
          <td>
            {#if campaign.status === "draft" || campaign.status === "scheduled"}
              <button onclick={() => void run(campaign)}>{t("campaign.run")}</button>
            {/if}
            <button onclick={() => void showFunnel(campaign)}>{t("campaign.funnel")}</button>
          </td>
        </tr>
        {#if funnel?.pid === campaign.pid}
          <tr data-testid="funnel">
            <td colspan="4">
              {t("nav.leads")}: {funnel.leads} ·
              {money(funnel.won_revenue_minor, campaign.currency, i18n.locale)} ·
              {t("campaign.roi")}: <strong>{roiLabel(funnel.roi)}</strong>
              <span class="muted">
                ({funnel.roi.numerator} / {funnel.roi.denominator})
              </span>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}
