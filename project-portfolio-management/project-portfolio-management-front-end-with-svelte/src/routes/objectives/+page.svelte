<!--
  OKR objectives (`/objectives`, PPM-5): the registry plus per-
  objective alignment rollups (which items serve it, with weights).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { PpmClient, type Alignment, type Objective } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let objectives = $state<Objective[]>([]);
  let alignments = $state<Record<string, Alignment>>({});
  let error = $state<string | null>(null);
  let title = $state("");
  let period = $state("");

  async function refresh() {
    try {
      objectives = await ppm.listObjectives();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "load failed";
    }
  }
  onMount(refresh);

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      await ppm.createObjective({ title, period: period || undefined });
      title = "";
      period = "";
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : "create failed";
    }
  }

  async function showAlignment(pid: string) {
    try {
      alignments = { ...alignments, [pid]: await ppm.alignment(pid) };
    } catch (err) {
      error = err instanceof Error ? err.message : "alignment failed";
    }
  }
</script>

<svelte:head><title>Objectives — PPM</title></svelte:head>

<h1>Objectives (OKRs)</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={create}>
  <input placeholder="Objective title" bind:value={title} required />
  <input placeholder="Period (e.g. 2026-H2)" bind:value={period} size="12" />
  <button class="button primary" type="submit">Register</button>
</form>

<ul class="objectives">
  {#each objectives as objective (objective.pid)}
    <li>
      <div>
        <strong>{objective.title}</strong>
        {#if objective.period}<span class="chip">{objective.period}</span>{/if}
        <button class="button small" onclick={() => showAlignment(objective.pid)}>Alignment</button>
      </div>
      {#if alignments[objective.pid]}
        {@const alignment = alignments[objective.pid]}
        {#if alignment}
          <p class="small">
            total weight <strong>{alignment.total_weight}</strong>
            {#each Object.entries(alignment.weight_by_collection) as [kind, weight] (kind)}
              <span class="chip">{kind}: {weight}</span>
            {/each}
          </p>
          <ul class="small">
            {#each alignment.items as item (item.pid)}
              <li>
                <a href={`/${item.kind.toLowerCase()}s/${item.pid}/governance`}>{item.name}</a>
                (w{item.weight})
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </li>
  {/each}
</ul>
<p class="small muted">Map items to objectives from each item's governance panel.</p>

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .objectives { list-style: none; padding: 0; }
  .objectives > li { padding: 0.5rem 0; border-bottom: 1px solid var(--border, #ddd); }
  .chip {
    display: inline-block;
    border: 1px solid var(--border, #ccc);
    border-radius: 999px;
    padding: 0 0.5rem;
    margin: 0 0.2rem;
    font-size: 0.78rem;
  }
</style>
