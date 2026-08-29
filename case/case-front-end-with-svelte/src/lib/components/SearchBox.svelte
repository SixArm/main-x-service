<!--
  SearchBox — a `role="search"` form with one search input and a submit
  button. Fires `onsearch` only on submit (not per keystroke), so the
  parent controls when a query actually runs.

  $props:
    - value?: string ($bindable) — the current query text.
    - placeholder?: string — input placeholder + aria-label (default t("search.placeholder")).
    - onsearch?: (value) => void — invoked with the query on submit.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";

  let {
    value = $bindable(""),
    placeholder,
    onsearch,
  }: {
    value?: string;
    placeholder?: string;
    onsearch?: (value: string) => void;
  } = $props();

  // Default the placeholder to the localized "Search…" when the parent
  // supplies none; it also doubles as the input's accessible name.
  const ph = $derived(placeholder ?? t("search.placeholder"));

  // Prevent the native GET navigation; hand the query to the parent.
  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    onsearch?.(value);
  }
</script>

<form class="searchbox" role="search" onsubmit={handleSubmit}>
  <input type="search" bind:value placeholder={ph} aria-label={ph} />
  <button type="submit" class="button primary">{t("search.submit")}</button>
</form>

<style>
  .searchbox {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .searchbox input {
    flex: 1;
  }
</style>
