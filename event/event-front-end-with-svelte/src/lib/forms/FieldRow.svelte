<!--
  FieldRow — responsive layout wrapper that lays out its child fields in a
  self-adjusting grid (auto-fit columns, min 180px wide). Purely
  presentational; carries no state.

  Props:
    - children (Snippet): the fields to arrange side by side.
-->
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
