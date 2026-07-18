<script lang="ts">
  import { listContacts } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Contact } from "$lib/api/crm";

  let contacts = $state<Contact[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        contacts = await listContacts();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("nav.contacts")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if contacts === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="contact-table">
    <thead>
      <tr>
        <th>{t("common.name")}</th>
        <th>{t("common.status")}</th>
        <th>{t("contact.consent")}</th>
      </tr>
    </thead>
    <tbody>
      {#each contacts as contact (contact.pid)}
        <tr>
          <td><a href={`/contacts/${contact.pid}`}>{contact.display_name}</a></td>
          <td><span class="chip">{contact.status}</span></td>
          <td><span class={`chip consent-${contact.marketing_consent}`}>{contact.marketing_consent}</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .consent-granted {
    color: var(--state-available, #1d8a4e);
  }
  .consent-withdrawn {
    color: var(--state-closed, #8a1d2d);
  }
</style>
