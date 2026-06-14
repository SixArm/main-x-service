<!--
  MatchResultsList — read-only list of match/duplicate candidates.
  Shows each hit's name, confidence pill, percentage score, a link to
  the course, and an expandable per-component score breakdown. Used by
  the match-check and new-course (duplicate) pages.

  $props:
    - results: MatchResult[] — candidates to render (sorted upstream).
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
                        <strong>{r.name}</strong>
                        <span class="quality" data-quality={r.confidence}>{r.confidence}</span>
                        <span class="score">{(r.score * 100).toFixed(0)}%</span>
                    </header>
                    <div class="meta small muted">
                        {#if r.course_code}{r.course_code}{/if}
                        <!-- Truncate the UUID to 8 chars for a compact, linkable hint. -->
                        {#if r.course_id} · <a href={`/courses/${r.course_id}`}>{r.course_id.slice(0, 8)}…</a>{/if}
                    </div>
                    {#if r.breakdown}
                        <details>
                            <summary class="small">Score breakdown</summary>
                            <ul class="breakdown small">
                                {#if r.breakdown.name_score != null}<li>name: {(r.breakdown.name_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.course_code_score != null}<li>course code: {(r.breakdown.course_code_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.provider_score != null}<li>provider: {(r.breakdown.provider_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.educational_level_score != null}<li>level: {(r.breakdown.educational_level_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.keywords_score != null}<li>keywords: {(r.breakdown.keywords_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.teaches_score != null}<li>teaches: {(r.breakdown.teaches_score * 100).toFixed(0)}%</li>{/if}
                                {#if r.breakdown.deterministic_match}<li>deterministic (identifier / provider+code / same-as)</li>{/if}
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
    .quality[data-quality="High"] { background: #dcfce7; color: var(--mxi-color-success); }
    .quality[data-quality="Medium"] { background: #dbeafe; color: var(--mxi-color-primary); }
    .quality[data-quality="Low"] { background: #fef3c7; color: #92400e; }
    .meta { margin-top: 0.25rem; }
    .breakdown { margin: 0.25rem 0 0; padding-left: 1rem; }
</style>
