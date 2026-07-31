<!--
  The site selector every view needs. Its own component because a site
  is the scope of almost every CMS question, and repeating the fetch in
  six pages is how six pages end up disagreeing about which one is
  selected.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import type { Site } from "$lib/api/cms";

  let { site = $bindable(), sites = $bindable([]) }: { site: string | null; sites?: Site[] } =
    $props();

  $effect(() => {
    cms.listSites().then((rows) => {
      sites = rows;
      site ??= rows[0]?.pid ?? null;
    });
  });
</script>

<div class="panel">
  <label>
    {t("site.choose")}
    <select bind:value={site}>
      {#each sites as option (option.pid)}
        <option value={option.pid}>{option.name} ({option.key})</option>
      {/each}
    </select>
  </label>
</div>
