<!--
  SearchBox — a single-input search form that emits the query on submit.

  Props:
    - value (string, $bindable, default ""): the current query text; two-way
      bindable so a parent can read/seed it.
    - placeholder (string, default "Search…"): input placeholder + aria-label.
    - onsearch ((value) => void, optional): callback fired on submit.
-->
<script lang="ts">
    import { t } from "$lib/i18n.svelte.js";

    let {
        value = $bindable(""),
        placeholder,
        onsearch,
    }: {
        value?: string;
        placeholder?: string;
        onsearch?: (value: string) => void;
    } = $props();

    // Default the placeholder to the localized "Search…" when not supplied.
    const effectivePlaceholder = $derived(placeholder ?? t("search.placeholder"));

    // Intercept native submit (no page reload) and forward the query upward.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        onsearch?.(value);
    }
</script>

<form class="searchbox" role="search" onsubmit={handleSubmit}>
    <input
        type="search"
        bind:value
        placeholder={effectivePlaceholder}
        aria-label={effectivePlaceholder}
    />
    <button type="submit" class="button primary">{t("search.submit")}</button>
</form>

<style>
    .searchbox { display: flex; gap: 0.5rem; align-items: center; }
    .searchbox input { flex: 1; }
</style>
