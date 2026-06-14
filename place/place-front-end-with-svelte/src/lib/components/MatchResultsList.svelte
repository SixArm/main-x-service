<!--
  MatchResultsList — read-only list of match/duplicate candidates with a
  confidence pill, a percentage score, light metadata, and an expandable
  per-component score breakdown. Pure presentation; emits no events.

  $props:
    - results (MatchResult[]) — candidates to render (already ranked).
    - title (string) — heading text. Default "Match results".
-->
<script lang="ts">
    import type { MatchResult } from "$lib/api/types.js";

    let {
        results,
        title = "Match results",
    }: {
        results: MatchResult[];
        title?: string;
    } = $props();

    // Render a PlaceType for display: known variants print as-is; the open
    // `{ Other }` variant is shown as "Other: <value>".
    function typeLabel(t: MatchResult["place"]["place_type"]): string {
        if (!t) return "";
        return typeof t === "string" ? t : `Other: ${t.Other}`;
    }
</script>

<section class="surface stack">
    <h2>{title} <span class="muted small">({results.length})</span></h2>
    {#if results.length === 0}
        <p class="muted">No candidates.</p>
    {:else}
        <ul class="results">
            {#each results as r}
                <li class="result">
                    <header>
                        <strong>{r.place.name}</strong>
                        <!-- data-quality drives the colour of the confidence pill (see <style>). -->
                        <span class="quality" data-quality={r.confidence}>{r.confidence}</span>
                        <!-- Scores are 0–1; render as a whole-number percentage. -->
                        <span class="score">{(r.score * 100).toFixed(0)}%</span>
                    </header>
                    <div class="meta small muted">
                        {#if r.place.place_type}{typeLabel(r.place.place_type)}{/if}
                        {#if r.place.address?.address_locality} · {r.place.address.address_locality}{/if}
                        <!-- Link to the candidate's detail page, showing a short id prefix. -->
                        {#if r.place.id} · <a href={`/places/${r.place.id}`}>{r.place.id.slice(0, 8)}…</a>{/if}
                    </div>
                    <!-- Optional per-component breakdown, collapsed by default. -->
                    {#if r.breakdown}
                        <details>
                            <summary class="small">Score breakdown</summary>
                            <ul class="breakdown small">
                                {#if r.breakdown.name_score != null}<li>name: {(r.breakdown.name_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.geo_score != null}<li>geo: {(r.breakdown.geo_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.address_score != null}<li>address: {(r.breakdown.address_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.identifier_score != null}<li>identifier: {(r.breakdown.identifier_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.phonetic_match}<li>phonetic match</li>{/if}
                                {#if r.breakdown.deterministic_match}<li>deterministic (GLN)</li>{/if}
                            </ul>
                        </details>
                    {/if}
                </li>
            {/each}
        </ul>
    {/if}
</section>

<style>
    .results { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.5rem; }
    .result { padding: 0.625rem 0.75rem; border: 1px solid var(--mxi-color-border); border-radius: var(--mxi-radius); }
    .result header { display: flex; gap: 0.5rem; align-items: baseline; }
    .score { margin-left: auto; font-variant-numeric: tabular-nums; font-weight: 600; }
    .quality { padding: 0.125rem 0.5rem; border-radius: 999px; font-size: 0.75rem; background: #f3f4f6; }
    .quality[data-quality="Certain"] { background: #dcfce7; color: var(--mxi-color-success); }
    .quality[data-quality="Probable"] { background: #dbeafe; color: var(--mxi-color-primary); }
    .quality[data-quality="Possible"] { background: #fef3c7; color: #92400e; }
    .quality[data-quality="Unlikely"] { background: #fee2e2; color: var(--mxi-color-danger); }
    .meta { margin-top: 0.25rem; }
    .breakdown { margin: 0.25rem 0 0; padding-left: 1rem; }
</style>
