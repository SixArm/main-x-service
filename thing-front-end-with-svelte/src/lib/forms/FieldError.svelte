<script lang="ts">
    let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
    <small class="error" role="alert">{error}</small>
{/if}

<style>
    .error { color: var(--mxi-color-danger); font-size: 0.75rem; }
</style>
