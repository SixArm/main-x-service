<!--
  SearchBox — a single-input search form that emits the query on submit.

  Props:
    - value (string, $bindable, default ""): the current query text; two-way
      bindable so a parent can read/seed it.
    - placeholder (string, default "Search…"): input placeholder + aria-label.
    - onsearch ((value) => void, optional): callback fired on submit.
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
        {placeholder}
        aria-label={placeholder}
    />
    <button type="submit" class="button primary">Search</button>
</form>

<style>
    .searchbox { display: flex; gap: 0.5rem; align-items: center; }
    .searchbox input { flex: 1; }
</style>
