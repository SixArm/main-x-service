<!--
  SearchBox — a small search `<form>` with a text input and submit button.

  $props:
    - value (string, $bindable) — the current query; two-way bound so the
      parent can read/seed it. Default "".
    - placeholder (string) — input placeholder, also used as the aria-label.
    - onsearch ((value) => void) — callback fired on submit with the query.
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

    // Fall back to the generic "Search…" placeholder when the parent omits one.
    const resolvedPlaceholder = $derived(
        placeholder ?? t("search.placeholder"),
    );

    // Submit handler: suppress native navigation, then notify the parent.
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
