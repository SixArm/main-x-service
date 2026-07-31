<!--
  Workflow: what is waiting for a person, and what is waiting for a
  clock.

  The backlog is bucketed by age because "three items in review" and
  "three items in review since March" are different problems, and the
  service already decided which bucket each falls in.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import { when } from "$lib/format";
  import SitePicker from "$lib/components/SitePicker.svelte";
  import type { Backlog, ScheduledItem } from "$lib/api/cms";

  let site = $state<string | null>(null);
  let backlog = $state<Backlog | null>(null);
  let queued = $state<ScheduledItem[]>([]);
  let failure = $state<string | null>(null);

  interface Waiting {
    entry_key: string;
    locale: string;
    age_days: number;
    bucket: string;
  }

  $effect(() => {
    const pid = site;
    if (!pid) return;
    Promise.all([cms.backlog(pid), cms.schedules(pid)])
      .then(([rows, schedule]) => {
        backlog = rows;
        queued = schedule.queued;
      })
      .catch((e: unknown) => (failure = String(e)));
  });

  const review = $derived((backlog?.pending_review ?? []) as Waiting[]);
  const translating = $derived((backlog?.open_translations ?? []) as Waiting[]);
</script>

<svelte:head><title>{t("nav.workflow")}</title></svelte:head>

<h1>{t("nav.workflow")}</h1>
<SitePicker bind:site />

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else if backlog}
  <p class="as-of">{t("insights.asOf")} {when(backlog.as_of)}</p>

  <h2>{t("entry.inReview")}</h2>
  {#if review.length === 0}
    <p class="no-data">{t("common.noData")}</p>
  {:else}
    <table>
      <thead>
        <tr><th>{t("entry.key")}</th><th>{t("common.locale")}</th><th>{t("common.updated")}</th></tr>
      </thead>
      <tbody>
        {#each review as item (item.entry_key + item.locale)}
          <tr>
            <td>{item.entry_key}</td>
            <td>{item.locale}</td>
            <td class:ahead={item.bucket === "older"}>{item.age_days}d · {item.bucket}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h2>{t("workflow.scheduled")}</h2>
  {#if queued.length === 0}
    <p class="no-data">{t("common.noData")}</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>{t("entry.key")}</th><th>{t("common.locale")}</th>
          <th>{t("common.status")}</th><th>{t("workflow.scheduled")}</th>
        </tr>
      </thead>
      <tbody>
        {#each queued as item (item.entry_pid + item.locale)}
          <tr>
            <td>{item.entry_key}</td>
            <td>{item.locale}</td>
            <td>{item.status}</td>
            <td>{when(item.publish_at ?? item.unpublish_at)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h2>{t("translations.queue")}</h2>
  {#if translating.length === 0}
    <p class="no-data">{t("common.noData")}</p>
  {:else}
    <table>
      <thead>
        <tr><th>{t("entry.key")}</th><th>{t("common.locale")}</th><th>{t("common.updated")}</th></tr>
      </thead>
      <tbody>
        {#each translating as item (item.entry_key + item.locale)}
          <tr><td>{item.entry_key}</td><td>{item.locale}</td><td>{item.age_days}d</td></tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/if}
