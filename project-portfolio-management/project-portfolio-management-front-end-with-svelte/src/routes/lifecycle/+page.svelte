<!--
  Bird's-eye visibility (`/lifecycle`): the whole challenge lifecycle in
  one view — every phase from idea to closed, how much is live in each,
  and how much has stalled there past the configured threshold.

  Every phase is shown even at zero, so an empty stage is visible rather
  than absent, and items whose phase could not be resolved are counted
  separately instead of being quietly folded in somewhere.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CapabilityClient, type LifecycleFunnel } from "$lib/api/capabilities";

  const api = CapabilityClient.withFetch();
  let view = $state<LifecycleFunnel | null>(null);
  let error = $state<string | null>(null);

  const total = $derived(
    view ? view.phases.reduce((sum, phase) => sum + phase.live, 0) : 0,
  );

  async function refresh() {
    try {
      view = await api.lifecycle();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load the funnel";
    }
  }
  onMount(refresh);
</script>

<svelte:head><title>Lifecycle — PPM</title></svelte:head>

<h1>Lifecycle</h1>
<p class="small muted">
  Where everything sits across the challenge lifecycle, so the next phase
  is never a surprise. Stalled counts items that have sat in a phase
  longer than the threshold.
</p>

{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if view}
  <p class="small muted">
    {total} live item(s) · stall threshold {view.stall_days} day(s)
    {#if view.unknown_phase > 0}
      · <strong>{view.unknown_phase}</strong> in an unrecognised phase
    {/if}
  </p>
  <table>
    <thead><tr><th>Phase</th><th>Live</th><th>Stalled</th><th>Share</th></tr></thead>
    <tbody>
      {#each view.phases as phase (phase.phase)}
        <tr>
          <td>{phase.phase.replaceAll("_", " ")}</td>
          <td>{phase.live}</td>
          <td class:stalled={phase.stalled > 0}>{phase.stalled}</td>
          <td>
            <span class="meter">
              <span
                class="fill"
                style={`width:${total === 0 ? 0 : (phase.live / total) * 100}%`}
              ></span>
            </span>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="small muted">as of {view.as_of}</p>
{/if}

<style>
  .stalled { color: #a4262c; font-weight: 700; }
  .meter {
    display: inline-block;
    width: 200px;
    height: 0.7rem;
    background: var(--surface-2, #f0f0f0);
    border-radius: 4px;
    overflow: hidden;
  }
  .fill { display: block; height: 100%; background: #1d8a4e; }
</style>
