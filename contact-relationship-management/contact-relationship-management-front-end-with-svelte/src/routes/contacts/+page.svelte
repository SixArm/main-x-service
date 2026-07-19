<script lang="ts">
  import { goto } from "$app/navigation";
  import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
  import {
    FilterBar,
    Willow as FilterTheme,
    createArrayFilter,
  } from "@svar-ui/svelte-filter";
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

  const columns = $derived([
    { id: "display_name", header: t("common.name"), flexgrow: 1 },
    { id: "job_title", header: t("contact.jobTitle"), width: 150 },
    { id: "status", header: t("common.status"), width: 110 },
    { id: "marketing_consent", header: t("contact.consent"), width: 150 },
  ]);

  const rows = $derived(
    (contacts ?? []).map((c) => ({
      id: c.pid,
      display_name: c.display_name,
      job_title: c.job_title ?? "",
      status: c.status,
      marketing_consent: c.marketing_consent,
    })),
  );

  const filterFields = $derived([
    { id: "display_name", label: t("common.name"), type: "text" },
    { id: "status", label: t("common.status"), type: "text" },
    { id: "marketing_consent", label: t("contact.consent"), type: "text" },
  ]);
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );

  // Row selection opens the contact timeline.
  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      void goto(`/contacts/${ev.id}`);
    });
  }
</script>

<h1>{t("nav.contacts")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if contacts === null}
  <p>{t("common.loading")}</p>
{:else}
  <div data-testid="contact-table">
    <GridTheme>
      <FilterTheme>
        <div class="filter-wrap">
          <FilterBar
            fields={filterFields}
            onchange={({ value }: { value: unknown }) => (filterRules = value)}
          />
        </div>
        <div class="grid-wrap">
          <Grid data={filtered} {columns} select init={initGrid} />
        </div>
      </FilterTheme>
    </GridTheme>
  </div>
{/if}

<style>
  .filter-wrap {
    margin-bottom: 0.5rem;
  }
  .grid-wrap {
    height: 480px;
    overflow: hidden;
  }
</style>
