<!--
  MatchResultsList — read-only list of scored match candidates.

  Purpose: renders match / duplicate-check results with a coloured quality
  pill, a percentage score, a link to the matched thing, and an expandable
  per-component score breakdown. Shows an empty-state when there are none.

  $props:
    - results (MatchResult[]): the candidates to display, in service order.
    - title (string, default "Match results"): section heading text.
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

    // Fall back to the localized default heading when no title is supplied.
    const heading = $derived(title ?? t("results.title"));
</script>

<section class="surface stack">
    <h2>{heading} <span class="muted small">({results.length})</span></h2>
    {#if results.length === 0}
        <p class="muted">{t("results.noCandidates")}</p>
    {:else}
        <ul class="results">
            {#each results as r}
                <li class="result">
                    <header>
                        <strong>{r.thing.name}</strong>
                        <!-- data-quality drives the per-bucket pill colour in CSS. -->
                        <span class="quality" data-quality={r.confidence}
                            >{r.confidence}</span
                        >
                        <!-- Score is 0..1 from the API; show as a whole percentage. -->
                        <span class="score">{(r.score * 100).toFixed(0)}%</span>
                    </header>
                    <div class="meta small muted">
                        {#if r.thing.additional_type}{r.thing
                                .additional_type}{/if}
                        <!-- Link to the candidate, abbreviating its UUID for readability. -->
                        {#if r.thing.id}
                            · <a href={`/things/${r.thing.id}`}
                                >{r.thing.id.slice(0, 8)}…</a
                            >{/if}
                    </div>
                    <!-- Optional per-component breakdown; each line only shows
                         when that component actually contributed (non-null). -->
                    {#if r.breakdown}
                        <details>
                            <summary class="small"
                                >{t("results.scoreBreakdown")}</summary
                            >
                            <ul class="breakdown small">
                                {#if r.breakdown.name_score != null}<li>
                                        {t("results.nameScore")}: {(
                                            r.breakdown.name_score * 100
                                        ).toFixed(0)}%
                                    </li>{/if}
                                {#if r.breakdown.identifier_score != null}<li>
                                        {t("results.identifierScore")}: {(
                                            r.breakdown.identifier_score * 100
                                        ).toFixed(0)}%
                                    </li>{/if}
                                {#if r.breakdown.description_score != null}<li>
                                        {t("results.descriptionScore")}: {(
                                            r.breakdown.description_score * 100
                                        ).toFixed(0)}%
                                    </li>{/if}
                                {#if r.breakdown.url_score != null}<li>
                                        {t("results.urlScore")}: {(
                                            r.breakdown.url_score * 100
                                        ).toFixed(0)}%
                                    </li>{/if}
                                {#if r.breakdown.same_as_score != null}<li>
                                        {t("results.sameAsScore")}: {(
                                            r.breakdown.same_as_score * 100
                                        ).toFixed(0)}%
                                    </li>{/if}
                                {#if r.breakdown.phonetic_match}<li>
                                        {t("results.phoneticMatch")}
                                    </li>{/if}
                                {#if r.breakdown.deterministic_match}<li>
                                        {t("results.deterministicMatch")}
                                    </li>{/if}
                            </ul>
                        </details>
                    {/if}
                </li>
            {/each}
        </ul>
    {/if}
</section>

<style>
    .results {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .result {
        padding: 0.625rem 0.75rem;
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
    }
    .result header {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
    }
    .score {
        margin-left: auto;
        font-variant-numeric: tabular-nums;
        font-weight: 600;
    }
    .quality {
        padding: 0.125rem 0.5rem;
        border-radius: 999px;
        font-size: 0.75rem;
        background: #f3f4f6;
    }
    .quality[data-quality="Certain"] {
        background: #dcfce7;
        color: var(--mxi-color-success);
    }
    .quality[data-quality="Probable"] {
        background: #dbeafe;
        color: var(--mxi-color-primary);
    }
    .quality[data-quality="Possible"] {
        background: #fef3c7;
        color: #92400e;
    }
    .quality[data-quality="Unlikely"] {
        background: #fee2e2;
        color: var(--mxi-color-danger);
    }
    .meta {
        margin-top: 0.25rem;
    }
    .breakdown {
        margin: 0.25rem 0 0;
        padding-left: 1rem;
    }
</style>
