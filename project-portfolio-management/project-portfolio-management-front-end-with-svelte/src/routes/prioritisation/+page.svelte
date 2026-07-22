<!--
  Data-driven prioritisation (`/prioritisation`): the Smart Score queue.
  Highest score first, with each plan's evidence coverage on show, and
  an expandable per-component breakdown — a score that ranks somebody's
  work has to be able to explain itself.

  Unscored plans keep the `unscored` band and sort last: no evidence is
  reported as no evidence, never as a low score.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    CapabilityClient,
    type Prioritisation,
    type PlanSmartScore,
  } from "$lib/api/capabilities";

  const api = CapabilityClient.withFetch();
  let view = $state<Prioritisation | null>(null);
  let error = $state<string | null>(null);
  let band = $state("");
  let expanded = $state<string | null>(null);
  let detail = $state<PlanSmartScore | null>(null);

  async function refresh() {
    try {
      view = await api.prioritisation({ band: band || undefined });
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load the queue";
    }
  }

  async function toggle(pid: string) {
    if (expanded === pid) {
      expanded = null;
      detail = null;
      return;
    }
    expanded = pid;
    detail = null;
    try {
      detail = await api.smartScore(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load the score";
    }
  }

  onMount(refresh);
</script>

<svelte:head><title>Prioritisation — PPM</title></svelte:head>

<h1>Prioritisation</h1>
<p class="small muted">
  Smart Score over ROI, strategic alignment, expert review, risk, demand,
  MoSCoW priority, and momentum. Components with no evidence are dropped
  and the rest renormalised — <em>coverage</em> shows how much of the
  score was actually backed by data.
</p>

{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form
  class="row"
  onsubmit={(e) => {
    e.preventDefault();
    refresh();
  }}
>
  <label class="small">
    band
    <select bind:value={band}>
      <option value="">all</option>
      <option value="high">high</option>
      <option value="medium">medium</option>
      <option value="low">low</option>
      <option value="unscored">unscored</option>
    </select>
  </label>
  <button class="button small" type="submit">Apply</button>
</form>

{#if view}
  <p class="small muted">{view.plans.length} of {view.scored_of} plan(s)</p>
  <table>
    <thead>
      <tr>
        <th>Plan</th><th>Score</th><th>Band</th><th>Coverage</th><th>Missing evidence</th><th></th>
      </tr>
    </thead>
    <tbody>
      {#each view.plans as plan (plan.pid)}
        <tr>
          <td><a href={`/plans/${plan.pid}`}>{plan.name}</a></td>
          <td>{plan.score === null ? "—" : plan.score.toFixed(1)}</td>
          <td><span class={`chip ${plan.band}`}>{plan.band}</span></td>
          <td>{Math.round(plan.coverage * 100)}%</td>
          <td class="small muted">
            {plan.missing_evidence.length === 0
              ? "none"
              : plan.missing_evidence.join(", ")}
          </td>
          <td>
            <button class="button small" type="button" onclick={() => toggle(plan.pid)}>
              {expanded === plan.pid ? "Hide" : "Explain"}
            </button>
          </td>
        </tr>
        {#if expanded === plan.pid}
          <tr>
            <td colspan="6">
              {#if detail}
                <table class="inner">
                  <thead>
                    <tr><th>Component</th><th>Weight</th><th>Value</th><th>Points</th></tr>
                  </thead>
                  <tbody>
                    {#each detail.smart_score.components as component (component.name)}
                      <tr>
                        <td>{component.name}</td>
                        <td>{Math.round(component.weight * 100)}%</td>
                        <td>{component.raw.toFixed(2)}</td>
                        <td>{component.contribution.toFixed(1)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                {#if detail.smart_score.components.length === 0}
                  <p class="small muted">
                    No evidence for any component — this plan is unscored.
                  </p>
                {/if}
              {:else}
                <p class="small muted">Loading the breakdown…</p>
              {/if}
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
  {#if view.plans.length === 0}
    <p class="small muted">No plans match that filter.</p>
  {/if}
{/if}

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .chip {
    display: inline-block;
    border-radius: 999px;
    padding: 0 0.5rem;
    font-size: 0.75rem;
    border: 1px solid currentColor;
  }
  .chip.high { color: #1d8a4e; }
  .chip.medium { color: #8a6d1d; }
  .chip.low { color: #a4262c; }
  .chip.unscored { color: var(--text-2, #666); }
  table.inner { width: 100%; font-size: 0.85rem; }
</style>
