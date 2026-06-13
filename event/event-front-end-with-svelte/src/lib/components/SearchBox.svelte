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
