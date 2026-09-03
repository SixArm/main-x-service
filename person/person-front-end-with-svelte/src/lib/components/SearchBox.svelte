<!--
  SearchBox — a labelled search input + submit button in a search form.

  Two-way binds `value` and fires `onsearch` on submit (Enter or button),
  not on every keystroke, so callers control when a query is issued.

  Props:
    - value (bindable): string — current query text.
    - placeholder?: string — input placeholder, also used as aria-label.
    - onsearch?: (value) => void — called with the query on form submit.
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

    // Default the placeholder (also used as aria-label) to the translated
    // "Search…" when the caller doesn't supply one.
    const resolvedPlaceholder = $derived(
        placeholder ?? t("search.placeholder"),
    );

    // Prevent the native full-page form navigation; emit the query instead.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        onsearch?.(value);
    }
</script>

<form class="searchbox" role="search" onsubmit={handleSubmit}>
    <input
        type="search"
        bind:value
        placeholder={resolvedPlaceholder}
        aria-label={resolvedPlaceholder}
    />
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
