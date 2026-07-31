<!--
  Dashboard (CMS-T25 scaffold): pick a site, then see what the service
  says about it.

  Two deliberate properties, both from `../../spec/insights.md`:

  - **The client formats, it does not compute.** Every number here comes
    from the API. Nothing is summed, averaged, or inferred in the
    browser, so the dashboard cannot disagree with the service.
  - **No data is not zero.** A rate whose denominator is zero arrives as
    `null` and renders as "no data yet", never as `0%` — which would
    read as "we measured, and it was nothing".

  The views themselves (entries, assets, workflow, translations,
  insights, settings) are CMS-T26.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import type { Health, Site, Backlog } from "$lib/api/cms";

  let sites = $state<Site[] | null>(null);
  let selected = $state<string | null>(null);
  let health = $state<Health | null>(null);
  let backlog = $state<Backlog | null>(null);
  let failure = $state<string | null>(null);

  $effect(() => {
    cms
      .listSites()
      .then((rows) => {
        sites = rows;
        selected ??= rows[0]?.pid ?? null;
      })
      .catch((error: unknown) => {
        failure = error instanceof Error ? error.message : String(error);
      });
  });

  $effect(() => {
    const pid = selected;
    if (!pid) return;
    Promise.all([cms.health(pid, null), cms.backlog(pid)])
      .then(([healthResponse, backlogResponse]) => {
        health = healthResponse.body;
        backlog = backlogResponse;
      })
      .catch((error: unknown) => {
        failure = error instanceof Error ? error.message : String(error);
      });
  });

  /** A timestamp as the page shows it — dates are formatted in the
   *  UI locale, not hard-coded to one region's order. */
  function when(value: string | undefined): string {
    return value ? new Date(value).toLocaleString() : "";
  }
</script>

<svelte:head><title>Content Management System</title></svelte:head>

<h1>{t("nav.dashboard")}</h1>

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else if !sites}
  <div class="panel">{t("common.loading")}</div>
{:else}
  <div class="panel">
    <label>
      {t("site.choose")}
      <select bind:value={selected}>
        {#each sites as site (site.pid)}
          <option value={site.pid}>{site.name} ({site.key})</option>
        {/each}
      </select>
    </label>
  </div>

  {#if health}
    <h2>{t("insights.health")}</h2>
    <p class="as-of">{t("insights.asOf")} {when(health.as_of)}</p>
    <div class="tiles">
      <div class="tile">
        <div class="value">{health.entries}</div>
        <div class="label">{t("nav.entries")}</div>
      </div>
      <div class="tile">
        <div class="value">{health.published_variants}</div>
        <div class="label">{t("entry.published")}</div>
      </div>
      <div class="tile">
        <div class="value">{health.findings_total}</div>
        <div class="label">{t("insights.findings")}</div>
      </div>
    </div>

    {#if health.by_rule.length === 0}
      <p class="no-data">{t("insights.noFindings")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{t("insights.rule")}</th>
            <th>{t("insights.findings")}</th>
            <th>{t("common.title")}</th>
          </tr>
        </thead>
        <tbody>
          {#each health.by_rule as group (group.rule)}
            <tr>
              <td><span class="rule">{group.rule}</span></td>
              <td>{group.count}</td>
              <!-- The explanation is the service's own sentence for the
                   rule. Rewording it here would let the UI and the API
                   disagree about what was actually checked. -->
              <td>{group.explanation}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}

  {#if backlog}
    <h2>{t("insights.backlog")}</h2>
    <div class="tiles">
      <div class="tile">
        <div class="value">{backlog.pending_review.length}</div>
        <div class="label">{t("entry.inReview")}</div>
      </div>
      <div class="tile">
        <div class="value">{backlog.pending_schedule.length}</div>
        <div class="label">{t("nav.workflow")}</div>
      </div>
      <div class="tile">
        <div class="value">{backlog.open_translations.length}</div>
        <div class="label">{t("nav.translations")}</div>
      </div>
    </div>
  {/if}
{/if}
