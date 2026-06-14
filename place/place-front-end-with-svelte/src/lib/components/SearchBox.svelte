<!--
  SearchBox — a small search `<form>` with a text input and submit button.

  $props:
    - value (string, $bindable) — the current query; two-way bound so the
      parent can read/seed it. Default "".
    - placeholder (string) — input placeholder, also used as the aria-label.
    - onsearch ((value) => void) — callback fired on submit with the query.
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
        {placeholder}
        aria-label={placeholder}
    />
    <button type="submit" class="button primary">Search</button>
</form>

<style>
    .searchbox { display: flex; gap: 0.5rem; align-items: center; }
    .searchbox input { flex: 1; }
</style>
