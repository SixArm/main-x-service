<!--
  MatchResultsList — renders a list of match/duplicate candidates, each with
  its name, quality badge, percentage score, key metadata, and an optional
  expandable per-component score breakdown.

  Props:
    - results (MatchResult[]): candidates to display (may be empty).
    - title (string, default "Match results"): section heading.
-->
<script lang="ts">
    import type { MatchResult } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let {
        results,
        title,
    }: {
        results: MatchResult[];
        title?: string;
    } = $props();

    // Default the section heading to the localized "Match results".
    const heading = $derived(title ?? t("results.title"));
</script>

<section class="surface stack">
    <h2>{heading} <span class="muted small">({results.length})</span></h2>
    {#if results.length === 0}
        <p class="muted">{t("results.none")}</p>
    {:else}
        <ul class="results">
            {#each results as r}
                <li class="result">
                    <header>
                        <strong>{r.event.name}</strong>
                        <span class="quality" data-quality={r.quality}>{r.quality}</span>
                        <span class="score">{(r.score * 100).toFixed(0)}%</span>
                    </header>
                    <div class="meta small muted">
                        <!-- Localized start date, then type, then a short id linking to detail. -->
                        {#if r.event.start_date}{new Date(r.event.start_date).toLocaleString()}{/if}
                        {#if r.event.event_type} · {r.event.event_type}{/if}
                        {#if r.event.id} · <a href={`/events/${r.event.id}`}>{r.event.id.slice(0, 8)}…</a>{/if}
                    </div>
                    {#if r.breakdown}
                        <details>
                            <summary class="small">{t("results.breakdown")}</summary>
                            <ul class="breakdown small">
                                <!-- Skip components the matcher left null (not applicable). -->
                                {#each Object.entries(r.breakdown) as [field, score]}
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
