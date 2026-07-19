<script lang="ts">
  import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
  import {
    FilterBar,
    Willow as FilterTheme,
    createArrayFilter,
  } from "@svar-ui/svelte-filter";
  import { listAccounts } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Account } from "$lib/api/crm";

  let accounts = $state<Account[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        accounts = await listAccounts();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const columns = $derived([
    { id: "display_name", header: t("common.name"), flexgrow: 1 },
    { id: "tier", header: t("account.tier"), width: 130 },
    { id: "industry", header: t("account.industry"), width: 170 },
    { id: "pid", header: "pid", width: 300 },
  ]);

  const rows = $derived(
    (accounts ?? []).map((a) => ({
      id: a.pid,
      display_name: a.display_name,
      tier: a.tier,
      industry: a.industry ?? "",
      pid: a.pid,
    })),
  );

  const filterFields = $derived([
    { id: "display_name", label: t("common.name"), type: "text" },
    { id: "tier", label: t("account.tier"), type: "text" },
    { id: "industry", label: t("account.industry"), type: "text" },
  ]);
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );
</script>

<h1>{t("nav.accounts")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if accounts === null}
  <p>{t("common.loading")}</p>
{:else}
  <div data-testid="account-table">
    <GridTheme>
      <FilterTheme>
        <div class="filter-wrap">
          <FilterBar
            fields={filterFields}
            onchange={({ value }: { value: unknown }) => (filterRules = value)}
          />
        </div>
        <div class="grid-wrap">
          <Grid data={filtered} {columns} />
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
