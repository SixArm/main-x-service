<!--
  FieldRow — responsive layout wrapper that arranges its child fields in a
  self-fitting grid (auto-fit, min 180px columns), so a row of inputs wraps
  gracefully on narrow viewports.

  $props:
    - children (Snippet) — the fields to lay out across the row.
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
