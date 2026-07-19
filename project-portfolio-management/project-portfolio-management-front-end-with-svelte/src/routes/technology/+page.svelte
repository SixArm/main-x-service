<!--
  CTO technology area (`/technology`): the dependency-risk lens (top
  fan-out items, cross-portfolio edges, edges with a RAG-red
  predecessor) and the tag-encoded technology radar
  (`tech:<name>[:<ring>]`; the server's convention string is displayed
  verbatim). All derivations are server-side. English-first, like the
  other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    type DependencyRisk,
    type TechnologyRadar,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let risk = $state<DependencyRisk | null>(null);
  let radar = $state<TechnologyRadar | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      risk = await ppm.technologyDependencyRisk();
      radar = await ppm.technologyRadar();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });

  const RINGS = ["adopt", "trial", "assess", "hold", "unclassified"] as const;
</script>

<svelte:head><title>{t("ppm.nav.technology")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.technology")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if radar}
  <h2>Technology radar</h2>
  <p class="muted">as of {radar.as_of} · {radar.convention}</p>
  {#each RINGS as ring (ring)}
    {@const inRing = radar.technologies.filter((tech) => tech.ring === ring)}
    {#if inRing.length > 0}
      <h3 class="ring-title">{ring}</h3>
      <ul class="ring" data-testid="radar-{ring}">
        {#each inRing as tech (tech.technology)}
          <li>
            <strong>{tech.technology}</strong>
            <span class="muted">
              {Object.entries(tech.per_collection)
                .map(([kind, count]) => `${count} ${kind}`)
                .join(" · ")}
              {#if tech.ring_votes > 0}&nbsp;({tech.ring_votes} ring votes){/if}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  {/each}
  {#if radar.technologies.length === 0}
    <p class="muted">No `tech:*` tags recorded yet.</p>
  {/if}
{/if}

{#if risk}
  <h2>Dependency risk</h2>
  <p class="muted">as of {risk.as_of} · {risk.edges} edges</p>

  <h3>Most depended-on (single-point-of-failure candidates)</h3>
  <table data-testid="tech-fan-out">
    <thead>
      <tr><th>Item</th><th>Kind</th><th>Dependents</th><th>RAG</th></tr>
    </thead>
    <tbody>
      {#each risk.top_fan_out as row (row.item.pid)}
        <tr>
          <td><a href="/{row.item.kind.toLowerCase()}s/{row.item.pid}">{row.item.name}</a></td>
          <td>{row.item.kind}</td>
          <td>{row.dependents}</td>
          <td>{row.rag ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No dependencies recorded yet.</td></tr>
      {/each}
    </tbody>
  </table>

  <h3>Edges importing risk from a red predecessor</h3>
  <table data-testid="tech-red-edges">
    <thead>
      <tr><th>Predecessor (red)</th><th>Successor at risk</th></tr>
    </thead>
    <tbody>
      {#each risk.red_predecessor_edges as edge (edge.edge)}
        <tr>
          <td>{edge.predecessor.name}</td>
          <td>{edge.successor.name}</td>
        </tr>
      {:else}
        <tr><td colspan="2" class="muted">None — no red predecessors.</td></tr>
      {/each}
    </tbody>
  </table>

  <h3>Cross-portfolio edges</h3>
  <table data-testid="tech-cross-portfolio">
    <thead>
      <tr><th>Predecessor</th><th>Successor</th></tr>
    </thead>
    <tbody>
      {#each risk.cross_portfolio as edge (edge.edge)}
        <tr>
          <td>{edge.predecessor.name} <span class="muted">({edge.predecessor.kind})</span></td>
          <td>{edge.successor.name} <span class="muted">({edge.successor.kind})</span></td>
        </tr>
      {:else}
        <tr><td colspan="2" class="muted">None — all edges stay within one portfolio.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .ring-title { text-transform: capitalize; margin-bottom: 0.25rem; }
  .ring { list-style: none; padding: 0; margin: 0 0 1rem; }
  .ring li { padding: 0.2rem 0; }
</style>
