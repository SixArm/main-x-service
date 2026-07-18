<!--
  Resource capacity (`/capacity`, PPM-8): the per-person rollup over
  a window; summed percent over 100 flags over-allocation.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { PpmClient, type CapacityView } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let view = $state<CapacityView | null>(null);
  let error = $state<string | null>(null);
  let from = $state("");
  let to = $state("");

  async function refresh() {
    try {
      view = await ppm.capacity(from || undefined, to || undefined);
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "load failed";
    }
  }
  onMount(refresh);
</script>

<svelte:head><title>Capacity — PPM</title></svelte:head>

<h1>Resource capacity</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form
  class="row"
  onsubmit={(e) => {
    e.preventDefault();
    refresh();
  }}
>
  <label class="small">from <input type="date" bind:value={from} /></label>
  <label class="small">to <input type="date" bind:value={to} /></label>
  <button class="button small" type="submit">Apply</button>
</form>

{#if view}
  <p class="small muted">window: {view.from} → {view.to}</p>
  <table>
    <thead><tr><th>Person</th><th>Allocated</th><th>Load</th><th>Allocations</th></tr></thead>
    <tbody>
      {#each view.people as person (person.person_ref)}
        <tr>
          <td class="small">{person.person_ref}</td>
          <td class:over={person.over_allocated}>
            {person.allocated_percent}%
            {#if person.over_allocated}<span class="chip red">over</span>{/if}
          </td>
          <td>
            <span class="meter">
              <span
                class="fill"
                class:over={person.over_allocated}
                style={`width:${Math.min(person.allocated_percent, 150) / 1.5}%`}
              ></span>
            </span>
          </td>
          <td>{person.allocations}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if view.people.length === 0}
    <p class="small muted">No allocations yet — add them on an item's governance panel.</p>
  {/if}
{/if}

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .over { color: #a4262c; font-weight: 700; }
  .chip.red {
    display: inline-block;
    border: 1px solid #a4262c;
    color: #a4262c;
    border-radius: 999px;
    padding: 0 0.5rem;
    font-size: 0.75rem;
  }
  .meter {
    display: inline-block;
    width: 160px;
    height: 0.7rem;
    background: var(--surface-2, #f0f0f0);
    border-radius: 4px;
    overflow: hidden;
  }
  .fill { display: block; height: 100%; background: #1d8a4e; }
  .fill.over { background: #a4262c; }
</style>
