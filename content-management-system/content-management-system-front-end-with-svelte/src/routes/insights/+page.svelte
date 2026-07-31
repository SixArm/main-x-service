<!--
  Insights: content health and editorial throughput.

  Three honesty rules, all from `../../spec/insights.md`, and all
  enforced by what this page refuses to do:

  - Every finding shows **the rule that produced it** and the service's
    own explanation of that rule.
  - A ratio shows its **numerator and denominator**, and a `null` value
    renders as no-data rather than `0%`.
  - Nothing here is computed in the browser. Percentages come from
    `value`; counts come from the payload. The page cannot disagree
    with the service because it never does the arithmetic.

  There are no reader analytics, because the service records none.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import { duration, percent, when, workings } from "$lib/format";
  import SitePicker from "$lib/components/SitePicker.svelte";
  import type { Health, Throughput } from "$lib/api/cms";

  let site = $state<string | null>(null);
  let health = $state<Health | null>(null);
  let throughput = $state<Throughput | null>(null);
  let days = $state(30);
  let failure = $state<string | null>(null);

  $effect(() => {
    const pid = site;
    if (!pid) return;
    const window = days;
    Promise.all([cms.health(pid, null), cms.throughput(pid, window)])
      .then(([h, tp]) => {
        health = h.body;
        throughput = tp;
      })
      .catch((e: unknown) => (failure = String(e)));
  });

  interface Percentiles {
    sample_size?: number;
    median_seconds?: number;
    p90_seconds?: number;
  }
</script>

<svelte:head><title>{t("nav.insights")}</title></svelte:head>

<h1>{t("nav.insights")}</h1>
<SitePicker bind:site />

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{/if}

{#if health}
  <h2>{t("insights.health")}</h2>
  <p class="as-of">{t("insights.asOf")} {when(health.as_of)}</p>
  {#if health.by_rule.length === 0}
    <p class="no-data">{t("insights.noFindings")}</p>
  {:else}
    {#each health.by_rule as group (group.rule)}
      <section class="panel">
        <h3><span class="rule">{group.rule}</span> · {group.count}</h3>
        <p class="muted">{group.explanation}</p>
        <table>
          <thead>
            <tr><th>{t("common.title")}</th><th>{t("common.locale")}</th><th>{t("workflow.remedy")}</th></tr>
          </thead>
          <tbody>
            <!-- Keyed by position: a finding has no id, and two
                 findings from the same rule can legitimately share a
                 subject and locale (one page with two broken
                 references). Keying on those crashed the view. -->
            {#each group.findings as finding, index (index)}
              <tr>
                <td>{finding.subject}</td>
                <td>{finding.locale ?? "—"}</td>
                <td>{finding.detail}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/each}
  {/if}
{/if}

{#if throughput}
  <h2>{t("insights.throughput")}</h2>
  <p class="as-of">{t("insights.asOf")} {when(throughput.as_of)} · {throughput.period_days}d</p>
  <label class="panel">
    {t("insights.throughput")}
    <select bind:value={days}>
      {#each [7, 30, 90, 365] as window (window)}
        <option value={window}>{window}d</option>
      {/each}
    </select>
  </label>

  <div class="tiles">
    {#each Object.entries(throughput.activity) as [name, count] (name)}
      <div class="tile">
        <div class="value">{count}</div>
        <div class="label">{name}</div>
      </div>
    {/each}
  </div>

  <table>
    <thead>
      <tr><th>{t("insights.rule")}</th><th>%</th><th>{t("insights.findings")}</th></tr>
    </thead>
    <tbody>
      {#each Object.entries(throughput.rates) as [name, ratio] (name)}
        <tr>
          <td>{name}</td>
          <td>
            <!-- `null` means there was nothing to measure, which is not
                 the same claim as zero. -->
            {#if percent(ratio) === null}
              <span class="no-data">{t("common.noData")}</span>
            {:else}
              {percent(ratio)}
            {/if}
          </td>
          <td class="muted">{workings(ratio)}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <table>
    <thead>
      <tr><th>{t("common.status")}</th><th>median</th><th>p90</th><th>n</th></tr>
    </thead>
    <tbody>
      {#each Object.entries(throughput.time_in_state) as [name, value] (name)}
        {#if typeof value === "object" && value !== null}
          {@const stats = value as Percentiles}
          <tr>
            <td>{name}</td>
            <td>{duration(stats.median_seconds) ?? t("common.noData")}</td>
            <!-- The service suppresses percentiles below a sample floor
                 rather than publishing a number from three data points;
                 an absent p90 shows as no-data, not as the median. -->
            <td>
              {#if stats.p90_seconds === undefined}
                <span class="no-data">{t("common.noData")}</span>
              {:else}
                {duration(stats.p90_seconds)}
              {/if}
            </td>
            <td class="muted">{stats.sample_size ?? 0}</td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}
