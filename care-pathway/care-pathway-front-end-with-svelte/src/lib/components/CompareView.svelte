<!--
  The rule-split "compare" two-column view (spec T-14f/T-14l): a
  matched side and, when `?compare=true` was asked for, its
  complement, each rendered in the same shape the unsplit cohort tile
  already uses. A side below the minimum cell count (T-14k) is shown
  withheld rather than blank — and so is its sibling, whenever showing
  it would let the withheld side's figures be recovered by subtracting
  from the published unsplit total.
-->
<script lang="ts">
  import type { Split, SplitSide } from "$lib/api/tba";
  import { percent } from "$lib/api/tba";
  import { withheldLabel } from "$lib/analytics-transforms";
  import { t } from "$lib/i18n.svelte";

  interface Props {
    split: Split;
  }
  let { split }: Props = $props();
</script>

{#snippet side(
  kind: "matched" | "complement",
  label: string,
  data: SplitSide | null,
)}
  <div class="compare-column" data-testid={`compare-${kind}`}>
    <h4>{label}</h4>
    {#if data === null}
      <p class="muted">
        Not requested — add <code>compare=true</code> to see the complement.
      </p>
    {:else if data.suppressed}
      <p class="finding">{withheldLabel(data.suppression_note)}</p>
      <p class="muted">{data.instances} instances</p>
    {:else}
      <p class="muted">{data.instances} instances</p>
      {#if data.cohort}
        <div class="tile">
          <span class="tile-label">Value-adding time</span>
          <span class="tile-value">
            {percent(data.cohort.aggregate_value_adding_ratio.value, 1)}
          </span>
        </div>
      {/if}
      {#if data.compliance}
        <div class="tile">
          <span class="tile-label">{data.compliance.standard}</span>
          <span class="tile-value"
            >{percent(data.compliance.achieved_ratio)}</span
          >
        </div>
      {/if}
    {/if}
  </div>
{/snippet}

<div class="compare-wrap" data-testid="compare-view">
  <p class="muted">
    Matching
    {#if split.rule.contains.length > 0}<code
        >contains: {split.rule.contains.join(", ")}</code
      >{/if}
    {#if split.rule.excludes.length > 0}<code
        >excludes: {split.rule.excludes.join(", ")}</code
      >{/if}
  </p>
  <div class="columns">
    {@render side("matched", t("time.matched"), split.matched)}
    {@render side("complement", t("time.complement"), split.complement)}
  </div>
</div>

<style>
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  .compare-column {
    border: 1px solid rgb(128 128 128 / 0.25);
    border-radius: 6px;
    padding: 0.75rem 1rem;
  }
  .compare-column h4 {
    margin: 0 0 0.5rem;
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    margin: 0.4rem 0;
  }
  .tile-label {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.75;
  }
  .tile-value {
    font-size: 1.4rem;
  }
  .finding {
    margin: 0.5rem 0;
    padding: 0.5rem 0.7rem;
    border-left: 3px solid rgb(128 128 128 / 0.4);
    font-size: 0.85rem;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.85rem;
  }
</style>
