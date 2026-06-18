<!--
  SearchBox — a single-input search form.

  Purpose: reusable search field that emits the query on submit (Enter or
  the button) rather than on every keystroke, so callers control when a
  request fires.

  $props:
    - value ($bindable string, default ""): the current query text.
    - placeholder (string, default "Search…"): placeholder + aria-label.
    - onsearch ((value: string) => void, optional): invoked on submit.
-->
<script lang="ts">
    import { t } from "$lib/i18n.svelte.js";

    let {
        value = $bindable(""),
        placeholder = "Search…",
        onsearch,
    }: {
        value?: string;
        placeholder?: string;
        onsearch?: (value: string) => void;
    } = $props();

    // Prevent native form navigation; hand the query to the parent instead.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        onsearch?.(value);
    }
</script>

<form class="searchbox" role="search" onsubmit={handleSubmit}>
    <input
        type="search"
        bind:value
        {placeholder}
        aria-label={placeholder}
    />
    <button type="submit" class="button primary">{t("search.action")}</button>
</form>

<style>
    .searchbox { display: flex; gap: 0.5rem; align-items: center; }
    .searchbox input { flex: 1; }
</style>
