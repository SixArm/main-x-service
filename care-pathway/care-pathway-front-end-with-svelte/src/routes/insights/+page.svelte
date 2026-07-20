<!--
  Insights route (`/insights`) — the five read-only registry lenses
  (directory, coverage, variants, providers, languages) rendered as
  tables. Each lens carries a `note` string from the service that is
  shown verbatim (the server derives the numbers; the UI does not
  recompute them). English-first bodies (family convention).
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type {
        CoverageInsight,
        DirectoryInsight,
        LanguagesInsight,
        ProvidersInsight,
        VariantsInsight,
    } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CarePathwayRepository.withFetch();

    let directory = $state<DirectoryInsight | null>(null);
    let coverage = $state<CoverageInsight | null>(null);
    let variants = $state<VariantsInsight | null>(null);
    let providers = $state<ProvidersInsight | null>(null);
    let languages = $state<LanguagesInsight | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            [directory, coverage, variants, providers, languages] =
                await Promise.all([
                    repo.insightsDirectory(),
                    repo.insightsCoverage(),
                    repo.insightsVariants(),
                    repo.insightsProviders(),
                    repo.insightsLanguages(),
                ]);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Entries of a `Record<string, T>` as [key, value] pairs (typed).
    function entries<T>(o: Record<string, T> | undefined): [string, T][] {
        return o ? Object.entries(o) : [];
    }
</script>

<svelte:head><title>{t("nav.insights")} — Main X</title></svelte:head>

<h1>{t("nav.insights")}</h1>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else}
    <!-- Directory: by care setting + by specialty. -->
    <section data-testid="insight-directory" class="stack">
        <h2>Directory</h2>
        {#if directory}
            <p class="muted small">{directory.note}</p>
            <table>
                <thead><tr><th>Care setting</th><th>Pathways</th></tr></thead>
                <tbody>
                    {#each entries(directory.by_setting) as [setting, rows] (setting)}
                        <tr>
                            <td>{setting}</td>
                            <td>{rows.map((r) => r.name).join(", ")}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
            <table>
                <thead><tr><th>Specialty</th><th>Count</th></tr></thead>
                <tbody>
                    {#each entries(directory.by_specialty) as [specialty, count] (specialty)}
                        <tr><td>{specialty}</td><td>{count}</td></tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </section>

    <!-- Coverage: conditions × settings + disclosed gaps. -->
    <section data-testid="insight-coverage" class="stack">
        <h2>Coverage</h2>
        {#if coverage}
            <p class="muted small">{coverage.note}</p>
            <table>
                <thead><tr><th>Condition</th><th>Settings</th></tr></thead>
                <tbody>
                    {#each coverage.conditions as row (row.condition)}
                        <tr><td>{row.condition}</td><td>{row.settings.join(", ")}</td></tr>
                    {/each}
                </tbody>
            </table>
            <h3>Gaps</h3>
            <table>
                <thead><tr><th>Rule</th><th>Condition</th><th>Detail</th></tr></thead>
                <tbody>
                    {#each coverage.gaps as gap, i (i)}
                        <tr><td>{gap.rule}</td><td>{gap.condition}</td><td>{gap.detail}</td></tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </section>

    <!-- Variants: conditions offered by more than one provider. -->
    <section data-testid="insight-variants" class="stack">
        <h2>Variants</h2>
        {#if variants}
            <p class="muted small">{variants.note}</p>
            <table>
                <thead><tr><th>Condition</th><th>Providers</th></tr></thead>
                <tbody>
                    {#each variants.variants as row (row.condition)}
                        <tr><td>{row.condition}</td><td>{row.providers}</td></tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </section>

    <!-- Providers: pathways per issuing provider. -->
    <section data-testid="insight-providers" class="stack">
        <h2>Providers</h2>
        {#if providers}
            <p class="muted small">{providers.note}</p>
            <table>
                <thead><tr><th>Provider</th><th>Pathways</th></tr></thead>
                <tbody>
                    {#each providers.providers as row (row.provider)}
                        <tr><td>{row.provider}</td><td>{row.pathways}</td></tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </section>

    <!-- Languages: per-language counts + single-language conditions. -->
    <section data-testid="insight-languages" class="stack">
        <h2>Languages</h2>
        {#if languages}
            <p class="muted small">{languages.note}</p>
            <table>
                <thead><tr><th>Language</th><th>Count</th></tr></thead>
                <tbody>
                    {#each entries(languages.by_language) as [lang, count] (lang)}
                        <tr><td>{lang}</td><td>{count}</td></tr>
                    {/each}
                </tbody>
            </table>
            <h3>Single-language conditions</h3>
            <table>
                <thead><tr><th>Condition</th><th>Language</th></tr></thead>
                <tbody>
                    {#each languages.single_language_conditions as row (row.condition)}
                        <tr><td>{row.condition}</td><td>{row.language ?? "—"}</td></tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </section>
{/if}

<style>
    table {
        border-collapse: collapse;
        width: 100%;
    }
    th,
    td {
        text-align: left;
        padding: 0.35rem 0.6rem;
        border-bottom: 1px solid var(--mxi-color-border, #ddd);
        vertical-align: top;
    }
    section + section {
        margin-top: 2rem;
    }
</style>
