<!--
  FieldRow — responsive layout wrapper that arranges its child fields in a
  CSS grid (auto-fit columns, min 180px). Purely presentational.

  $props:
    - children: Snippet — the fields (typically LabeledFields) to lay out.
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
