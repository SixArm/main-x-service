<!--
  SearchBox — a one-line search form (text input + submit button). Submits
  via the `onsearch` callback rather than navigating, so the parent owns
  how the query runs.

  $props:
    - value?: string ($bindable) — the query text (default "").
    - placeholder?: string — input placeholder and aria-label.
    - onsearch?: (value) => void — invoked with the current value on submit.
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

    // Prevent the native form navigation; hand the query to the parent.
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
