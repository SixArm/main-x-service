<script lang="ts">
    import type { Snippet } from "svelte";
    let { children }: { children: Snippet } = $props();
</script>

<div class="field-row">
    {@render children()}
</div>

<style>
    .field-row {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
        gap: 0.75rem;
    }
</style>
