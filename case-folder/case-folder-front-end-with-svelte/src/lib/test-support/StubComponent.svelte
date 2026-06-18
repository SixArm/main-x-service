<script lang="ts">
    // Test stub — a no-op component standing in for an external helper
    // (e.g. a Lily `lily-design-system-svelte-*` select) during unit tests, so a layout render
    // does not depend on the sibling design-system repo being resolvable.
    let { ..._rest }: Record<string, unknown> = $props();
</script>
