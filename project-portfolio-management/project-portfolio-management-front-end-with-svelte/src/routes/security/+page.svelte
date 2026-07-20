<!--
  CISO area (`/security`): the security risk register plus the
  server's disclosed heuristic — items at a late stage with no
  security-category risk ever recorded (a proxy, not proof).
  English-first, like the other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, type RiskRegister } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let register = $state<RiskRegister | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      register = await ppm.securityRegister();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>{t("ppm.nav.security")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.security")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if register}
  <h2>Security risk register</h2>
  <p class="muted">{register.note} · open exposure {register.open_exposure}</p>
  <table data-testid="security-register">
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
        <tr><td colspan="5" class="muted">No security risks recorded.</td></tr>
      {/each}
    </tbody>
  </table>

  {#if register.unreviewed_at_late_stage}
    <h2>Late-stage items with no security risk recorded</h2>
    <p class="muted">{register.unreviewed_at_late_stage.heuristic}</p>
    <ul data-testid="security-unreviewed">
      {#each register.unreviewed_at_late_stage.items as row (row.item.pid)}
        <li>{row.item.name} <span class="muted">({row.stage})</span></li>
      {:else}
        <li class="muted">None — every late-stage item has a recorded security risk.</li>
      {/each}
    </ul>
  {/if}
{/if}
