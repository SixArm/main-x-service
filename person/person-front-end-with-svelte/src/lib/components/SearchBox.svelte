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
    let {
        value = $bindable(""),
        placeholder = "Search…",
        onsearch,
    }: {
        value?: string;
        placeholder?: string;
        onsearch?: (value: string) => void;
    } = $props();

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
        {placeholder}
        aria-label={placeholder}
    />
    <button type="submit" class="button primary">Search</button>
</form>

<style>
    .searchbox { display: flex; gap: 0.5rem; align-items: center; }
    .searchbox input { flex: 1; }
</style>
