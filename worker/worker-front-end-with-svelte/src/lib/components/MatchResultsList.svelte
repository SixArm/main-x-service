<!--
  MatchResultsList — renders a list of scored match/duplicate candidates.
  Each row shows the worker's name, a colour-coded quality badge, the score
  as a percentage, light identity metadata, and an expandable per-component
  score breakdown.

  $props:
    - results: MatchResult[] — candidates to display (may be empty).
    - title?: string — section heading (default "Match results").
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
                        <strong>
                            {r.worker.name.given.join(" ")} {r.worker.name.family}
                        </strong>
                        <!-- data-quality drives the badge colour via CSS. -->
                        <span class="quality" data-quality={r.quality}>{r.quality}</span>
                        <!-- Score is 0–1; render as a whole-number percentage. -->
                        <span class="score">{(r.score * 100).toFixed(0)}%</span>
                    </header>
                    <div class="meta small muted">
                        {#if r.worker.birth_date}DOB {r.worker.birth_date}{/if}
                        {#if r.worker.gender} · {r.worker.gender}{/if}
                        {#if r.worker.id} · <a href={`/workers/${r.worker.id}`}>{r.worker.id.slice(0, 8)}…</a>{/if}
                    </div>
                    {#if r.breakdown}
                        <details>
                            <summary class="small">Score breakdown</summary>
                            <ul class="breakdown small">
                                {#each Object.entries(r.breakdown) as [field, score]}
                                    <!-- Skip components that didn't contribute (null). -->
                                    {#if score != null}
                                        <li>{field}: {(score * 100).toFixed(0)}%</li>
                                    {/if}
                                {/each}
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
    .quality[data-quality="certain"], .quality[data-quality="definite"] { background: #dcfce7; color: var(--mxi-color-success); }
    .quality[data-quality="probable"] { background: #dbeafe; color: var(--mxi-color-primary); }
    .quality[data-quality="possible"] { background: #fef3c7; color: #92400e; }
    .quality[data-quality="unlikely"] { background: #fee2e2; color: var(--mxi-color-danger); }
    .meta { margin-top: 0.25rem; }
    .breakdown { margin: 0.25rem 0 0; padding-left: 1rem; }
</style>
