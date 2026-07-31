<!--
  Translations: coverage, open requests, and staleness.

  Staleness is shown as **"N source revisions behind"**, never as a bare
  badge — the number is what tells a translator whether this is a typo
  fix or a rewrite. The service also distinguishes "unknown" from "up to
  date", and so does this page: telling someone their translation is
  fine when nobody knows is the failure mode worth avoiding.

  The rule the service applied is printed verbatim, so the page and the
  API cannot disagree about what "stale" meant.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import { actor, when } from "$lib/format";
  import SitePicker from "$lib/components/SitePicker.svelte";
  import type { TranslationQueueItem } from "$lib/api/cms";

  let site = $state<string | null>(null);
  let queue = $state<TranslationQueueItem[]>([]);
  let rule = $state("");
  let asOf = $state("");
  let coverage = $state<CoverageRow[]>([]);
  let failure = $state<string | null>(null);

  interface CoverageRow {
    locale: string;
    entries_total: number;
    entries_started: number;
    entries_published: number;
    missing_entry_keys: string[];
  }

  $effect(() => {
    const pid = site;
    if (!pid) return;
    Promise.all([cms.translations(pid), cms.localeCoverage(pid)])
      .then(([rows, covered]) => {
        queue = rows.queue;
        rule = rows.rule;
        asOf = rows.as_of;
        coverage = covered.coverage as CoverageRow[];
      })
      .catch((e: unknown) => (failure = String(e)));
  });
</script>

<svelte:head><title>{t("nav.translations")}</title></svelte:head>

<h1>{t("nav.translations")}</h1>
<SitePicker bind:site />

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else}
  <p class="as-of">{t("insights.asOf")} {when(asOf)}</p>
  {#if rule}
    <!-- The service's own sentence for the rule. Rewording it here
         would let the page and the API disagree about what was
         actually checked. -->
    <p class="rule">{rule}</p>
  {/if}

  <h2>{t("entry.locales")}</h2>
  <table>
    <thead>
      <tr>
        <th>{t("common.locale")}</th>
        <th>{t("nav.entries")}</th>
        <th>{t("entry.draft")}</th>
        <th>{t("entry.published")}</th>
      </tr>
    </thead>
    <tbody>
      {#each coverage as row (row.locale)}
        <tr>
          <td>{row.locale}</td>
          <td>{row.entries_total}</td>
          <td>{row.entries_started}</td>
          <td>{row.entries_published}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("translations.queue")}</h2>
  {#if queue.length === 0}
    <p class="no-data">{t("common.noData")}</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>{t("entry.key")}</th><th>{t("common.locale")}</th>
          <th>{t("common.status")}</th><th>{t("common.author")}</th><th>{t("common.updated")}</th>
        </tr>
      </thead>
      <tbody>
        {#each queue as item (item.entry_pid + item.locale)}
          <tr>
            <td><a href="/entries/{item.entry_pid}">{item.entry_key}</a></td>
            <td>{item.locale}</td>
            <td>{item.translation_status}</td>
            <td>{actor(item.translator_ref)}</td>
            <td>{when(item.requested_at)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/if}
