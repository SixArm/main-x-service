<!--
  DPO area (`/dpo`): consent coverage (each contact's current state
  counted verbatim), withdrawals in the window, consent events by
  source, and duplicate-contact hygiene (CRM-local rows sharing one
  person URN — the server's note states identity dedup stays
  upstream). Also the SLA register lives with support, not here.
-->
<script lang="ts">
  import { consentByAccount, dpo } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  type DpoView = Awaited<ReturnType<typeof dpo>>;
  type ByAccount = Awaited<ReturnType<typeof consentByAccount>>;
  let view = $state<DpoView | null>(null);
  let byAccount = $state<ByAccount | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        view = await dpo();
        byAccount = await consentByAccount();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<svelte:head><title>{t("nav.dpo")} — CRM</title></svelte:head>

<h1>{t("nav.dpo")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if view}
  <p class="muted">{view.note}</p>
  <section class="tiles" data-testid="dpo-tiles">
    <div class="tile"><strong>{view.contacts}</strong><span>contacts</span></div>
    {#each Object.entries(view.consent_coverage) as [state, count] (state)}
      <div class="tile"><strong>{count}</strong><span>consent: {state}</span></div>
    {/each}
    <div class="tile">
      <strong>{view.withdrawals_in_window}</strong>
      <span>withdrawals ({view.window_days}d)</span>
    </div>
  </section>

  <h2>Consent events by source</h2>
  <table data-testid="dpo-sources">
    <thead><tr><th>Source</th><th>Events</th></tr></thead>
    <tbody>
      {#each Object.entries(view.consent_events_by_source) as [source, count] (source)}
        <tr><td>{source}</td><td>{count}</td></tr>
      {:else}
        <tr><td colspan="2" class="muted">No consent events recorded.</td></tr>
      {/each}
    </tbody>
  </table>

  <h2>Consent by account</h2>
  {#if byAccount}
    <table data-testid="dpo-by-account">
      <thead><tr><th>Account</th><th>Coverage</th><th>Withdrawals ({byAccount.window_days}d)</th></tr></thead>
      <tbody>
        {#each byAccount.accounts as row (row.pid)}
          <tr>
            <td>{row.display_name}</td>
            <td>
              {#each Object.entries(row.consent_coverage) as [state, count] (state)}
                {state}: {count}&nbsp;
              {:else}
                <span class="muted">no contacts</span>
              {/each}
            </td>
            <td>{row.withdrawals_in_window}</td>
          </tr>
        {:else}
          <tr><td colspan="3" class="muted">No accounts.</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h2>Duplicate contact rows</h2>
  <table data-testid="dpo-duplicates">
    <thead><tr><th>Person URN</th><th>Contacts</th></tr></thead>
    <tbody>
      {#each view.duplicate_person_refs as group (group.person_ref)}
        <tr>
          <td><code>{group.person_ref}</code></td>
          <td>
            {#each group.contacts as contact (contact.pid)}
              {contact.display_name}&nbsp;
            {/each}
          </td>
        </tr>
      {:else}
        <tr><td colspan="2" class="muted">No duplicates — one row per person.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
