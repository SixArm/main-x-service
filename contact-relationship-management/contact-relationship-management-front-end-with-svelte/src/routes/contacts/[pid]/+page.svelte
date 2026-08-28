<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { eraseContact, getContact, money, recordConsent } from "$lib/api/crm";
  import { i18n, t } from "$lib/i18n.svelte";

  let detail = $state<Awaited<ReturnType<typeof getContact>> | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let erasing = $state(false);

  const pid = $derived(page.params.pid ?? "");
  // Rough client-side gate from data already on the page — the
  // service is the authority (CRM-R20 also checks nurture
  // enrolments, which this page does not load); this only avoids
  // offering the button when it would obviously be refused.
  const hasOpenEngagement = $derived(
    (detail?.deals.some((deal) => deal.closed_at === null) ?? false) ||
      (detail?.tickets.some((ticket) => ticket.status === "open" || ticket.status === "pending") ??
        false),
  );

  async function load() {
    try {
      detail = await getContact(pid);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    if (pid) void load();
  });

  async function consent(action: "granted" | "withdrawn") {
    actionError = null;
    try {
      await recordConsent(pid, action);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function erase() {
    if (!confirm(t("contact.eraseConfirm"))) return;
    actionError = null;
    erasing = true;
    try {
      await eraseContact(pid);
      await goto("/contacts");
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      erasing = false;
    }
  }
</script>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if detail === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1>{detail.contact.display_name}</h1>
  <div class="panel">
    <p>
      {detail.contact.job_title ?? ""} ·
      <span class="chip">{detail.contact.status}</span> ·
      {t("contact.consent")}:
      <strong data-testid="consent">{detail.contact.marketing_consent}</strong>
    </p>
    {#if detail.contact.marketing_consent === "granted"}
      <button onclick={() => void consent("withdrawn")}>{t("contact.withdraw")}</button>
    {:else}
      <button onclick={() => void consent("granted")}>{t("contact.grant")}</button>
    {/if}
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
  </div>

  <div class="panel">
    <a
      href={`/api/proxy/contacts/${pid}/subject-access`}
      target="_blank"
      rel="noreferrer"
      data-testid="subject-access"
    >
      {t("contact.subjectAccess")}
    </a>
    {#if !hasOpenEngagement}
      <button onclick={() => void erase()} disabled={erasing} data-testid="erase">
        {t("contact.erase")}
      </button>
    {/if}
  </div>

  <h2>{t("contact.timeline")}</h2>
  <table data-testid="timeline">
    <tbody>
      {#each detail.activities as activity (activity.pid)}
        <tr>
          <td>{new Date(activity.occurred_at).toLocaleString()}</td>
          <td><span class="chip">{activity.kind}</span></td>
          <td>{activity.summary}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("nav.deals")}</h2>
  <table>
    <tbody>
      {#each detail.deals as deal (deal.pid)}
        <tr>
          <td>{deal.name}</td>
          <td>{money(deal.amount_minor, deal.currency, i18n.locale)}</td>
          <td>
            {#if deal.closed_at}
              <span class="chip">{deal.won ? t("deal.won") : t("deal.lost")}</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("nav.tickets")}</h2>
  <table>
    <tbody>
      {#each detail.tickets as ticket (ticket.pid)}
        <tr>
          <td>{ticket.title}</td>
          <td><span class="chip">{ticket.status}</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
