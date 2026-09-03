<!--
  FieldError — inline validation message for a single form field.

  Renders nothing when `error` is null/empty; otherwise shows the message
  with `role="alert"` so assistive tech announces it. Used by
  LabeledField and reusable on its own.

  $props:
    - error?: string | null — message to show, or null/empty to hide.
-->
<script lang="ts">
    let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
    <small class="error" role="alert">{error}</small>
{/if}

<style>
    .error {
        color: var(--mxi-color-danger);
        font-size: 0.75rem;
    }
</style>
