<!--
  Landing route (`/`) — a collection switcher into the four work-item
  collections (portfolios / projects / products / programs).
-->
<script lang="ts">
  import { COLLECTIONS } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
</script>

<svelte:head><title>{t("brand.name")}</title></svelte:head>

<h1>{t("list.title")}</h1>
<ul class="stack">
  {#each COLLECTIONS as collection (collection)}
    <li class="surface row">
      <a href={`/${collection}`}>{collection}</a>
    </li>
  {/each}
</ul>
