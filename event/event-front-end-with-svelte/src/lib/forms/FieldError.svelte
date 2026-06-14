<!--
  FieldError — inline form-field validation message.

  Purpose: render a single field's error text, with `role="alert"` so
  assistive tech announces it. Renders nothing when there is no error.

  Props:
    - error (string | null, default null): the message to show; falsy = hidden.
-->
<script lang="ts">
    let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
    <small class="error" role="alert">{error}</small>
{/if}

<style>
    .error { color: var(--mxi-color-danger); font-size: 0.75rem; }
</style>
