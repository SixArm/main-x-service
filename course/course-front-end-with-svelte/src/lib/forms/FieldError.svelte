<!--
  FieldError — inline validation message for a single form field.

  Renders an assertive `role="alert"` so screen readers announce the
  error as it appears, and renders nothing when `error` is null/empty.

  $props:
    - error?: string | null — message to show; falsy ⇒ nothing rendered.
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
