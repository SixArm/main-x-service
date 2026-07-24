<script lang="ts">
  import type { OrgNode } from "$lib/api/types";
  import OrgTree from "./OrgTree.svelte";

  let { node }: { node: OrgNode } = $props();
</script>

<div class="node">
  <a href={`/employees/${node.pid}`}>{node.display_name}</a>
  <span class="muted">— {node.job_title} · {node.department}</span>
  {#if node.reports.length}
    <div class="reports">
      {#each node.reports as report (report.pid)}
        <OrgTree node={report} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .reports {
    margin-inline-start: 1.25rem;
    border-inline-start: 2px solid var(--line);
    padding-inline-start: 0.75rem;
  }
  .node {
    padding: 0.15rem 0;
  }
</style>
