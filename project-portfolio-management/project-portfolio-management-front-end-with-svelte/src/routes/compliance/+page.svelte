<!--
  Compliance area (`/compliance`): the compliance risk register and
  the derived conformance findings (a findings list with each rule
  disclosed — no invented compliance score). English-first, like the
  other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    type ConformanceFindings,
    type RiskRegister,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let register = $state<RiskRegister | null>(null);
  let findings = $state<ConformanceFindings | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      register = await ppm.complianceRegister();
      findings = await ppm.complianceFindings();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>{t("ppm.nav.compliance")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.compliance")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if findings}
  <h2>Conformance findings</h2>
  <p class="muted">gate-review recency threshold: {findings.review_days} days</p>
  <table data-testid="compliance-findings">
    <thead><tr><th>Rule</th><th>Detail</th></tr></thead>
    <tbody>
      {#each findings.findings as finding, index (index)}
        <tr><td><code>{finding.rule}</code></td><td>{finding.detail}</td></tr>
      {:else}
        <tr><td colspan="2" class="muted">No conformance findings.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if register}
  <h2>Compliance risk register</h2>
  <p class="muted">{register.note} · open exposure {register.open_exposure}</p>
  <table data-testid="compliance-register">
    <thead>
      <tr><th>Risk</th><th>Item</th><th>Status</th><th>Exposure</th><th>Owner</th></tr>
    </thead>
    <tbody>
      {#each register.register as row (row.pid)}
        <tr>
          <td>{row.title}{#if row.escalated}&nbsp;⚠{/if}</td>
          <td>{row.item?.name ?? "—"}</td>
          <td>{row.status}</td>
          <td>{row.exposure}</td>
          <td>{row.owner_ref ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">No compliance risks recorded.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
