<script lang="ts">
  import { listEmployees, orgChart } from "$lib/api/wpm";
  import { t } from "$lib/i18n.svelte";
  import type { OrgNode } from "$lib/api/types";
  import OrgTree from "$lib/components/OrgTree.svelte";

  let roots = $state<OrgNode[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        // The demo serves one organization: derive it from any employee.
        const employees = await listEmployees();
        const organization = employees[0]?.organization_ref;
        roots = organization ? await orgChart(organization) : [];
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("nav.orgChart")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if roots === null}
  <p>{t("common.loading")}</p>
{:else}
  <div class="panel" data-testid="org-chart">
    {#each roots as root (root.pid)}
      <OrgTree node={root} />
    {/each}
  </div>
{/if}
