<script lang="ts">
  import { listEmployees, listRequisitions, successionGaps } from "$lib/api/hcm";
  import { t } from "$lib/i18n.svelte";
  import type { Employee, Requisition } from "$lib/api/types";

  let active = $state<Employee[] | null>(null);
  let open = $state<Requisition[] | null>(null);
  let gapCount = $state<number | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const [employees, requisitions, gaps] = await Promise.all([
          listEmployees({ status: "active" }),
          listRequisitions("open"),
          successionGaps(),
        ]);
        active = employees;
        open = requisitions;
        gapCount = gaps.gaps.length;
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("dash.title")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if active === null}
  <p>{t("common.loading")}</p>
{:else}
  <div class="tiles">
    <a class="tile" href="/employees" data-testid="tile-active">
      <strong>{active.length}</strong>
      <span>{t("dash.activeEmployees")}</span>
    </a>
    <a class="tile" href="/requisitions" data-testid="tile-open">
      <strong>{open?.length ?? 0}</strong>
      <span>{t("dash.openRequisitions")}</span>
    </a>
    <a class="tile" href="/development" data-testid="tile-gaps">
      <strong>{gapCount ?? 0}</strong>
      <span>{t("dash.successionGaps")}</span>
    </a>
  </div>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 1rem;
  }
  .tile {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: inherit;
  }
  .tile strong {
    font-size: 2rem;
  }
</style>
