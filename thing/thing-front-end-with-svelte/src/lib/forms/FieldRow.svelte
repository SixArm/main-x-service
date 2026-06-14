<!--
  FieldRow — responsive horizontal grouping wrapper for form fields.

  Purpose: lays out its children in an auto-fitting grid (fields wrap to new
  rows on narrow viewports), used to place 2+ LabeledFields side by side.

  $props:
    - children (Snippet): the fields to arrange; rendered via {@render}.
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
