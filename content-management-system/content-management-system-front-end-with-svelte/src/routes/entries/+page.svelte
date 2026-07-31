<!--
  Entry list. One row per entry, with the locales it exists in and
  which of them are live — the two questions an author actually opens
  this page to answer.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import type { ContentType, Entry, Site } from "$lib/api/cms";

  let sites = $state<Site[] | null>(null);
  let site = $state<string | null>(null);
  let types = $state<ContentType[]>([]);
  let entries = $state<Entry[] | null>(null);
  let typeFilter = $state("");
  let search = $state("");
  let failure = $state<string | null>(null);

  $effect(() => {
    cms
      .listSites()
      .then((rows) => {
        sites = rows;
        site ??= rows[0]?.pid ?? null;
      })
      .catch((e: unknown) => (failure = String(e)));
  });

  $effect(() => {
    const pid = site;
    if (!pid) return;
    Promise.all([cms.listEntries(pid), cms.listContentTypes(pid)])
      .then(([rows, declared]) => {
        entries = rows;
        types = declared;
      })
      .catch((e: unknown) => (failure = String(e)));
  });

  // Filtering is client-side over an already-fetched list: this is a
  // narrowing aid, not a search endpoint, and it does not pretend to
  // be one by paging.
  const shown = $derived(
    (entries ?? []).filter(
      (entry) =>
        (!typeFilter || entry.content_type_key === typeFilter) &&
        (!search || entry.key.toLowerCase().includes(search.toLowerCase())),
    ),
  );
</script>

<svelte:head><title>{t("nav.entries")}</title></svelte:head>

<h1>{t("nav.entries")}</h1>

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else if !entries}
  <div class="panel">{t("common.loading")}</div>
{:else}
  <div class="panel row">
    <label>
      {t("site.choose")}
      <select bind:value={site}>
        {#each sites ?? [] as option (option.pid)}
          <option value={option.pid}>{option.name}</option>
        {/each}
      </select>
    </label>
    <label>
      {t("entry.type")}
      <select bind:value={typeFilter}>
        <option value="">—</option>
        {#each types as type (type.pid)}
          <option value={type.key}>{type.name}</option>
        {/each}
      </select>
    </label>
    <label>
      {t("entry.key")}
      <input bind:value={search} />
    </label>
  </div>

  <table>
    <thead>
      <tr>
        <th>{t("entry.key")}</th>
        <th>{t("entry.type")}</th>
        <th>{t("translations.source")}</th>
        <th>{t("common.status")}</th>
      </tr>
    </thead>
    <tbody>
      {#each shown as entry (entry.pid)}
        <tr>
          <td><a href="/entries/{entry.pid}">{entry.key}</a></td>
          <td>{entry.content_type_key}</td>
          <td>{entry.source_locale}</td>
          <td>
            {#if entry.archived_at}
              <span class="state archived">{t("entry.archived")}</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if shown.length === 0}
    <p class="no-data">{t("common.noData")}</p>
  {/if}
{/if}
