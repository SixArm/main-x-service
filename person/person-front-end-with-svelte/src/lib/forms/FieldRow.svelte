<!--
  FieldRow — responsive layout wrapper that arranges its child fields into
  an auto-fitting grid (wrapping columns of >=180px). Purely presentational.

  Props:
    - children: Snippet — the field(s) to lay out side by side.
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
